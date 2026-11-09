//! Dynamic Host Configuration Protocol, one Transmission Control Protocol
//! socket, and a Transport Layer Security fetch of the host fixture.
//!
//! The browser image also queries `host` on the fixed resolver and follows
//! one relative redirect. The worker owns the stack. `fetch_body` does not
//! run on the root cothread.

extern crate alloc;

use core::{cell::UnsafeCell, mem::MaybeUninit};

use alloc::boxed::Box;

use sel4_microkit::{Channel, ChannelSet};

#[cfg(feature = "browser-net")]
use smoltcp::socket::dns::{self, GetQueryResultError};
#[cfg(feature = "browser-net")]
use smoltcp::wire::DnsQueryType;
use smoltcp::{
    iface::{Interface, SocketHandle, SocketSet, SocketStorage},
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
use lerux_posix::{Errno, SockFd, Sockets, Tcp};
use lerux_sddf::{
    net_buffers_init, net_client_config_t, net_connection_resource_t,
    net_image::SHELL_NET_RX_CHANNEL, net_queue_handle_t, net_queue_init,
};
#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )
))]
use lerux_tls::{ClientSession, Status};

use crate::net_device::NetDevice;

const SPINS_PER_VIRTUAL_MS: u32 = 30_000;
const LIMIT_MS: u64 = 30_000;
const LOCAL_PORT: u16 = 49152;
const GATEWAY: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);
#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )
))]
const FIXTURE_PORT: u16 = 8443;
#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
const ECHO_PORT: u16 = 18080;
#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
const CHAT_LINE: &[u8] = b"chat\n";
#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )
))]
const HTTP_GET: &[u8] = b"GET /fixture.html HTTP/1.1\r\nHost: host\r\nConnection: close\r\n\r\n";
#[cfg(feature = "browser-net")]
const HTTP_GO: &[u8] = b"GET /go HTTP/1.1\r\nHost: host\r\nConnection: close\r\n\r\n";
#[cfg(feature = "browser-net")]
const DNS_SERVER: Ipv4Address = Ipv4Address::new(10, 0, 2, 3);
#[cfg(any(
    feature = "board-qemu_virt_aarch64_web_sddf",
    feature = "board-qemu_virt_riscv64_web_sddf",
    feature = "board-x86_64_generic_web_sddf"
))]
const WEB_PORT: u16 = 8080;

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
    // SAFETY: this client produces the transmit free queue once.
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

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )
))]
pub fn fetch_body(out: &mut [u8]) -> Result<usize, ()> {
    let sockets = sockets()?;
    let fd = sockets.socket().map_err(|_| ())?;
    let result = (|| {
        sockets
            .connect(fd, GATEWAY.octets(), FIXTURE_PORT)
            .map_err(|_| ())?;
        http_body(sockets, fd, out)
    })();
    let _ = sockets.close(fd);
    result
}

/// Resolve `host`, follow one relative redirect, and return that address
/// with the final body.
#[cfg(feature = "browser-net")]
pub(crate) fn fetch_named(out: &mut [u8]) -> Result<([u8; 4], usize), ()> {
    let sockets = sockets()?;
    let addr = sockets.with_pipe(Stack::resolve_host)?;
    let path = redirect_path(sockets, addr)?;
    let n = follow_path(sockets, addr, path.bytes(), out)?;
    Ok((addr, n))
}

/// The baked-in smoke certificate authority loads.
#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
pub fn cert_ready() -> bool {
    ClientSession::new("host").is_ok()
}

/// Send one line to the host echo and return the bytes it wrote back.
#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf"
))]
pub fn chat_echo(out: &mut [u8]) -> Result<usize, ()> {
    let sockets = sockets()?;
    let fd = sockets.socket().map_err(|_| ())?;
    let result = (|| {
        sockets
            .connect(fd, GATEWAY.octets(), ECHO_PORT)
            .map_err(|_| ())?;
        sockets.send(fd, CHAT_LINE).map_err(|_| ())?;
        sockets.recv(fd, out).map_err(|_| ())
    })();
    let _ = sockets.close(fd);
    result
}

