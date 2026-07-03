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
        tcp::{Socket as TcpSocket, SocketBuffer as TcpSocketBuffer},
        udp::{PacketBuffer, PacketMetadata, Socket as UdpSocket},
    },
    time::Instant,
    wire::{
        EthernetAddress, HardwareAddress, IpAddress, IpCidr, IpEndpoint, IpListenEndpoint,
        Ipv4Address,
    },
};

use crate::config;

const NET_DRIVER: Channel = Channel::new(1);
#[cfg(feature = "board-rpi4b_4gb_workstation")]
const GUEST_IP: Ipv4Address = Ipv4Address::new(192, 168, 1, 10);
#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
const GUEST_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 15);
#[cfg(feature = "board-rpi4b_4gb_workstation")]
const HOST_IP: Ipv4Address = Ipv4Address::new(192, 168, 1, 1);
#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
const HOST_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);
#[cfg(feature = "board-rpi4b_4gb_workstation")]
const DNS_IP: Ipv4Address = Ipv4Address::new(192, 168, 1, 1);
#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
const DNS_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 3);
const LOCAL_UDP_PORT: u16 = 4242;
const REMOTE_UDP_PORT: u16 = 12345;
const TCP_LOCAL_PORT: u16 = 49152;

type NetRingBuffers = (
    RingBuffers<'static, sel4_shared_ring_buffer::roles::Provide, fn()>,
    RingBuffers<'static, sel4_shared_ring_buffer::roles::Provide, fn()>,
);

struct SocketArena {
    storage: [SocketStorage<'static>; 2],
    udp_rx_meta: [PacketMetadata; 1],
    udp_rx_payload: [u8; 128],
    udp_tx_meta: [PacketMetadata; 1],
    udp_tx_payload: [u8; 128],
    tcp_rx: [u8; 1024],
    tcp_tx: [u8; 1024],
    udp_handle: Option<SocketHandle>,
    tcp_handle: Option<SocketHandle>,
}

impl SocketArena {
    const fn empty() -> Self {
        Self {
            storage: [SocketStorage::EMPTY; 2],
            udp_rx_meta: [PacketMetadata::EMPTY],
            udp_rx_payload: [0; 128],
            udp_tx_meta: [PacketMetadata::EMPTY],
            udp_tx_payload: [0; 128],
            tcp_rx: [0; 1024],
            tcp_tx: [0; 1024],
            udp_handle: None,
            tcp_handle: None,
        }
    }
}

static mut SOCKET_ARENA: SocketArena = SocketArena::empty();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    None,
    UdpTx,
    TcpConnect,
    TcpSend,
    TcpRecv,
}

pub struct NetStack {
    device: DeviceImpl<WithAlignmentBound<BasicAllocator>>,
    iface: Interface,
    op: Op,
    pending_udp_len: Option<u8>,
    pending_udp: [u8; MAX_NET_UDP_PAYLOAD],
    pending_tcp_connect: Option<([u8; 4], u16)>,
    pending_tcp_send_len: Option<u16>,
    pending_tcp_send: [u8; MAX_NET_TCP_PAYLOAD],
    tcp_connected: bool,
    completed: Option<NetResponse>,
    udp_tx_logged: bool,
    last_was_udp_tx: bool,
}

fn create_dma_region() -> SharedMemoryRef<'static, [u8]> {
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
    let hardware_addr = HardwareAddress::Ethernet(EthernetAddress(mac.0));
    let mut iface = Interface::new(Config::new(hardware_addr), device, Instant::ZERO);
    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs
            .push(IpCidr::new(IpAddress::Ipv4(GUEST_IP), 24))
            .expect("guest IPv4 address");
    });
    iface
        .routes_mut()
        .add_default_ipv4_route(HOST_IP)
        .expect("default route");
    iface
}

