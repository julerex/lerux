use lerux_logging::log;
use sel4_abstract_allocator::WithAlignmentBound;
use sel4_abstract_allocator::basic::BasicAllocator;
use sel4_driver_interfaces::net::MacAddress;
use sel4_microkit::memory_region_symbol;
use sel4_microkit::Channel;
use sel4_shared_memory::SharedMemoryRef;
use sel4_shared_ring_buffer::RingBuffers;
use sel4_shared_ring_buffer_smoltcp::DeviceImpl;
use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet, SocketStorage};
use smoltcp::phy::{DeviceCapabilities, Medium};
use smoltcp::socket::tcp::{Socket as TcpSocket, SocketBuffer as TcpSocketBuffer};
use smoltcp::socket::udp::{PacketBuffer, PacketMetadata, Socket as UdpSocket};
use smoltcp::time::Instant;
use smoltcp::wire::{
    EthernetAddress, HardwareAddress, IpAddress, IpCidr, IpEndpoint, IpListenEndpoint, Ipv4Address,
};

use crate::config;

const NET_DRIVER: Channel = Channel::new(1);
const GUEST_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 15);
const HOST_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);
const TCP_ECHO_PORT: u16 = 18080;

struct SocketArena {
    storage: [SocketStorage<'static>; 2],
    udp_rx_meta: [PacketMetadata; 1],
    udp_rx_payload: [u8; 128],
    udp_tx_meta: [PacketMetadata; 1],
    udp_tx_payload: [u8; 128],
    tcp_cli_rx: [u8; 512],
    tcp_cli_tx: [u8; 512],
    udp_handle: Option<SocketHandle>,
    tcp_cli_handle: Option<SocketHandle>,
    initialized: bool,
}

impl SocketArena {
    const fn empty() -> Self {
        Self {
            storage: [SocketStorage::EMPTY; 2],
            udp_rx_meta: [PacketMetadata::EMPTY],
            udp_rx_payload: [0; 128],
            udp_tx_meta: [PacketMetadata::EMPTY],
            udp_tx_payload: [0; 128],
            tcp_cli_rx: [0; 512],
            tcp_cli_tx: [0; 512],
            udp_handle: None,
            tcp_cli_handle: None,
            initialized: false,
        }
    }
}

static mut SOCKET_ARENA: SocketArena = SocketArena::empty();

pub struct NetIo {
    device: DeviceImpl<WithAlignmentBound<BasicAllocator>>,
    iface: Interface,
    udp_tx_done: bool,
    udp_tx_logged: bool,
    tcp_client_sent: bool,
    tcp_rx_done: bool,
    done: bool,
}

impl NetIo {
    pub fn new(mac: MacAddress) -> Self {
        let notify_net: fn() = || NET_DRIVER.notify();

        let dma_region = unsafe {
            SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
                virtio_net_client_dma_vaddr: *mut [u8],
                n = config::VIRTIO_NET_CLIENT_DMA_SIZE
            ))
        };

        let bounce_buffer_allocator =
            WithAlignmentBound::new(BasicAllocator::new(dma_region.as_ptr().len()), 1);

        let rx_ring_buffers =
            RingBuffers::from_ptrs_using_default_initialization_strategy_for_role(
                unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_rx_free: *mut _)) },
                unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_rx_used: *mut _)) },
                notify_net,
            );

        let tx_ring_buffers =
            RingBuffers::from_ptrs_using_default_initialization_strategy_for_role(
                unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_tx_free: *mut _)) },
                unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_tx_used: *mut _)) },
                notify_net,
            );

        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1500;
        caps.medium = Medium::Ethernet;

        let mut device = DeviceImpl::new(
            Default::default(),
            dma_region,
            bounce_buffer_allocator,
            rx_ring_buffers,
            tx_ring_buffers,
            config::NET_QUEUE_SIZE,
            config::NET_BUFFER_LEN,
            caps,
        )
        .expect("virtio-net device");

        let hardware_addr = HardwareAddress::Ethernet(EthernetAddress(mac.0));
        let mut iface = Interface::new(Config::new(hardware_addr), &mut device, Instant::ZERO);
        iface.update_ip_addrs(|ip_addrs| {
            ip_addrs
                .push(IpCidr::new(IpAddress::Ipv4(GUEST_IP), 24))
                .expect("guest IPv4 address");
        });
        iface
            .routes_mut()
            .add_default_ipv4_route(HOST_IP)
            .expect("default route");

        Self {
            device,
            iface,
            udp_tx_done: false,
            udp_tx_logged: false,
            tcp_client_sent: false,
            tcp_rx_done: false,
            done: false,
        }
    }

    fn init_sockets(iface: &mut Interface) {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        if arena.initialized {
            return;
        }

        let mut sockets = SocketSet::new(&mut arena.storage[..]);

        let udp_socket = UdpSocket::new(
            PacketBuffer::new(&mut arena.udp_rx_meta[..], &mut arena.udp_rx_payload[..]),
            PacketBuffer::new(&mut arena.udp_tx_meta[..], &mut arena.udp_tx_payload[..]),
        );
        arena.udp_handle = Some(sockets.add(udp_socket));

        let mut tcp_cli = TcpSocket::new(
            TcpSocketBuffer::new(&mut arena.tcp_cli_rx[..]),
            TcpSocketBuffer::new(&mut arena.tcp_cli_tx[..]),
        );
        let remote = IpEndpoint::new(IpAddress::Ipv4(HOST_IP), TCP_ECHO_PORT);
        let local = IpListenEndpoint::from((IpAddress::Ipv4(GUEST_IP), 49152));
        tcp_cli
            .connect(iface.context(), remote, local)
            .expect("tcp connect");
        arena.tcp_cli_handle = Some(sockets.add(tcp_cli));

        arena.initialized = true;
    }

    pub fn poll(&mut self) {
        if self.done {
            return;
        }

        self.device.poll();
        Self::init_sockets(&mut self.iface);

        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let mut sockets = SocketSet::new(&mut arena.storage[..]);

        if !self.udp_tx_done {
            let local = IpListenEndpoint::from((IpAddress::Ipv4(GUEST_IP), 4242));
            let remote = IpEndpoint::new(IpAddress::Ipv4(HOST_IP), 12345);
            let udp = sockets.get_mut::<UdpSocket>(arena.udp_handle.unwrap());
            if udp.bind(local).is_ok() && udp.send_slice(b"lerux-net", remote).is_ok() {
                self.udp_tx_done = true;
            }
        }

        let tcp_cli = sockets.get_mut::<TcpSocket>(arena.tcp_cli_handle.unwrap());
        if !self.tcp_client_sent && tcp_cli.may_send() {
            if tcp_cli.send_slice(b"lerux-tcp").is_ok() {
                self.tcp_client_sent = true;
            }
        }
        if !self.tcp_rx_done && tcp_cli.may_recv() {
            let mut buf = [0u8; 16];
            if let Ok(len) = tcp_cli.recv_slice(&mut buf) {
                if len >= 9 && &buf[..9] == b"lerux-tcp" {
                    log::info!("virtio-net: TCP RX ok");
                    self.tcp_rx_done = true;
                }
            }
        }

        self.iface
            .poll(Instant::ZERO, &mut self.device, &mut sockets);

        if self.udp_tx_done && !self.udp_tx_logged {
            log::info!("virtio-net: TX ok");
            self.udp_tx_logged = true;
        }

        if self.udp_tx_done && self.tcp_rx_done {
            self.done = true;
        }
    }

    pub fn is_done(&self) -> bool {
        self.done
    }
}