//! Dynamic Host Configuration Protocol, one Transmission Control Protocol
//! socket, and one Transport Layer Security GET.
//!
//! The worker owns the stack. [`fetch`] does not run on the root cothread.
//! [`install`] runs on the root and initialises the transmit free queue once.

extern crate alloc;

use core::{cell::UnsafeCell, mem::MaybeUninit};

use alloc::{boxed::Box, vec::Vec};

use sel4_microkit::{Channel, ChannelSet};

use smoltcp::{
    iface::{Config, Interface, SocketHandle, SocketSet, SocketStorage},
    socket::{
        dhcpv4::{self, Event},
        tcp::{self, SocketBuffer},
    },
    time::{Duration, Instant},
    wire::{
        EthernetAddress, HardwareAddress, IpAddress, IpCidr, IpEndpoint, IpListenEndpoint,
        Ipv4Address,
    },
};

use lerux_cothread::microkit_cothread_recv_ntfn;
use lerux_ladybird_http::{response_body, ResponseBody, BODY_CAP, RESPONSE_SLACK, TLS_NAME};
use lerux_posix::{Errno, SockFd, Sockets, Tcp};
use lerux_sddf::{
    net_buffers_init, net_client_config_t, net_connection_resource_t, net_queue_handle_t,
    net_queue_init,
};
use lerux_tls::{ClientSession, Status};

use crate::net_device::NetDevice;

const SPINS_PER_VIRTUAL_MS: u32 = 30_000;
const LIMIT_MS: u64 = 30_000;
const LOCAL_PORT: u16 = 49152;
const GATEWAY: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);
const STEPS: usize = 48;

struct Map {
    rx: net_queue_handle_t,
    tx: net_queue_handle_t,
    rx_data: *mut u8,
    tx_data: *mut u8,
    rx_ch: usize,
    tx_ch: usize,
    mac: [u8; 6],
}

struct MapCell(UnsafeCell<MaybeUninit<Map>>);

// SAFETY: init writes this cell before the worker starts. The worker is the only reader.
unsafe impl Sync for MapCell {}

static MAP: MapCell = MapCell(UnsafeCell::new(MaybeUninit::uninit()));

struct SocksCell(UnsafeCell<*mut Sockets<Stack>>);

// SAFETY: the worker creates the sockets once and is the only caller afterwards.
unsafe impl Sync for SocksCell {}

static SOCKS: SocksCell = SocksCell(UnsafeCell::new(core::ptr::null_mut()));

pub fn install(net: &net_client_config_t) {
    let rx = queue_from(&net.rx);
    let tx = queue_from(&net.tx);
    // SAFETY: this client produces the transmit free queue once, with base 0.
    unsafe { net_buffers_init(&tx, 0) };
    // SAFETY: init is the only writer, and it runs before the worker.
    unsafe {
        (*MAP.0.get()).write(Map {
            rx,
            tx,
            rx_data: net.rx_data.vaddr,
            tx_data: net.tx_data.vaddr,
            rx_ch: usize::from(net.rx.id),
            tx_ch: usize::from(net.tx.id),
            mac: net.mac_addr.addr,
        });
    }
}

pub fn on_notify(channels: ChannelSet) {
    let map = map();
    if channels.contains(Channel::new(map.rx_ch)) {
        microkit_cothread_recv_ntfn(map.rx_ch);
    }
    if channels.contains(Channel::new(map.tx_ch)) {
        microkit_cothread_recv_ntfn(map.tx_ch);
    }
}

/// Connect to `10.0.2.2` on `port` and copy one response body into `out`.
///
/// `sockets` and the handshake stay in their own frames. Inlining them into
/// this function puts the curve25519 window and the rustls client hello on
/// the same 0x4000 worker stack.
#[inline(never)]
pub fn fetch(request: &[u8], port: u16, out: &mut [u8]) -> Result<usize, ()> {
    let sockets = sockets()?;
    let fd = sockets.socket().map_err(|_| ())?;
    let result = exchange(sockets, fd, request, port, out);
    let _ = sockets.close(fd);
    result
}

