//! Virtio-net / genet smoltcp stack (Phase 51: DHCP, real DNS, dual TCP).

use lerux_interface_types::{NetResponse, MAX_DNS_NAME, MAX_NET_TCP_PAYLOAD, MAX_NET_UDP_PAYLOAD};
use lerux_logging::log;
use sel4_abstract_allocator::{basic::BasicAllocator, WithAlignmentBound};
use sel4_driver_interfaces::net::MacAddress;
use sel4_microkit::{memory_region_symbol, Channel};
use sel4_shared_memory::SharedMemoryRef;
use sel4_shared_ring_buffer::RingBuffers;
use sel4_shared_ring_buffer_smoltcp::DeviceImpl;
use smoltcp::{
    iface::{Config, Interface, SocketHandle, SocketSet, SocketStorage},
    phy::{DeviceCapabilities, Medium},
    socket::{
        dhcpv4::{self, Socket as DhcpSocket},
        dns::{self, GetQueryResultError, Socket as DnsSocket},
        tcp::{Socket as TcpSocket, SocketBuffer as TcpSocketBuffer},
        udp::{PacketBuffer, PacketMetadata, Socket as UdpSocket},
        AnySocket,
    },
    time::Instant,
    wire::{
        DnsQueryType, EthernetAddress, HardwareAddress, IpAddress, IpCidr, IpEndpoint,
        IpListenEndpoint, Ipv4Address,
    },
};

use crate::config;

const NET_DRIVER: Channel = Channel::new(1);

/// Static fallback when DHCP does not complete (QEMU user-net / RPi4 demo).
#[cfg(feature = "board-rpi4b_4gb_workstation")]
const STATIC_GUEST_IP: Ipv4Address = Ipv4Address::new(192, 168, 1, 10);
#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
const STATIC_GUEST_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 15);
#[cfg(feature = "board-rpi4b_4gb_workstation")]
const STATIC_GATEWAY: Ipv4Address = Ipv4Address::new(192, 168, 1, 1);
#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
const STATIC_GATEWAY: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);
#[cfg(feature = "board-rpi4b_4gb_workstation")]
const STATIC_DNS: Ipv4Address = Ipv4Address::new(192, 168, 1, 1);
#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
const STATIC_DNS: Ipv4Address = Ipv4Address::new(10, 0, 2, 3);
const STATIC_PREFIX: u8 = 24;

const LOCAL_UDP_PORT: u16 = 4242;
const REMOTE_UDP_PORT: u16 = 12345;
const TCP_LOCAL_PORT: u16 = 49152;

/// Fake-time step per poll (ms). DHCP/DNS retries need advancing Instant.
const POLL_TICK_MS: i64 = 20;
/// Give DHCP this much fake time before applying static fallback.
const DHCP_GIVE_UP_MS: i64 = 3_000;

type NetRingBuffers = (
    RingBuffers<'static, sel4_shared_ring_buffer::roles::Provide, fn()>,
    RingBuffers<'static, sel4_shared_ring_buffer::roles::Provide, fn()>,
);

/// udp + tcp_client + tcp_listen + dhcp + dns
const SOCK_SLOTS: usize = 5;

struct SocketArena {
    storage: [SocketStorage<'static>; SOCK_SLOTS],
    udp_rx_meta: [PacketMetadata; 1],
    udp_rx_payload: [u8; 128],
    udp_tx_meta: [PacketMetadata; 1],
    udp_tx_payload: [u8; 128],
    tcp_client_rx: [u8; 1024],
    tcp_client_tx: [u8; 1024],
    tcp_listen_rx: [u8; 1024],
    tcp_listen_tx: [u8; 1024],
    dns_queries: [Option<dns::DnsQuery>; 1],
    udp_handle: Option<SocketHandle>,
    tcp_client_handle: Option<SocketHandle>,
    tcp_listen_handle: Option<SocketHandle>,
    dhcp_handle: Option<SocketHandle>,
    dns_handle: Option<SocketHandle>,
    udp_bound: bool,
}

impl SocketArena {
    const fn empty() -> Self {
        Self {
            storage: [SocketStorage::EMPTY; SOCK_SLOTS],
            udp_rx_meta: [PacketMetadata::EMPTY],
            udp_rx_payload: [0; 128],
            udp_tx_meta: [PacketMetadata::EMPTY],
            udp_tx_payload: [0; 128],
            tcp_client_rx: [0; 1024],
            tcp_client_tx: [0; 1024],
            tcp_listen_rx: [0; 1024],
            tcp_listen_tx: [0; 1024],
            dns_queries: [None],
            udp_handle: None,
            tcp_client_handle: None,
            tcp_listen_handle: None,
            dhcp_handle: None,
            dns_handle: None,
            udp_bound: false,
        }
    }
}

static mut SOCKET_ARENA: SocketArena = SocketArena::empty();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    None,
    UdpTx,
    UdpRecv,
    DnsResolve,
    TcpConnect,
    TcpListen,
    TcpSend,
    TcpRecv,
    TcpClose,
}

