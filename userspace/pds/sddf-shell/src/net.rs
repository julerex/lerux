//! Dynamic Host Configuration Protocol, one Transmission Control Protocol
//! socket, and a Transport Layer Security fetch of the host fixture.
//!
//! The worker owns the stack. `fetch_body` does not run on the root cothread.

extern crate alloc;

use core::{cell::UnsafeCell, mem::MaybeUninit};

use alloc::boxed::Box;

use sel4_microkit::{Channel, ChannelSet};

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
    let mut stack = Stack::new()?;
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
    ms: u64,
    op_start: u64,
    local_port: u16,
    local_ip: Ipv4Address,
    connecting: bool,
    listening: bool,
}

impl Stack {
    fn new() -> Result<Self, ()> {
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
        Ok(Self {
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
            listening: false,
        })
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
    let mut session = ClientSession::new("host").map_err(|_| ())?;
    let mut requested = false;
    let mut raw = [0u8; 1024];
    let mut raw_len = 0usize;
    for _ in 0..48 {
        if let Some(n) = copy_body(&raw[..raw_len], out) {
            return Ok(n);
        }
        match session.drive().map_err(|_| ())? {
            Status::WantSend => {
                let pending = session.take_outgoing();
                sockets.send(fd, &pending).map_err(|_| ())?;
            }
            Status::WantRecv => feed(&mut session, sockets, fd)?,
            Status::Connected => {
                let mut plain = [0u8; 256];
                let n = session.read_plain(&mut plain);
                if n > 0 {
                    if raw_len + n > raw.len() {
                        return Err(());
                    }
                    raw[raw_len..raw_len + n].copy_from_slice(&plain[..n]);
                    raw_len += n;
                    continue;
                }
                if !requested {
                    session.write_plain(HTTP_GET).map_err(|_| ())?;
                    requested = true;
                }
                // `drive` stays on `Connected` while ciphertext sits in the session.
                if session.has_outgoing() {
                    let pending = session.take_outgoing();
                    sockets.send(fd, &pending).map_err(|_| ())?;
                    continue;
                }
                feed(&mut session, sockets, fd)?;
            }
            Status::Closed => return Err(()),
        }
    }
    copy_body(&raw[..raw_len], out).ok_or(())
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
