//! One-client network guest.
//!
//! Dynamic Host Configuration Protocol runs on QEMU user-net. The address line
//! is written to the serial queue, then an Internet Control Message Protocol
//! echo is sent to the gateway. The client maps its own queues and data, not
//! the device queue.

#![no_std]
#![no_main]

extern crate alloc;

use core::mem::size_of;

use alloc::boxed::Box;

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use smoltcp::{
    iface::{Config, Interface, SocketSet, SocketStorage},
    phy::ChecksumCapabilities,
    socket::{
        dhcpv4::{self, Event},
        icmp,
    },
    time::{Duration, Instant},
    wire::{
        EthernetAddress, HardwareAddress, Icmpv4Packet, Icmpv4Repr, IpAddress, IpCidr, Ipv4Address,
    },
};

use lerux_sddf::{
    net_buffers_init, net_client_config_t, net_connection_resource_t, net_image,
    net_queue_handle_t, net_queue_init, serial_client_config_t, serial_enqueue,
    serial_handle_from_connection, serial_image, serial_queue_handle_t, SDDF_NET_MAGIC,
    SDDF_SERIAL_MAGIC,
};

mod device;

use device::NetDevice;

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
const SPINS_PER_VIRTUAL_MS: u32 = 30_000;
const LIMIT_MS: u64 = 30_000;
const PING_INTERVAL_MS: u64 = 2_000;
const ECHO_IDENT: u16 = 0x4c58;
const ECHO_DATA: [u8; 2] = [0x4c, 0x58];
const IP_ERR: &[u8] = b"ip err\n";
const PING_OK: &[u8] = b"ping ok\n";
const PING_ERR: &[u8] = b"ping err\n";
const USER_NET_GATEWAY: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);

struct HandlerImpl;

#[protection_domain(heap_size = 64 * 1024)]
fn init() -> HandlerImpl {
    let serial_len = size_of::<serial_client_config_t>();
    let net_len = size_of::<net_client_config_t>();
    assert_eq!(CONFIG.len(), serial_len + net_len);
    // SAFETY: the build script wrote the serial client page, then the network client page.
    let serial = unsafe {
        serial_image::serial_config_from_bytes::<serial_client_config_t>(&CONFIG[..serial_len])
    };
    let net =
        unsafe { net_image::net_config_from_bytes::<net_client_config_t>(&CONFIG[serial_len..]) };
    assert_eq!(serial.magic, SDDF_SERIAL_MAGIC);
    assert_eq!(net.magic, SDDF_NET_MAGIC);
    // SAFETY: the template maps this client's serial transmit region.
    let serial_tx = unsafe { serial_handle_from_connection(&serial.tx) };
    let serial_ch = Channel::new(usize::from(serial.tx.id));

    let rx = queue_from(&net.rx);
    let tx = queue_from(&net.tx);
    // SAFETY: this client produces the transmit free queue once. The transmit
    // virtualiser is the producer after this returns.
    unsafe { net_buffers_init(&tx, 0) };
    let mut device = NetDevice::new(
        rx,
        tx,
        net.rx_data.vaddr,
        net.tx_data.vaddr,
        Channel::new(usize::from(net.rx.id)),
        Channel::new(usize::from(net.tx.id)),
    );
    let mut iface = Interface::new(
        Config::new(HardwareAddress::Ethernet(EthernetAddress(
            net.mac_addr.addr,
        ))),
        &mut device,
        Instant::from_millis(0),
    );

    let mut dhcp = dhcpv4::Socket::new();
    dhcp.set_retry_config(dhcp_retry());
    let icmp = icmp_socket();
    let storage: &'static mut [SocketStorage<'static>] =
        Box::leak(Box::new([SocketStorage::EMPTY; 2]));
    let mut sockets = SocketSet::new(storage);
    let dhcp_handle = sockets.add(dhcp);
    let icmp_handle = sockets.add(icmp);

    let mut guest = Guest {
        ms: 0,
        iface: &mut iface,
        device: &mut device,
        sockets: &mut sockets,
        serial_tx: &serial_tx,
        serial_ch,
    };
    let Some(gateway) = wait_for_address(&mut guest, dhcp_handle) else {
        return HandlerImpl;
    };
    wait_for_ping(&mut guest, gateway, icmp_handle);
    HandlerImpl
}

struct Guest<'a> {
    ms: u64,
    iface: &'a mut Interface,
    device: &'a mut NetDevice,
    sockets: &'a mut SocketSet<'static>,
    serial_tx: &'a serial_queue_handle_t,
    serial_ch: Channel,
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

fn dhcp_retry() -> dhcpv4::RetryConfig {
    let mut retry = dhcpv4::RetryConfig::default();
    retry.discover_timeout = Duration::from_secs(2);
    retry.initial_request_timeout = Duration::from_secs(2);
    retry.request_retries = 5;
    retry.min_renew_timeout = Duration::from_secs(60);
    retry.max_renew_timeout = Duration::from_secs(120);
    retry
}

fn icmp_socket() -> icmp::Socket<'static> {
    let mut socket = icmp::Socket::new(packet_buffer(), packet_buffer());
    socket
        .bind(icmp::Endpoint::Ident(ECHO_IDENT))
        .expect("echo ident binds");
    socket
}

fn packet_buffer() -> icmp::PacketBuffer<'static> {
    let meta: &'static mut [icmp::PacketMetadata] =
        Box::leak(Box::new([icmp::PacketMetadata::EMPTY; 4]));
    let payload: &'static mut [u8] = Box::leak(Box::new([0u8; 512]));
    icmp::PacketBuffer::new(meta, payload)
}