#[inline(never)]
fn exchange(
    sockets: &mut Sockets<Stack>,
    fd: SockFd,
    request: &[u8],
    port: u16,
    out: &mut [u8],
) -> Result<usize, ()> {
    sockets
        .connect(fd, GATEWAY.octets(), port)
        .map_err(|_| ())?;
    http_exchange(sockets, fd, request, out)
}

fn map() -> &'static Map {
    // SAFETY: `install` wrote this cell before the worker was spawned.
    unsafe { (*MAP.0.get()).assume_init_ref() }
}

#[inline(never)]
fn sockets() -> Result<&'static mut Sockets<Stack>, ()> {
    // SAFETY: the worker is the only caller, and it does not overlap itself.
    let slot = unsafe { &mut *SOCKS.0.get() };
    if !slot.is_null() {
        return Ok(unsafe { &mut **slot });
    }
    let mut stack = Stack::new();
    if !stack.bring_up() {
        return Err(());
    }
    let sockets = Box::leak(Box::new(Sockets::new(stack)));
    *slot = sockets;
    Ok(sockets)
}

fn queue_from(conn: &net_connection_resource_t) -> net_queue_handle_t {
    let mut handle = net_queue_handle_t {
        free: core::ptr::null_mut(),
        active: core::ptr::null_mut(),
        capacity: 0,
    };
    // SAFETY: the template maps these queues into this protection domain.
    unsafe {
        net_queue_init(
            &mut handle,
            conn.free_queue.vaddr.cast(),
            conn.active_queue.vaddr.cast(),
            u32::from(conn.num_buffers),
        );
    }
    handle
}

fn copy_queue(queue: &net_queue_handle_t) -> net_queue_handle_t {
    net_queue_handle_t {
        free: queue.free,
        active: queue.active,
        capacity: queue.capacity,
    }
}

struct Stack {
    device: NetDevice,
    iface: Interface,
    sockets: SocketSet<'static>,
    dhcp: SocketHandle,
    tcp: SocketHandle,
    ms: u64,
    op_start: u64,
    local_port: u16,
    local_ip: Ipv4Address,
    connecting: bool,
}

impl Stack {
    fn new() -> Self {
        let map = map();
        let mut device = device_from(map);
        let iface = Interface::new(
            Config::new(HardwareAddress::Ethernet(EthernetAddress(map.mac))),
            &mut device,
            Instant::from_millis(0),
        );
        let (sockets, dhcp, tcp) = socket_set();
        Self {
            device,
            iface,
            sockets,
            dhcp,
            tcp,
            ms: 0,
            op_start: 0,
            local_port: LOCAL_PORT,
            local_ip: Ipv4Address::UNSPECIFIED,
            connecting: false,
        }
    }

    fn bring_up(&mut self) -> bool {
        let start = self.ms;
        loop {
            self.poll_iface();
            match self.take_dhcp() {
                Ok(true) => return true,
                Ok(false) => {}
                Err(()) => return false,
            }
            if self.ms.saturating_sub(start) >= LIMIT_MS {
                return false;
            }
            self.advance();
        }
    }

    fn poll_iface(&mut self) {
        let _ = self
            .iface
            .poll(instant(self.ms), &mut self.device, &mut self.sockets);
    }

    fn advance(&mut self) {
        for _ in 0..SPINS_PER_VIRTUAL_MS {
            core::hint::spin_loop();
        }
        self.ms += 1;
    }

    fn take_dhcp(&mut self) -> Result<bool, ()> {
        let Some((address, prefix, router)) = self.dhcp_offer() else {
            return Ok(false);
        };
        self.apply_offer(address, prefix, router)
    }

    fn dhcp_offer(&mut self) -> Option<(Ipv4Address, u8, Ipv4Address)> {
        let dhcp = self.sockets.get_mut::<dhcpv4::Socket>(self.dhcp);
        match dhcp.poll() {
            Some(Event::Configured(cfg)) => Some((
                cfg.address.address(),
                cfg.address.prefix_len(),
                cfg.router.unwrap_or(GATEWAY),
            )),
            Some(Event::Deconfigured) | None => None,
        }
    }