/// Which TCP socket an in-flight send/recv/close targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TcpRole {
    Client,
    Listen,
}

#[derive(Clone, Copy)]
struct IfaceState {
    addr: Ipv4Address,
    prefix: u8,
    gateway: Ipv4Address,
    dns: Ipv4Address,
    dhcp: bool,
    configured: bool,
}

impl IfaceState {
    const fn static_fallback() -> Self {
        Self {
            addr: STATIC_GUEST_IP,
            prefix: STATIC_PREFIX,
            gateway: STATIC_GATEWAY,
            dns: STATIC_DNS,
            dhcp: false,
            configured: false,
        }
    }
}

pub struct NetStack {
    device: DeviceImpl<WithAlignmentBound<BasicAllocator>>,
    iface: Interface,
    iface_state: IfaceState,
    op: Op,
    tcp_role: TcpRole,
    pending_udp_len: Option<u8>,
    pending_udp: [u8; MAX_NET_UDP_PAYLOAD],
    pending_dns_name: [u8; MAX_DNS_NAME],
    pending_dns_len: u8,
    dns_query: Option<dns::QueryHandle>,
    pending_tcp_connect: Option<([u8; 4], u16)>,
    pending_tcp_listen: Option<u16>,
    listen_port: Option<u16>,
    pending_tcp_send_len: Option<u16>,
    pending_tcp_send: [u8; MAX_NET_TCP_PAYLOAD],
    tcp_client_active: bool,
    tcp_listening: bool,
    /// True when an inbound listen socket may have recv data (for client notify).
    pub listen_activity: bool,
    completed: Option<NetResponse>,
    udp_tx_logged: bool,
    last_was_udp_tx: bool,
    /// Monotonic fake clock for smoltcp timers (DHCP/DNS).
    millis: i64,
    dhcp_done: bool,
    dhcp_fallback_applied: bool,
    /// When false, try_tcp_listen succeeds silently (background re-listen).
    listen_notify_client: bool,
}

fn create_dma_region() -> SharedMemoryRef<'static, [u8]> {
    #[cfg(feature = "unified-dma")]
    {
        use core::ptr::{self, NonNull};
        use sel4_microkit::var;
        let base = *var!(virtio_net_client_dma_vaddr: usize = 0);
        let ptr = NonNull::new(ptr::slice_from_raw_parts_mut(
            (base + config::VIRTIO_NET_HAL_SIZE) as *mut u8,
            config::VIRTIO_NET_BOUNCE_SIZE,
        ))
        .expect("net bounce region");
        unsafe { SharedMemoryRef::new(ptr) }
    }
    #[cfg(not(feature = "unified-dma"))]
    unsafe {
        SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
            virtio_net_client_dma_vaddr: *mut [u8],
            n = config::VIRTIO_NET_CLIENT_DMA_SIZE
        ))
    }
}

fn create_net_ring_buffers(notify_net: fn()) -> NetRingBuffers {
    let rx_ring_buffers = RingBuffers::from_ptrs_using_default_initialization_strategy_for_role(
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_rx_free: *mut _)) },
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_rx_used: *mut _)) },
        notify_net,
    );
    let tx_ring_buffers = RingBuffers::from_ptrs_using_default_initialization_strategy_for_role(
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_tx_free: *mut _)) },
        unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_tx_used: *mut _)) },
        notify_net,
    );
    (rx_ring_buffers, tx_ring_buffers)
}