fn map() -> &'static Map {
    // SAFETY: `install` wrote this cell before the worker was spawned.
    unsafe { (*MAP.0.get()).assume_init_ref() }
}

fn sockets() -> Result<&'static mut Sockets<Stack>, ()> {
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

fn copy_queue(queue: &net_queue_handle_t) -> net_queue_handle_t {
    net_queue_handle_t {
        free: queue.free,
        active: queue.active,
        capacity: queue.capacity,
    }
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

struct Stack {
    device: NetDevice,
    iface: Interface,
    sockets: SocketSet<'static>,
    dhcp: SocketHandle,
    tcp: SocketHandle,
    #[cfg(feature = "browser-net")]
    dns: SocketHandle,
    ms: u64,
    op_start: u64,
    local_port: u16,
    local_ip: Ipv4Address,
    connecting: bool,
    listening: bool,
}

impl Stack {
    fn new() -> Self {
        let map = map();
        let mut device = NetDevice::new(
            copy_queue(&map.rx),
            copy_queue(&map.tx),
            map.rx_data,
            map.tx_data,
            Channel::new(map.rx_ch),
            Channel::new(map.tx_ch),
        );
        let iface = Interface::new(
            smoltcp::iface::Config::new(HardwareAddress::Ethernet(EthernetAddress(map.mac))),
            &mut device,
            Instant::from_millis(0),
        );
        let mut sockets = empty_sockets();
        let dhcp = add_dhcp(&mut sockets);
        let tcp = add_tcp(&mut sockets);
        #[cfg(feature = "browser-net")]
        let dns = add_dns(&mut sockets);
        Self {
            device,
            iface,
            sockets,
            dhcp,
            tcp,
            #[cfg(feature = "browser-net")]
            dns,
            ms: 0,
            op_start: 0,
            local_port: LOCAL_PORT,
            local_ip: Ipv4Address::UNSPECIFIED,
            connecting: false,
            listening: false,
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
        let configured = {
            let dhcp = self.sockets.get_mut::<dhcpv4::Socket>(self.dhcp);
            match dhcp.poll() {
                Some(Event::Configured(cfg)) => {
                    let address = cfg.address.address();
                    let prefix = cfg.address.prefix_len();
                    let router = cfg.router.unwrap_or(GATEWAY);
                    Some((address, prefix, router))
                }
                Some(Event::Deconfigured) | None => None,
            }
        };
        let Some((address, prefix, router)) = configured else {
            return Ok(false);
        };
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
        if self
            .iface
            .routes_mut()
            .add_default_ipv4_route(router)
            .is_err()
        {
            return Err(());
        }
        self.local_ip = address;
        Ok(true)
    }

    fn within_budget(&self) -> bool {
        self.ms.saturating_sub(self.op_start) < LIMIT_MS
    }

    #[cfg(feature = "browser-net")]
    fn resolve_host(&mut self) -> Result<[u8; 4], ()> {
        let query = self.start_dns()?;
        let start = self.ms;
        loop {
            self.poll_iface();
            match self.dns_result(query) {
                DnsPoll::Wait => {}
                DnsPoll::Addr(addr) => return Ok(addr),
                DnsPoll::Fail => return Err(()),
            }
            if self.ms.saturating_sub(start) >= LIMIT_MS {
                self.cancel_dns(query);
                return Err(());
            }
            self.advance();
        }
    }

    #[cfg(feature = "browser-net")]
    fn cancel_dns(&mut self, query: dns::QueryHandle) {
        self.sockets
            .get_mut::<dns::Socket>(self.dns)
            .cancel_query(query);
    }

    #[cfg(feature = "browser-net")]
    fn start_dns(&mut self) -> Result<dns::QueryHandle, ()> {
        let handle = self.dns;
        let iface = &mut self.iface;
        let sockets = &mut self.sockets;
        sockets
            .get_mut::<dns::Socket>(handle)
            .start_query(iface.context(), "host", DnsQueryType::A)
            .map_err(|_| ())
    }

    #[cfg(feature = "browser-net")]
    fn dns_result(&mut self, query: dns::QueryHandle) -> DnsPoll {
        match self
            .sockets
            .get_mut::<dns::Socket>(self.dns)
            .get_query_result(query)
        {
            Ok(addrs) => first_ipv4(&addrs),
            Err(GetQueryResultError::Pending) => DnsPoll::Wait,
            Err(GetQueryResultError::Failed) => DnsPoll::Fail,
        }
    }
}

fn empty_sockets() -> SocketSet<'static> {
    let storage: &'static mut [SocketStorage<'static>] =
        Box::leak(Box::new([SocketStorage::EMPTY; 4]));
    SocketSet::new(storage)
}