    fn apply_offer(
        &mut self,
        address: Ipv4Address,
        prefix: u8,
        router: Ipv4Address,
    ) -> Result<bool, ()> {
        let mut stored = false;
        self.iface.update_ip_addrs(|addrs| {
            addrs.clear();
            stored = addrs
                .push(IpCidr::new(IpAddress::Ipv4(address), prefix))
                .is_ok();
        });
        if !stored {
            return Err(());
        }
        self.iface.routes_mut().remove_default_ipv4_route();
        self.iface
            .routes_mut()
            .add_default_ipv4_route(router)
            .map_err(|_| ())?;
        self.local_ip = address;
        Ok(true)
    }

    fn within_budget(&self) -> bool {
        self.ms.saturating_sub(self.op_start) < LIMIT_MS
    }
}

fn device_from(map: &Map) -> NetDevice {
    NetDevice::new(
        copy_queue(&map.rx),
        copy_queue(&map.tx),
        map.rx_data,
        map.tx_data,
        Channel::new(map.rx_ch),
        Channel::new(map.tx_ch),
    )
}

fn socket_set() -> (SocketSet<'static>, SocketHandle, SocketHandle) {
    let storage: &'static mut [SocketStorage<'static>] =
        Box::leak(Box::new([SocketStorage::EMPTY; 4]));
    let mut sockets = SocketSet::new(storage);
    let mut dhcp = dhcpv4::Socket::new();
    dhcp.set_retry_config(dhcp_retry());
    let dhcp = sockets.add(dhcp);
    let rx: &'static mut [u8] = Box::leak(Box::new([0u8; 16_384]));
    let tx: &'static mut [u8] = Box::leak(Box::new([0u8; 16_384]));
    let tcp = sockets.add(tcp::Socket::new(
        SocketBuffer::new(rx),
        SocketBuffer::new(tx),
    ));
    (sockets, dhcp, tcp)
}

impl Tcp for Stack {
    fn start(&mut self) {
        self.op_start = self.ms;
    }

    fn poll(&mut self) -> bool {
        if !self.within_budget() {
            return false;
        }
        self.advance();
        self.poll_iface();
        self.within_budget()
    }

    fn connect(&mut self, addr: [u8; 4], port: u16) -> Result<(), Errno> {
        let remote = IpEndpoint::new(
            IpAddress::Ipv4(Ipv4Address::new(addr[0], addr[1], addr[2], addr[3])),
            port,
        );
        let local = IpListenEndpoint {
            addr: Some(IpAddress::Ipv4(self.local_ip)),
            port: self.local_port,
        };
        self.local_port = self.local_port.saturating_add(1);
        let handle = self.tcp;
        {
            let iface = &mut self.iface;
            let sockets = &mut self.sockets;
            sockets
                .get_mut::<tcp::Socket>(handle)
                .connect(iface.context(), remote, local)
                .map_err(|_| Errno(Errno::ECONNREFUSED))?;
        }
        self.connecting = true;
        Ok(())
    }

    fn ready(&self) -> bool {
        self.sockets.get::<tcp::Socket>(self.tcp).may_send()
    }