fn create_net_device(
    dma_region: SharedMemoryRef<'static, [u8]>,
    rx_ring_buffers: RingBuffers<'static, sel4_shared_ring_buffer::roles::Provide, fn()>,
    tx_ring_buffers: RingBuffers<'static, sel4_shared_ring_buffer::roles::Provide, fn()>,
) -> DeviceImpl<WithAlignmentBound<BasicAllocator>> {
    let bounce_buffer_allocator =
        WithAlignmentBound::new(BasicAllocator::new(dma_region.as_ptr().len()), 1);
    let mut caps = DeviceCapabilities::default();
    caps.max_transmission_unit = 1500;
    caps.medium = Medium::Ethernet;
    DeviceImpl::new(
        Default::default(),
        dma_region,
        bounce_buffer_allocator,
        rx_ring_buffers,
        tx_ring_buffers,
        config::NET_QUEUE_SIZE,
        config::NET_BUFFER_LEN,
        caps,
    )
    .expect("virtio-net device")
}

fn configure_iface(
    device: &mut DeviceImpl<WithAlignmentBound<BasicAllocator>>,
    mac: MacAddress,
) -> Interface {
    // Start without a unicast address; DHCP or static fallback fills it in.
    let hardware_addr = HardwareAddress::Ethernet(EthernetAddress(mac.0));
    Interface::new(Config::new(hardware_addr), device, Instant::from_millis(0))
}

fn resolve_static_dns(name: &[u8], gateway: Ipv4Address, dns: Ipv4Address) -> Option<[u8; 4]> {
    if name == b"host" {
        return Some(gateway.octets());
    }
    if name == b"dns" {
        return Some(dns.octets());
    }
    None
}

fn apply_ipv4(iface: &mut Interface, addr: Ipv4Address, prefix: u8, gateway: Ipv4Address) {
    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs.clear();
        let _ = ip_addrs.push(IpCidr::new(IpAddress::Ipv4(addr), prefix));
    });
    iface.routes_mut().remove_default_ipv4_route();
    let _ = iface.routes_mut().add_default_ipv4_route(gateway);
}

impl NetStack {
    pub fn new(mac: MacAddress) -> Self {
        let notify_net: fn() = || NET_DRIVER.notify();
        let dma_region = create_dma_region();
        let (rx_ring_buffers, tx_ring_buffers) = create_net_ring_buffers(notify_net);
        let mut device = create_net_device(dma_region, rx_ring_buffers, tx_ring_buffers);
        let iface = configure_iface(&mut device, mac);
        Self {
            device,
            iface,
            iface_state: IfaceState::static_fallback(),
            op: Op::None,
            tcp_role: TcpRole::Client,
            pending_udp_len: None,
            pending_udp: [0; MAX_NET_UDP_PAYLOAD],
            pending_dns_name: [0; MAX_DNS_NAME],
            pending_dns_len: 0,
            dns_query: None,
            pending_tcp_connect: None,
            pending_tcp_listen: None,
            listen_port: None,
            pending_tcp_send_len: None,
            pending_tcp_send: [0; MAX_NET_TCP_PAYLOAD],
            tcp_client_active: false,
            tcp_listening: false,
            listen_activity: false,
            completed: None,
            udp_tx_logged: false,
            last_was_udp_tx: false,
            millis: 0,
            dhcp_done: false,
            dhcp_fallback_applied: false,
            listen_notify_client: true,
        }
    }

    fn now(&self) -> Instant {
        Instant::from_millis(self.millis)
    }

    pub fn queue_udp_tx(&mut self, payload_len: u8, payload: [u8; MAX_NET_UDP_PAYLOAD]) {
        self.pending_udp = payload;
        self.pending_udp_len = Some(payload_len);
        self.op = Op::UdpTx;
        self.completed = None;
        self.last_was_udp_tx = false;
    }

    pub fn queue_udp_recv(&mut self) {
        self.op = Op::UdpRecv;
        self.completed = None;
    }