fn add_dhcp(sockets: &mut SocketSet<'static>) -> SocketHandle {
    let mut dhcp = dhcpv4::Socket::new();
    dhcp.set_retry_config(dhcp_retry());
    sockets.add(dhcp)
}

fn add_tcp(sockets: &mut SocketSet<'static>) -> SocketHandle {
    let rx: &'static mut [u8] = Box::leak(Box::new([0u8; 16_384]));
    let tx: &'static mut [u8] = Box::leak(Box::new([0u8; 16_384]));
    sockets.add(tcp::Socket::new(
        SocketBuffer::new(rx),
        SocketBuffer::new(tx),
    ))
}

#[cfg(feature = "browser-net")]
fn add_dns(sockets: &mut SocketSet<'static>) -> SocketHandle {
    let queries: &'static mut [Option<dns::DnsQuery>] = Box::leak(Box::new([None]));
    let server = IpAddress::Ipv4(DNS_SERVER);
    sockets.add(dns::Socket::new(&[server], queries))
}

#[cfg(feature = "browser-net")]
enum DnsPoll {
    Wait,
    Addr([u8; 4]),
    Fail,
}

#[cfg(feature = "browser-net")]
fn first_ipv4(addrs: &[IpAddress]) -> DnsPoll {
    let Some(addr) = addrs.first() else {
        return DnsPoll::Fail;
    };
    let IpAddress::Ipv4(v4) = *addr;
    DnsPoll::Addr(v4.octets())
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
        let port = self.local_port;
        self.local_port = self.local_port.saturating_add(1);
        let local = IpListenEndpoint {
            addr: Some(IpAddress::Ipv4(self.local_ip)),
            port,
        };
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
        self.listening = false;
        Ok(())
    }

    fn listen(&mut self, port: u16) -> Result<(), Errno> {
        let local = IpListenEndpoint { addr: None, port };
        let handle = self.tcp;
        self.sockets
            .get_mut::<tcp::Socket>(handle)
            .listen(local)
            .map_err(|err| match err {
                tcp::ListenError::Unaddressable => Errno(Errno::EINVAL),
                tcp::ListenError::InvalidState => Errno(Errno::EADDRINUSE),
            })?;
        self.connecting = false;
        self.listening = true;
        Ok(())
    }

    fn ready(&self) -> bool {
        self.sockets.get::<tcp::Socket>(self.tcp).may_send()
    }

    fn failed(&self) -> bool {
        let closed = self.sockets.get::<tcp::Socket>(self.tcp).state() == tcp::State::Closed;
        (self.connecting || self.listening) && closed
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
        self.listening = false;
        self.poll_iface();
    }

    fn shutdown(&mut self) -> Result<bool, Errno> {
        let handle = self.tcp;
        let done = {
            let tcp = self.sockets.get_mut::<tcp::Socket>(handle);
            tcp.close();
            !tcp.is_open()
        };
        if done {
            self.connecting = false;
            self.listening = false;
        }
        Ok(done)
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

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )
))]
fn http_body(sockets: &mut Sockets<Stack>, fd: SockFd, out: &mut [u8]) -> Result<usize, ()> {
    let mut raw = [0u8; 1024];
    let n = http_exchange(sockets, fd, HTTP_GET, &mut raw)?;
    copy_body(&raw[..n], out).ok_or(())
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )
))]
struct HttpRead {
    raw_len: usize,
    requested: bool,
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )
))]
fn http_exchange(
    sockets: &mut Sockets<Stack>,
    fd: SockFd,
    request: &[u8],
    raw: &mut [u8],
) -> Result<usize, ()> {
    let mut session = ClientSession::new("host").map_err(|_| ())?;
    let mut state = HttpRead {
        raw_len: 0,
        requested: false,
    };
    for _ in 0..48 {
        if response_ready(&raw[..state.raw_len]) {
            return Ok(state.raw_len);
        }
        step_tls(&mut session, sockets, fd, request, raw, &mut state)?;
    }
    if response_ready(&raw[..state.raw_len]) {
        Ok(state.raw_len)
    } else {
        Err(())
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    feature = "browser-net"
))]
fn step_tls(
    session: &mut ClientSession,
    sockets: &mut Sockets<Stack>,
    fd: SockFd,
    request: &[u8],
    raw: &mut [u8],
    state: &mut HttpRead,
) -> Result<(), ()> {
    match session.drive().map_err(|_| ())? {
        Status::WantSend => send_pending(session, sockets, fd),
        Status::WantRecv => feed(session, sockets, fd),
        Status::Connected => on_connected(session, sockets, fd, request, raw, state),
        Status::Closed => Err(()),
    }
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    feature = "browser-net"
))]
fn on_connected(
    session: &mut ClientSession,
    sockets: &mut Sockets<Stack>,
    fd: SockFd,
    request: &[u8],
    raw: &mut [u8],
    state: &mut HttpRead,
) -> Result<(), ()> {
    if take_plain(session, raw, state)? {
        return Ok(());
    }
    if !state.requested {
        session.write_plain(request).map_err(|_| ())?;
        state.requested = true;
    }
    // `drive` stays on `Connected` while ciphertext sits in the session.
    if session.has_outgoing() {
        return send_pending(session, sockets, fd);
    }
    feed(session, sockets, fd)
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    feature = "browser-net"
))]
fn take_plain(
    session: &mut ClientSession,
    raw: &mut [u8],
    state: &mut HttpRead,
) -> Result<bool, ()> {
    let mut plain = [0u8; 256];
    let n = session.read_plain(&mut plain);
    if n == 0 {
        return Ok(false);
    }
    let end = state.raw_len.checked_add(n).ok_or(())?;
    if end > raw.len() {
        return Err(());
    }
    raw[state.raw_len..end].copy_from_slice(&plain[..n]);
    state.raw_len = end;
    Ok(true)
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    feature = "browser-net"
))]
fn send_pending(
    session: &mut ClientSession,
    sockets: &mut Sockets<Stack>,
    fd: SockFd,
) -> Result<(), ()> {
    let pending = session.take_outgoing();
    sockets.send(fd, &pending).map_err(|_| ())?;
    Ok(())
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    feature = "browser-net"
))]
fn response_ready(raw: &[u8]) -> bool {
    let Some(split) = raw.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let Some(len) = content_length(&raw[..split]) else {
        return false;
    };
    raw.len() >= split + 4 + len
}