    fn failed(&self) -> bool {
        let closed = self.sockets.get::<tcp::Socket>(self.tcp).state() == tcp::State::Closed;
        self.connecting && closed
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, Errno> {
        let tcp = self.sockets.get_mut::<tcp::Socket>(self.tcp);
        if !tcp.may_send() {
            return Err(Errno(Errno::EAGAIN));
        }
        tcp.send_slice(data).map_err(|_| Errno(Errno::EPIPE))
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Errno> {
        let tcp = self.sockets.get_mut::<tcp::Socket>(self.tcp);
        if tcp.can_recv() {
            return tcp.recv_slice(buf).map_err(|_| Errno(Errno::EPIPE));
        }
        if tcp.may_recv() {
            return Err(Errno(Errno::EAGAIN));
        }
        Ok(0)
    }

    fn finish(&mut self) {
        let tcp = self.sockets.get_mut::<tcp::Socket>(self.tcp);
        if tcp.is_open() {
            tcp.abort();
        }
        self.connecting = false;
        self.poll_iface();
    }
}

fn dhcp_retry() -> dhcpv4::RetryConfig {
    let mut retry = dhcpv4::RetryConfig::default();
    retry.discover_timeout = Duration::from_secs(2);
    retry.initial_request_timeout = Duration::from_secs(2);
    retry.request_retries = 5;
    retry.min_renew_timeout = Duration::from_secs(60);
    retry.max_renew_timeout = Duration::from_secs(120);
    retry
}

fn instant(ms: u64) -> Instant {
    Instant::from_millis(i64::try_from(ms).unwrap_or(i64::MAX))
}

#[inline(never)]
fn http_exchange(
    sockets: &mut Sockets<Stack>,
    fd: SockFd,
    request: &[u8],
    out: &mut [u8],
) -> Result<usize, ()> {
    let mut session = Box::new(ClientSession::new(TLS_NAME).map_err(|_| ())?);
    let mut raw = Vec::new();
    let mut requested = false;
    for _ in 0..STEPS {
        if let Some(len) = take_ready(&raw, out)? {
            return Ok(len);
        }
        step(&mut session, sockets, fd, request, &mut raw, &mut requested)?;
    }
    take_ready(&raw, out)?.ok_or(())
}

fn step(
    session: &mut ClientSession,
    sockets: &mut Sockets<Stack>,
    fd: SockFd,
    request: &[u8],
    raw: &mut Vec<u8>,
    requested: &mut bool,
) -> Result<(), ()> {
    match session.drive().map_err(|_| ())? {
        Status::WantSend => send_pending(session, sockets, fd),
        Status::WantRecv => feed(session, sockets, fd),
        Status::Connected => on_connected(session, sockets, fd, request, raw, requested),
        Status::Closed => Err(()),
    }
}

fn on_connected(
    session: &mut ClientSession,
    sockets: &mut Sockets<Stack>,
    fd: SockFd,
    request: &[u8],
    raw: &mut Vec<u8>,
    requested: &mut bool,
) -> Result<(), ()> {
    let mut plain = [0u8; 256];
    let n = session.read_plain(&mut plain);
    if n > 0 {
        return push_plain(raw, &plain[..n]);
    }
    if !*requested {
        session.write_plain(request).map_err(|_| ())?;
        *requested = true;
    }
    if session.has_outgoing() {
        return send_pending(session, sockets, fd);
    }
    feed(session, sockets, fd)
}

fn send_pending(
    session: &mut ClientSession,
    sockets: &mut Sockets<Stack>,
    fd: SockFd,
) -> Result<(), ()> {
    let pending = session.take_outgoing();
    sockets.send(fd, &pending).map_err(|_| ())?;
    Ok(())
}

fn feed(session: &mut ClientSession, sockets: &mut Sockets<Stack>, fd: SockFd) -> Result<(), ()> {
    let mut buf = [0u8; 512];
    let n = sockets.recv(fd, &mut buf).map_err(|_| ())?;
    if n == 0 {
        return Err(());
    }
    session.feed_incoming(&buf[..n]);
    Ok(())
}

fn push_plain(raw: &mut Vec<u8>, plain: &[u8]) -> Result<(), ()> {
    let cap = BODY_CAP.saturating_add(RESPONSE_SLACK);
    let next = raw.len().checked_add(plain.len()).ok_or(())?;
    if next > cap {
        return Err(());
    }
    raw.extend_from_slice(plain);
    Ok(())
}

fn take_ready(raw: &[u8], out: &mut [u8]) -> Result<Option<usize>, ()> {
    match response_body(raw, out.len()) {
        ResponseBody::Pending => Ok(None),
        ResponseBody::Error => Err(()),
        ResponseBody::Ready(body) => {
            out[..body.len()].copy_from_slice(body);
            Ok(Some(body.len()))
        }
    }
}