    pub fn queue_dns_resolve(&mut self, name_len: u8, name: [u8; MAX_DNS_NAME]) {
        let len = (name_len as usize).min(MAX_DNS_NAME);
        // Static aliases always win (deterministic smokes: host/dns).
        if let Some(addr) =
            resolve_static_dns(&name[..len], self.iface_state.gateway, self.iface_state.dns)
        {
            self.completed = Some(NetResponse::Ipv4 { addr });
            self.op = Op::None;
            self.dns_query = None;
            return;
        }
        self.pending_dns_name = name;
        self.pending_dns_len = len as u8;
        self.dns_query = None;
        self.op = Op::DnsResolve;
        self.completed = None;
    }

    pub fn queue_tcp_connect(&mut self, addr: [u8; 4], port: u16) {
        self.tcp_client_active = false;
        self.pending_tcp_send_len = None;
        self.pending_tcp_connect = Some((addr, port));
        self.op = Op::TcpConnect;
        self.tcp_role = TcpRole::Client;
        self.completed = None;
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        arena.tcp_client_handle = None;
    }

    pub fn queue_tcp_listen(&mut self, port: u16) {
        self.tcp_listening = false;
        self.pending_tcp_send_len = None;
        self.pending_tcp_listen = Some(port);
        self.listen_port = Some(port);
        self.op = Op::TcpListen;
        self.tcp_role = TcpRole::Listen;
        self.listen_notify_client = true;
        self.completed = None;
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        arena.tcp_listen_handle = None;
    }

    pub fn queue_tcp_send(&mut self, payload_len: u16, payload: [u8; MAX_NET_TCP_PAYLOAD]) {
        self.pending_tcp_send = payload;
        self.pending_tcp_send_len = Some(payload_len);
        // Prefer listen when active (http-fs); else client (fetch).
        self.tcp_role = if self.tcp_listening && !self.tcp_client_active {
            TcpRole::Listen
        } else if self.tcp_client_active {
            TcpRole::Client
        } else if self.tcp_listening {
            TcpRole::Listen
        } else {
            TcpRole::Client
        };
        self.op = Op::TcpSend;
        self.completed = None;
    }

    pub fn queue_tcp_recv(&mut self) {
        self.tcp_role = if self.tcp_listening && !self.tcp_client_active {
            TcpRole::Listen
        } else if self.tcp_client_active {
            TcpRole::Client
        } else if self.tcp_listening {
            TcpRole::Listen
        } else {
            TcpRole::Client
        };
        self.op = Op::TcpRecv;
        self.completed = None;
        self.listen_activity = false;
    }

    pub fn queue_tcp_close(&mut self) {
        self.tcp_role = if self.tcp_listening && !self.tcp_client_active {
            TcpRole::Listen
        } else if self.tcp_client_active {
            TcpRole::Client
        } else {
            TcpRole::Listen
        };
        self.op = Op::TcpClose;
        self.completed = None;
    }

    /// Non-blocking interface snapshot.
    pub fn iface_response(&self) -> NetResponse {
        if !self.iface_state.configured {
            return NetResponse::Error;
        }
        NetResponse::Iface {
            addr: self.iface_state.addr.octets(),
            prefix: self.iface_state.prefix,
            gateway: self.iface_state.gateway.octets(),
            dns: self.iface_state.dns.octets(),
            dhcp: self.iface_state.dhcp,
        }
    }

    pub fn take_completed(&mut self) -> Option<NetResponse> {
        self.completed.take()
    }

    pub fn is_busy(&self) -> bool {
        self.op != Op::None
    }

    /// Drop any in-flight async op (Abort). Prevents a later driver notify from
    /// stashing a completion with no owning client.
    pub fn cancel_async(&mut self) {
        if self.op == Op::DnsResolve {
            self.dns_query = None;
        }
        self.op = Op::None;
        self.completed = None;
    }