#[cfg(feature = "browser-net")]
struct RelPath {
    buf: [u8; 64],
    len: usize,
}

#[cfg(feature = "browser-net")]
impl RelPath {
    fn from_response(raw: &[u8]) -> Result<Self, ()> {
        if http_status(raw) != Some(302) {
            return Err(());
        }
        let path = location_path(raw)?;
        let mut buf = [0u8; 64];
        buf[..path.len()].copy_from_slice(path);
        Ok(Self {
            buf,
            len: path.len(),
        })
    }

    fn bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

#[cfg(feature = "browser-net")]
fn redirect_path(sockets: &mut Sockets<Stack>, addr: [u8; 4]) -> Result<RelPath, ()> {
    let mut raw = [0u8; 1024];
    let n = connect_exchange(sockets, addr, HTTP_GO, &mut raw)?;
    RelPath::from_response(&raw[..n])
}

#[cfg(feature = "browser-net")]
fn follow_path(
    sockets: &mut Sockets<Stack>,
    addr: [u8; 4],
    path: &[u8],
    out: &mut [u8],
) -> Result<usize, ()> {
    let mut raw = [0u8; 1024];
    let mut request = [0u8; 128];
    let nreq = write_get(path, &mut request)?;
    let n = connect_exchange(sockets, addr, &request[..nreq], &mut raw)?;
    if http_status(&raw[..n]) != Some(200) {
        return Err(());
    }
    copy_body(&raw[..n], out).ok_or(())
}

#[cfg(feature = "browser-net")]
fn connect_exchange(
    sockets: &mut Sockets<Stack>,
    addr: [u8; 4],
    request: &[u8],
    raw: &mut [u8],
) -> Result<usize, ()> {
    let fd = sockets.socket().map_err(|_| ())?;
    let result = (|| {
        sockets.connect(fd, addr, FIXTURE_PORT).map_err(|_| ())?;
        http_exchange(sockets, fd, request, raw)
    })();
    let _ = sockets.close(fd);
    result
}

#[cfg(feature = "browser-net")]
fn write_get(path: &[u8], out: &mut [u8]) -> Result<usize, ()> {
    let prefix = b"GET ";
    let suffix = b" HTTP/1.1\r\nHost: host\r\nConnection: close\r\n\r\n";
    let n = prefix.len() + path.len() + suffix.len();
    if n > out.len() {
        return Err(());
    }
    out[..prefix.len()].copy_from_slice(prefix);
    let mid = prefix.len() + path.len();
    out[prefix.len()..mid].copy_from_slice(path);
    out[mid..n].copy_from_slice(suffix);
    Ok(n)
}

#[cfg(feature = "browser-net")]
fn http_status(raw: &[u8]) -> Option<u16> {
    let line_end = raw.iter().position(|&byte| byte == b'\r')?;
    let mut parts = raw[..line_end].split(|&byte| byte == b' ');
    let version = parts.next()?;
    if !version.starts_with(b"HTTP/") {
        return None;
    }
    let code = parts.next()?;
    if code.len() != 3 || !code.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let mut value = 0u16;
    for &byte in code {
        value = value
            .saturating_mul(10)
            .saturating_add(u16::from(byte - b'0'));
    }
    Some(value)
}

#[cfg(feature = "browser-net")]
fn location_path(raw: &[u8]) -> Result<&[u8], ()> {
    let key = b"\r\nLocation: ";
    let start = raw
        .windows(key.len())
        .position(|window| window == key)
        .ok_or(())?
        + key.len();
    let rest = raw.get(start..).ok_or(())?;
    let end = rest.iter().position(|&byte| byte == b'\r').ok_or(())?;
    let path = rest.get(..end).ok_or(())?;
    if relative_path(path) {
        Ok(path)
    } else {
        Err(())
    }
}

#[cfg(feature = "browser-net")]
fn relative_path(path: &[u8]) -> bool {
    if path.is_empty() || path.len() > 64 || path[0] != b'/' {
        return false;
    }
    if path.len() >= 2 && path[1] == b'/' {
        return false;
    }
    path.iter()
        .all(|&byte| byte != b' ' && byte != b'\r' && byte != b'\n' && byte != b':')
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )
))]
fn feed(session: &mut ClientSession, sockets: &mut Sockets<Stack>, fd: SockFd) -> Result<(), ()> {
    let mut buf = [0u8; 512];
    let n = sockets.recv(fd, &mut buf).map_err(|_| ())?;
    if n == 0 {
        return Err(());
    }
    session.feed_incoming(&buf[..n]);
    Ok(())
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )
))]
fn copy_body(raw: &[u8], out: &mut [u8]) -> Option<usize> {
    let split = raw.windows(4).position(|window| window == b"\r\n\r\n")?;
    let len = content_length(&raw[..split])?;
    let body = raw.get(split + 4..)?;
    if body.len() < len || len > out.len() {
        return None;
    }
    out[..len].copy_from_slice(&body[..len]);
    Some(len)
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_fetch_sddf",
    feature = "board-qemu_virt_riscv64_fetch_sddf",
    feature = "board-x86_64_generic_fetch_sddf",
    any(
        feature = "board-qemu_virt_aarch64_browser_sddf",
        feature = "board-qemu_virt_riscv64_browser_sddf"
    )
))]
fn content_length(headers: &[u8]) -> Option<usize> {
    let key = b"Content-Length: ";
    let start = headers
        .windows(key.len())
        .position(|window| window == key)?;
    let mut value = 0usize;
    let mut saw = false;
    for &byte in &headers[start + key.len()..] {
        if !byte.is_ascii_digit() {
            break;
        }
        saw = true;
        value = value
            .checked_mul(10)?
            .checked_add(usize::from(byte - b'0'))?;
    }
    saw.then_some(value)
}