fn resolve_static_dns(name: &[u8]) -> Option<[u8; 4]> {
    if name == b"host" {
        return Some(HOST_IP.octets());
    }
    if name == b"dns" {
        return Some(DNS_IP.octets());
    }
    None
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
            op: Op::None,
            pending_udp_len: None,
            pending_udp: [0; MAX_NET_UDP_PAYLOAD],
            pending_tcp_connect: None,
            pending_tcp_send_len: None,
            pending_tcp_send: [0; MAX_NET_TCP_PAYLOAD],
            tcp_connected: false,
            completed: None,
            udp_tx_logged: false,
            last_was_udp_tx: false,
        }
    }

    pub fn queue_udp_tx(&mut self, payload_len: u8, payload: [u8; MAX_NET_UDP_PAYLOAD]) {
        self.pending_udp = payload;
        self.pending_udp_len = Some(payload_len);
        self.op = Op::UdpTx;
        self.completed = None;
        self.last_was_udp_tx = false;
    }

    pub fn queue_dns_resolve(&mut self, name_len: u8, name: [u8; MAX_DNS_NAME]) {
        let len = name_len as usize;
        self.completed = resolve_static_dns(&name[..len])
            .map(|addr| NetResponse::Ipv4 { addr })
            .or(Some(NetResponse::Error));
        self.op = Op::None;
    }

    pub fn queue_tcp_connect(&mut self, addr: [u8; 4], port: u16) {
        self.tcp_connected = false;
        self.pending_tcp_send_len = None;
        self.pending_tcp_connect = Some((addr, port));
        self.op = Op::TcpConnect;
        self.completed = None;
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        arena.tcp_handle = None;
    }

    pub fn queue_tcp_send(&mut self, payload_len: u16, payload: [u8; MAX_NET_TCP_PAYLOAD]) {
        self.pending_tcp_send = payload;
        self.pending_tcp_send_len = Some(payload_len);
        self.op = Op::TcpSend;
        self.completed = None;
    }

    pub fn queue_tcp_recv(&mut self) {
        self.op = Op::TcpRecv;
        self.completed = None;
    }

    pub fn take_completed(&mut self) -> Option<NetResponse> {
        self.completed.take()
    }

    fn init_udp_socket(sockets: &mut SocketSet<'static>) -> SocketHandle {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let udp = UdpSocket::new(
            PacketBuffer::new(&mut arena.udp_rx_meta[..], &mut arena.udp_rx_payload[..]),
            PacketBuffer::new(&mut arena.udp_tx_meta[..], &mut arena.udp_tx_payload[..]),
        );
        sockets.add(udp)
    }

    fn ensure_udp_socket(&mut self, sockets: &mut SocketSet<'static>) {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        if arena.udp_handle.is_none() {
            arena.udp_handle = Some(Self::init_udp_socket(sockets));
        }
    }

    fn ensure_tcp_socket(&mut self, sockets: &mut SocketSet<'static>) -> Option<SocketHandle> {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        if let Some(handle) = arena.tcp_handle {
            return Some(handle);
        }
        let tcp = TcpSocket::new(
            TcpSocketBuffer::new(&mut arena.tcp_rx[..]),
            TcpSocketBuffer::new(&mut arena.tcp_tx[..]),
        );
        let handle = sockets.add(tcp);
        arena.tcp_handle = Some(handle);
        Some(handle)
    }

    fn try_udp_tx(&mut self, sockets: &mut SocketSet<'static>) {
        if self.op != Op::UdpTx {
            return;
        }
        let Some(payload_len) = self.pending_udp_len else {
            return;
        };
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let Some(udp_handle) = arena.udp_handle else {
            return;
        };
        let local = IpListenEndpoint::from((IpAddress::Ipv4(GUEST_IP), LOCAL_UDP_PORT));
        let remote = IpEndpoint::new(IpAddress::Ipv4(HOST_IP), REMOTE_UDP_PORT);
        let udp = sockets.get_mut::<UdpSocket>(udp_handle);
        let payload = &self.pending_udp[..payload_len as usize];
        if udp.bind(local).is_ok() && udp.send_slice(payload, remote).is_ok() {
            self.pending_udp_len = None;
            self.completed = Some(NetResponse::Ok);
            self.op = Op::None;
            self.last_was_udp_tx = true;
        }
    }

    fn try_tcp_connect(&mut self, sockets: &mut SocketSet<'static>) {
        if self.op != Op::TcpConnect {
            return;
        }
        let Some((addr, port)) = self.pending_tcp_connect else {
            return;
        };
        let Some(tcp_handle) = self.ensure_tcp_socket(sockets) else {
            return;
        };
        let remote = IpEndpoint::new(
            IpAddress::Ipv4(Ipv4Address::new(addr[0], addr[1], addr[2], addr[3])),
            port,
        );
        let local = IpListenEndpoint::from((IpAddress::Ipv4(GUEST_IP), TCP_LOCAL_PORT));
        let tcp = sockets.get_mut::<TcpSocket>(tcp_handle);
        if tcp.state() == smoltcp::socket::tcp::State::Closed {
            let _ = tcp.connect(self.iface.context(), remote, local);
        }
        if tcp.is_active() {
            self.tcp_connected = true;
            self.pending_tcp_connect = None;
            self.completed = Some(NetResponse::Ok);
            self.op = Op::None;
        }
    }

    fn try_tcp_send(&mut self, sockets: &mut SocketSet<'static>) {
        if self.op != Op::TcpSend || !self.tcp_connected {
            return;
        }
        let Some(payload_len) = self.pending_tcp_send_len else {
            return;
        };
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let Some(tcp_handle) = arena.tcp_handle else {
            return;
        };
        let tcp = sockets.get_mut::<TcpSocket>(tcp_handle);
        if !tcp.may_send() {
            return;
        }
        let payload = &self.pending_tcp_send[..payload_len as usize];
        if tcp.send_slice(payload).is_ok() {
            self.pending_tcp_send_len = None;
            self.completed = Some(NetResponse::Ok);
            self.op = Op::None;
        }
    }

    fn try_tcp_recv(&mut self, sockets: &mut SocketSet<'static>) {
        if self.op != Op::TcpRecv || !self.tcp_connected {
            return;
        }
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let Some(tcp_handle) = arena.tcp_handle else {
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

    fn log_udp_tx_done(&mut self) {
        if self.last_was_udp_tx && !self.udp_tx_logged {
            log::info!("lerux-net: TX ok");
            self.udp_tx_logged = true;
            self.last_was_udp_tx = false;
        }
    }

    pub fn poll(&mut self) {
        self.device.poll();
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let mut sockets = SocketSet::new(&mut arena.storage[..]);
        self.ensure_udp_socket(&mut sockets);
        self.try_udp_tx(&mut sockets);
        self.try_tcp_connect(&mut sockets);
        self.try_tcp_send(&mut sockets);
        self.try_tcp_recv(&mut sockets);
        self.iface
            .poll(Instant::ZERO, &mut self.device, &mut sockets);
        self.log_udp_tx_done();
    }
}