    fn ensure_core_sockets(&mut self, sockets: &mut SocketSet<'static>) {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        if arena.udp_handle.is_none() {
            let udp = UdpSocket::new(
                PacketBuffer::new(&mut arena.udp_rx_meta[..], &mut arena.udp_rx_payload[..]),
                PacketBuffer::new(&mut arena.udp_tx_meta[..], &mut arena.udp_tx_payload[..]),
            );
            arena.udp_handle = Some(sockets.add(udp));
        }
        if arena.dhcp_handle.is_none() {
            arena.dhcp_handle = Some(sockets.add(DhcpSocket::new()));
        }
        if arena.dns_handle.is_none() {
            let servers = [IpAddress::Ipv4(self.iface_state.dns)];
            let dns = DnsSocket::new(&servers, &mut arena.dns_queries[..]);
            arena.dns_handle = Some(sockets.add(dns));
        }
    }

    fn ensure_tcp_client(&mut self, sockets: &mut SocketSet<'static>) -> Option<SocketHandle> {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        if let Some(h) = arena.tcp_client_handle {
            return Some(h);
        }
        let tcp = TcpSocket::new(
            TcpSocketBuffer::new(&mut arena.tcp_client_rx[..]),
            TcpSocketBuffer::new(&mut arena.tcp_client_tx[..]),
        );
        let h = sockets.add(tcp);
        arena.tcp_client_handle = Some(h);
        Some(h)
    }

    fn ensure_tcp_listen(&mut self, sockets: &mut SocketSet<'static>) -> Option<SocketHandle> {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        if let Some(h) = arena.tcp_listen_handle {
            return Some(h);
        }
        let tcp = TcpSocket::new(
            TcpSocketBuffer::new(&mut arena.tcp_listen_rx[..]),
            TcpSocketBuffer::new(&mut arena.tcp_listen_tx[..]),
        );
        let h = sockets.add(tcp);
        arena.tcp_listen_handle = Some(h);
        Some(h)
    }

    fn guest_ip(&self) -> Ipv4Address {
        self.iface_state.addr
    }

    fn try_udp_tx(&mut self, sockets: &mut SocketSet<'static>) {
        if self.op != Op::UdpTx || !self.iface_state.configured {
            return;
        }
        let Some(payload_len) = self.pending_udp_len else {
            return;
        };
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let Some(udp_handle) = arena.udp_handle else {
            return;
        };
        let local = IpListenEndpoint::from((IpAddress::Ipv4(self.guest_ip()), LOCAL_UDP_PORT));
        let remote = IpEndpoint::new(IpAddress::Ipv4(self.iface_state.gateway), REMOTE_UDP_PORT);
        let udp = sockets.get_mut::<UdpSocket>(udp_handle);
        let len = (payload_len as usize).min(MAX_NET_UDP_PAYLOAD);
        let payload = &self.pending_udp[..len];
        if !arena.udp_bound {
            if udp.bind(local).is_err() {
                return;
            }
            arena.udp_bound = true;
        }
        if udp.send_slice(payload, remote).is_ok() {
            self.pending_udp_len = None;
            self.completed = Some(NetResponse::Ok);
            self.op = Op::None;
            self.last_was_udp_tx = true;
        }
    }

    fn try_udp_recv(&mut self, sockets: &mut SocketSet<'static>) {
        if self.op != Op::UdpRecv || !self.iface_state.configured {
            return;
        }
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let Some(udp_handle) = arena.udp_handle else {
            return;
        };
        let local = IpListenEndpoint::from((IpAddress::Ipv4(self.guest_ip()), LOCAL_UDP_PORT));
        let udp = sockets.get_mut::<UdpSocket>(udp_handle);
        if !arena.udp_bound {
            if udp.bind(local).is_err() {
                return;
            }
            arena.udp_bound = true;
        }
        if !udp.can_recv() {
            return;
        }
        let mut buf = [0u8; MAX_NET_UDP_PAYLOAD];
        match udp.recv_slice(&mut buf) {
            Ok((len, _meta)) => {
                let mut data = [0u8; MAX_NET_UDP_PAYLOAD];
                let n = len.min(MAX_NET_UDP_PAYLOAD);
                data[..n].copy_from_slice(&buf[..n]);
                self.completed = Some(NetResponse::UdpData {
                    data_len: n as u8,
                    data,
                });
                self.op = Op::None;
            }
            Err(_) => {
                self.completed = Some(NetResponse::Error);
                self.op = Op::None;
            }
        }
    }

    fn try_dns_resolve(&mut self, sockets: &mut SocketSet<'static>) {
        if self.op != Op::DnsResolve || !self.iface_state.configured {
            return;
        }
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let Some(dns_handle) = arena.dns_handle else {
            return;
        };

        // Update DNS server to current iface DNS (may change after DHCP).
        {
            let dns = sockets.get_mut::<DnsSocket>(dns_handle);
            dns.update_servers(&[IpAddress::Ipv4(self.iface_state.dns)]);
        }

        if self.dns_query.is_none() {
            let len = self.pending_dns_len.min(MAX_DNS_NAME as u8) as usize;
            let Ok(name) = core::str::from_utf8(&self.pending_dns_name[..len]) else {
                self.completed = Some(NetResponse::Error);
                self.op = Op::None;
                return;
            };
            let dns = sockets.get_mut::<DnsSocket>(dns_handle);
            match dns.start_query(self.iface.context(), name, DnsQueryType::A) {
                Ok(h) => self.dns_query = Some(h),
                Err(_) => {
                    self.completed = Some(NetResponse::Error);
                    self.op = Op::None;
                }
            }
            return;
        }

        let qh = self.dns_query.expect("dns query");
        let dns = sockets.get_mut::<DnsSocket>(dns_handle);
        match dns.get_query_result(qh) {
            Ok(addrs) => {
                self.dns_query = None;
                self.completed = match addrs.iter().next() {
                    Some(IpAddress::Ipv4(a)) => Some(NetResponse::Ipv4 { addr: a.octets() }),
                    _ => Some(NetResponse::Error),
                };
                self.op = Op::None;
            }
            Err(GetQueryResultError::Pending) => {}
            Err(GetQueryResultError::Failed) => {
                self.dns_query = None;
                // Soft-fail: static map already tried; report error.
                self.completed = Some(NetResponse::Error);
                self.op = Op::None;
            }
        }
    }

    fn try_tcp_connect(&mut self, sockets: &mut SocketSet<'static>) {
        if self.op != Op::TcpConnect || !self.iface_state.configured {
            return;
        }
        let Some((addr, port)) = self.pending_tcp_connect else {
            return;
        };
        let Some(tcp_handle) = self.ensure_tcp_client(sockets) else {
            return;
        };
        let remote = IpEndpoint::new(
            IpAddress::Ipv4(Ipv4Address::new(addr[0], addr[1], addr[2], addr[3])),
            port,
        );
        let local = IpListenEndpoint::from((IpAddress::Ipv4(self.guest_ip()), TCP_LOCAL_PORT));
        let tcp = sockets.get_mut::<TcpSocket>(tcp_handle);
        if tcp.state() == smoltcp::socket::tcp::State::Closed {
            let _ = tcp.connect(self.iface.context(), remote, local);
        }
        if tcp.is_active() {
            self.tcp_client_active = true;
            self.pending_tcp_connect = None;
            self.completed = Some(NetResponse::Ok);
            self.op = Op::None;
        }
    }

    fn try_tcp_listen(&mut self, sockets: &mut SocketSet<'static>) {
        if self.op != Op::TcpListen {
            return;
        }
        let Some(port) = self.pending_tcp_listen else {
            return;
        };
        let Some(tcp_handle) = self.ensure_tcp_listen(sockets) else {
            return;
        };
        let endpoint = IpListenEndpoint { addr: None, port };
        let tcp = sockets.get_mut::<TcpSocket>(tcp_handle);
        if !tcp.is_open() && tcp.listen(endpoint).is_err() {
            self.op = Op::None;
            if self.listen_notify_client {
                self.completed = Some(NetResponse::Error);
            }
            self.listen_notify_client = true;
            return;
        }
        self.tcp_listening = true;
        self.pending_tcp_listen = None;
        self.op = Op::None;
        if self.listen_notify_client {
            self.completed = Some(NetResponse::Ok);
            log::info!("lerux-net: listen :{}", port);
        }
        self.listen_notify_client = true;
    }

    fn tcp_handle_for_role(&self, role: TcpRole) -> Option<SocketHandle> {
        let arena = unsafe { &*core::ptr::addr_of!(SOCKET_ARENA) };
        match role {
            TcpRole::Client => arena.tcp_client_handle,
            TcpRole::Listen => arena.tcp_listen_handle,
        }
    }

    fn try_tcp_send(&mut self, sockets: &mut SocketSet<'static>) {
        let active = match self.tcp_role {
            TcpRole::Client => self.tcp_client_active,
            TcpRole::Listen => self.tcp_listening,
        };
        if self.op != Op::TcpSend || !active {
            return;
        }
        let Some(payload_len) = self.pending_tcp_send_len else {
            return;
        };
        let Some(tcp_handle) = self.tcp_handle_for_role(self.tcp_role) else {
            return;
        };
        let tcp = sockets.get_mut::<TcpSocket>(tcp_handle);
        if !tcp.may_send() {
            return;
        }
        let len = (payload_len as usize).min(MAX_NET_TCP_PAYLOAD);
        let payload = &self.pending_tcp_send[..len];
        if tcp.send_slice(payload).is_ok() {
            self.pending_tcp_send_len = None;
            self.completed = Some(NetResponse::Ok);
            self.op = Op::None;
        }
    }

    fn try_tcp_recv(&mut self, sockets: &mut SocketSet<'static>) {
        let active = match self.tcp_role {
            TcpRole::Client => self.tcp_client_active,
            TcpRole::Listen => self.tcp_listening,
        };
        if self.op != Op::TcpRecv || !active {
            return;
        }
        let Some(tcp_handle) = self.tcp_handle_for_role(self.tcp_role) else {
            return;
        };
        let tcp = sockets.get_mut::<TcpSocket>(tcp_handle);
        if !tcp.may_recv() {
            return;
        }
        let mut buf = [0u8; MAX_NET_TCP_PAYLOAD];
        match tcp.recv_slice(&mut buf) {
            Ok(0) => {
                self.completed = Some(NetResponse::Ok);
                self.op = Op::None;
            }
            Ok(len) => {
                let mut data = [0u8; MAX_NET_TCP_PAYLOAD];
                data[..len].copy_from_slice(&buf[..len]);
                self.completed = Some(NetResponse::TcpData {
                    data_len: len as u16,
                    data,
                });
                self.op = Op::None;
            }
            Err(_) => {
                self.completed = Some(NetResponse::Error);
                self.op = Op::None;
            }
        }
    }

    fn try_tcp_close(&mut self, sockets: &mut SocketSet<'static>) {
        if self.op != Op::TcpClose {
            return;
        }
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        match self.tcp_role {
            TcpRole::Client => {
                if let Some(h) = arena.tcp_client_handle.take() {
                    let mut sock = sockets.remove(h);
                    if let Some(tcp) = TcpSocket::downcast_mut(&mut sock) {
                        tcp.close();
                        tcp.abort();
                    }
                }
                self.tcp_client_active = false;
            }
            TcpRole::Listen => {
                if let Some(h) = arena.tcp_listen_handle.take() {
                    let mut sock = sockets.remove(h);
                    if let Some(tcp) = TcpSocket::downcast_mut(&mut sock) {
                        tcp.close();
                        tcp.abort();
                    }
                }
                self.tcp_listening = false;
            }
        }
        self.completed = Some(NetResponse::Ok);
        self.op = Op::None;
    }

    fn note_listen_activity(&mut self, sockets: &mut SocketSet<'static>) {
        if !self.tcp_listening || self.listen_port.is_none() {
            return;
        }
        let Some(tcp_handle) = self.tcp_handle_for_role(TcpRole::Listen) else {
            return;
        };
        let tcp = sockets.get_mut::<TcpSocket>(tcp_handle);
        if tcp.may_recv() {
            self.listen_activity = true;
        }
    }

    fn process_dhcp(&mut self, sockets: &mut SocketSet<'static>) {
        if self.dhcp_done {
            return;
        }
        let arena = unsafe { &*core::ptr::addr_of!(SOCKET_ARENA) };
        let Some(dhcp_handle) = arena.dhcp_handle else {
            return;
        };
        let event = {
            let dhcp = sockets.get_mut::<DhcpSocket>(dhcp_handle);
            dhcp.poll()
        };
        match event {
            Some(dhcpv4::Event::Configured(cfg)) => {
                let addr = cfg.address.address();
                let prefix = cfg.address.prefix_len();
                let gateway = cfg.router.unwrap_or(STATIC_GATEWAY);
                let dns = cfg.dns_servers.iter().next().copied().unwrap_or(STATIC_DNS);
                apply_ipv4(&mut self.iface, addr, prefix, gateway);
                self.iface_state = IfaceState {
                    addr,
                    prefix,
                    gateway,
                    dns,
                    dhcp: true,
                    configured: true,
                };
                self.dhcp_done = true;
                // Refresh DNS socket servers.
                if let Some(dns_h) = arena.dns_handle {
                    let dns_sock = sockets.get_mut::<DnsSocket>(dns_h);
                    dns_sock.update_servers(&[IpAddress::Ipv4(dns)]);
                }
                log::info!(
                    "lerux-net: dhcp ok {}.{}.{}.{}/{}",
                    addr.octets()[0],
                    addr.octets()[1],
                    addr.octets()[2],
                    addr.octets()[3],
                    prefix
                );
            }
            Some(dhcpv4::Event::Deconfigured) => {
                self.iface_state.configured = false;
                self.iface.update_ip_addrs(|a| a.clear());
            }
            None => {}
        }

        if !self.dhcp_done && !self.dhcp_fallback_applied && self.millis >= DHCP_GIVE_UP_MS {
            self.apply_static_fallback(sockets);
        }
    }

    fn apply_static_fallback(&mut self, sockets: &mut SocketSet<'static>) {
        apply_ipv4(
            &mut self.iface,
            STATIC_GUEST_IP,
            STATIC_PREFIX,
            STATIC_GATEWAY,
        );
        self.iface_state = IfaceState {
            addr: STATIC_GUEST_IP,
            prefix: STATIC_PREFIX,
            gateway: STATIC_GATEWAY,
            dns: STATIC_DNS,
            dhcp: false,
            configured: true,
        };
        self.dhcp_fallback_applied = true;
        self.dhcp_done = true;
        let arena = unsafe { &*core::ptr::addr_of!(SOCKET_ARENA) };
        if let Some(dns_h) = arena.dns_handle {
            let dns_sock = sockets.get_mut::<DnsSocket>(dns_h);
            dns_sock.update_servers(&[IpAddress::Ipv4(STATIC_DNS)]);
        }
        log::info!(
            "lerux-net: static {}.{}.{}.{}",
            STATIC_GUEST_IP.octets()[0],
            STATIC_GUEST_IP.octets()[1],
            STATIC_GUEST_IP.octets()[2],
            STATIC_GUEST_IP.octets()[3]
        );
    }

    fn log_udp_tx_done(&mut self) {
        if self.last_was_udp_tx && !self.udp_tx_logged {
            log::info!("lerux-net: TX ok");
            self.udp_tx_logged = true;
            self.last_was_udp_tx = false;
        }
    }

    pub fn poll(&mut self) {
        self.millis = self.millis.saturating_add(POLL_TICK_MS);
        let now = self.now();
        self.device.poll();
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let mut sockets = SocketSet::new(&mut arena.storage[..]);
        self.ensure_core_sockets(&mut sockets);
        self.iface.poll(now, &mut self.device, &mut sockets);
        self.process_dhcp(&mut sockets);
        self.try_udp_tx(&mut sockets);
        self.try_udp_recv(&mut sockets);
        self.try_dns_resolve(&mut sockets);
        self.try_tcp_connect(&mut sockets);
        self.try_tcp_listen(&mut sockets);
        self.try_tcp_send(&mut sockets);
        self.try_tcp_recv(&mut sockets);
        self.try_tcp_close(&mut sockets);
        // Second iface poll so TX/DHCP replies make progress in the same tick.
        self.iface.poll(now, &mut self.device, &mut sockets);
        self.process_dhcp(&mut sockets);

        // Re-open listen after close if a listen port is configured.
        if !self.tcp_listening
            && self.op == Op::None
            && let Some(port) = self.listen_port
        {
            let arena = unsafe { &*core::ptr::addr_of!(SOCKET_ARENA) };
            if arena.tcp_listen_handle.is_none() {
                self.pending_tcp_listen = Some(port);
                self.op = Op::TcpListen;
                self.listen_notify_client = false;
                self.try_tcp_listen(&mut sockets);
            }
        }
        self.note_listen_activity(&mut sockets);
        self.log_udp_tx_done();
    }
}