fn instant(ms: u64) -> Instant {
    Instant::from_millis(i64::try_from(ms).expect("virtual time fits"))
}

fn spin_virtual_ms() {
    for _ in 0..SPINS_PER_VIRTUAL_MS {
        core::hint::spin_loop();
    }
}

fn write_line(tx: &serial_queue_handle_t, tx_ch: Channel, bytes: &[u8]) {
    for &byte in bytes {
        // SAFETY: `tx` points at this client's mapped serial transmit queue.
        let status = unsafe { serial_enqueue(tx, byte) };
        assert_eq!(status, 0, "serial line fits");
    }
    tx_ch.notify();
}

fn format_ip(octets: [u8; 4], out: &mut [u8; 32]) -> usize {
    let mut n = 0;
    for &byte in b"ip " {
        out[n] = byte;
        n += 1;
    }
    for (index, octet) in octets.into_iter().enumerate() {
        if index > 0 {
            out[n] = b'.';
            n += 1;
        }
        push_octet(out, &mut n, octet);
    }
    out[n] = b'\n';
    n += 1;
    n
}

fn push_octet(buf: &mut [u8], n: &mut usize, value: u8) {
    let mut digits = [0u8; 3];
    let mut count = 0;
    let mut rest = value;
    loop {
        digits[count] = b'0' + rest % 10;
        count += 1;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    while count > 0 {
        count -= 1;
        buf[*n] = digits[count];
        *n += 1;
    }
}

fn take_dhcp(
    guest: &mut Guest<'_>,
    dhcp_handle: smoltcp::iface::SocketHandle,
) -> Option<Ipv4Address> {
    let configured = {
        let dhcp = guest.sockets.get_mut::<dhcpv4::Socket>(dhcp_handle);
        match dhcp.poll() {
            Some(Event::Configured(cfg)) => {
                let address = cfg.address.address();
                let prefix = cfg.address.prefix_len();
                let router = cfg.router.unwrap_or(USER_NET_GATEWAY);
                Some((address, prefix, router))
            }
            Some(Event::Deconfigured) | None => None,
        }
    };
    let (address, prefix, router) = configured?;
    guest.iface.update_ip_addrs(|addrs| {
        addrs.clear();
        addrs
            .push(IpCidr::new(IpAddress::Ipv4(address), prefix))
            .expect("address slot");
    });
    guest.iface.routes_mut().remove_default_ipv4_route();
    guest
        .iface
        .routes_mut()
        .add_default_ipv4_route(router)
        .expect("default route");
    let mut line = [0u8; 32];
    let len = format_ip(address.octets(), &mut line);
    write_line(guest.serial_tx, guest.serial_ch, &line[..len]);
    Some(router)
}

fn wait_for_address(
    guest: &mut Guest<'_>,
    dhcp_handle: smoltcp::iface::SocketHandle,
) -> Option<Ipv4Address> {
    loop {
        let _ = guest
            .iface
            .poll(instant(guest.ms), guest.device, guest.sockets);
        if let Some(gateway) = take_dhcp(guest, dhcp_handle) {
            return Some(gateway);
        }
        if guest.ms >= LIMIT_MS {
            write_line(guest.serial_tx, guest.serial_ch, IP_ERR);
            return None;
        }
        spin_virtual_ms();
        guest.ms += 1;
    }
}

fn send_echo(socket: &mut icmp::Socket<'_>, gateway: Ipv4Address, seq_no: u16) {
    let repr = Icmpv4Repr::EchoRequest {
        ident: ECHO_IDENT,
        seq_no,
        data: &ECHO_DATA,
    };
    let mut bytes = [0u8; 16];
    let len = repr.buffer_len();
    assert!(len <= bytes.len(), "echo queued");
    {
        let mut packet = Icmpv4Packet::new_unchecked(&mut bytes[..len]);
        repr.emit(&mut packet, &ChecksumCapabilities::default());
    }
    socket
        .send_slice(&bytes[..len], IpAddress::Ipv4(gateway))
        .expect("echo queued");
}

fn wait_for_ping(
    guest: &mut Guest<'_>,
    gateway: Ipv4Address,
    icmp_handle: smoltcp::iface::SocketHandle,
) {
    let start = guest.ms;
    let mut last_send = None;
    let mut seq_no = 0u16;
    loop {
        let _ = guest
            .iface
            .poll(instant(guest.ms), guest.device, guest.sockets);
        let now = guest.ms;
        let replied = {
            let icmp = guest.sockets.get_mut::<icmp::Socket>(icmp_handle);
            if icmp.can_recv() {
                let _ = icmp.recv();
                true
            } else {
                let due =
                    last_send.is_none_or(|sent: u64| now.saturating_sub(sent) >= PING_INTERVAL_MS);
                if due && icmp.can_send() {
                    send_echo(icmp, gateway, seq_no);
                    last_send = Some(now);
                    seq_no = seq_no.wrapping_add(1);
                }
                false
            }
        };
        if replied {
            write_line(guest.serial_tx, guest.serial_ch, PING_OK);
            return;
        }
        if now.saturating_sub(start) >= LIMIT_MS {
            write_line(guest.serial_tx, guest.serial_ch, PING_ERR);
            return;
        }
        spin_virtual_ms();
        guest.ms += 1;
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, _channels: ChannelSet) -> Result<(), Self::Error> {
        Ok(())
    }
}