const _: () = assert!(SHELL_NET_RX_CHANNEL == 3);

/// Listen on the guest Hypertext Transfer Protocol port.
///
/// Dynamic Host Configuration Protocol runs first. The caller prints its ready
/// line before [`serve_one`], so the host can connect while this worker waits.
#[cfg(any(
    feature = "board-qemu_virt_aarch64_web_sddf",
    feature = "board-qemu_virt_riscv64_web_sddf",
    feature = "board-x86_64_generic_web_sddf"
))]
pub fn begin_listen() -> Result<SockFd, ()> {
    let sockets = sockets()?;
    let fd = sockets.socket().map_err(|_| ())?;
    if sockets.listen(fd, WEB_PORT).is_err() {
        let _ = sockets.close(fd);
        return Err(());
    }
    Ok(fd)
}

/// Read one request and write `body` as the response.
#[cfg(any(
    feature = "board-qemu_virt_aarch64_web_sddf",
    feature = "board-qemu_virt_riscv64_web_sddf",
    feature = "board-x86_64_generic_web_sddf"
))]
pub fn serve_one(fd: SockFd, body: &[u8]) -> Result<(), ()> {
    let sockets = sockets()?;
    let result = (|| {
        sockets.accept(fd).map_err(|_| ())?;
        read_headers(sockets, fd)?;
        let mut header = [0u8; 96];
        let n = write_header(body.len(), &mut header);
        sockets.send(fd, &header[..n]).map_err(|_| ())?;
        if !body.is_empty() {
            sockets.send(fd, body).map_err(|_| ())?;
        }
        sockets.shutdown(fd).map_err(|_| ())
    })();
    if result.is_err() {
        let _ = sockets.close(fd);
    }
    result
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_web_sddf",
    feature = "board-qemu_virt_riscv64_web_sddf",
    feature = "board-x86_64_generic_web_sddf"
))]
fn read_headers(sockets: &mut Sockets<Stack>, fd: SockFd) -> Result<(), ()> {
    let mut req = [0u8; 512];
    let mut len = 0usize;
    while !req[..len].windows(4).any(|window| window == b"\r\n\r\n") {
        if len == req.len() {
            return Err(());
        }
        let n = sockets.recv(fd, &mut req[len..]).map_err(|_| ())?;
        if n == 0 {
            return Err(());
        }
        len += n;
    }
    Ok(())
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_web_sddf",
    feature = "board-qemu_virt_riscv64_web_sddf",
    feature = "board-x86_64_generic_web_sddf"
))]
fn write_header(len: usize, out: &mut [u8; 96]) -> usize {
    const HEAD: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: ";
    const TAIL: &[u8] = b"\r\nConnection: close\r\n\r\n";
    out[..HEAD.len()].copy_from_slice(HEAD);
    let mut n = HEAD.len();
    let mut digits = [0u8; 20];
    let mut value = len;
    let mut count = 0usize;
    loop {
        let digit = u8::try_from(value % 10).unwrap_or(0);
        digits[count] = b'0' + digit;
        count += 1;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    while count > 0 {
        count -= 1;
        out[n] = digits[count];
        n += 1;
    }
    out[n..n + TAIL.len()].copy_from_slice(TAIL);
    n + TAIL.len()
}
