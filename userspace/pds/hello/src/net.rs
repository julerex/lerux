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
const GUEST_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 15);
const HOST_IP: Ipv4Address = Ipv4Address::new(10, 0, 2, 2);
const TCP_ECHO_PORT: u16 = 18080;

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
        .expect("hello net bounce");
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

impl NetIo {
    pub fn new(mac: MacAddress) -> Self {
        let notify_net: fn() = || NET_DRIVER.notify();
        let dma_region = create_dma_region();
        let (rx_ring_buffers, tx_ring_buffers) = create_net_ring_buffers(notify_net);
        let mut device = create_net_device(dma_region, rx_ring_buffers, tx_ring_buffers);
        let iface = configure_iface(&mut device, mac);
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

    fn init_udp_socket(sockets: &mut SocketSet<'static>) -> SocketHandle {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let udp_socket = UdpSocket::new(
            PacketBuffer::new(&mut arena.udp_rx_meta[..], &mut arena.udp_rx_payload[..]),
            PacketBuffer::new(&mut arena.udp_tx_meta[..], &mut arena.udp_tx_payload[..]),
        );
        sockets.add(udp_socket)
    }

    fn init_tcp_client_socket(
        sockets: &mut SocketSet<'static>,
        iface: &mut Interface,
    ) -> SocketHandle {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let mut tcp_cli = TcpSocket::new(
            TcpSocketBuffer::new(&mut arena.tcp_cli_rx[..]),
            TcpSocketBuffer::new(&mut arena.tcp_cli_tx[..]),
        );
        let remote = IpEndpoint::new(IpAddress::Ipv4(HOST_IP), TCP_ECHO_PORT);
        let local = IpListenEndpoint::from((IpAddress::Ipv4(GUEST_IP), 49152));
        tcp_cli
            .connect(iface.context(), remote, local)
            .expect("tcp connect");
        sockets.add(tcp_cli)
    }

    fn init_sockets(iface: &mut Interface) {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        if arena.initialized {
            return;
        }
        let mut sockets = SocketSet::new(&mut arena.storage[..]);
        arena.udp_handle = Some(Self::init_udp_socket(&mut sockets));
        arena.tcp_cli_handle = Some(Self::init_tcp_client_socket(&mut sockets, iface));
        arena.initialized = true;
    }

    fn poll_udp_tx(&mut self, sockets: &mut SocketSet<'static>) {
        if self.udp_tx_done {
            return;
        }
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let local = IpListenEndpoint::from((IpAddress::Ipv4(GUEST_IP), 4242));
        let remote = IpEndpoint::new(IpAddress::Ipv4(HOST_IP), 12345);
        let udp = sockets.get_mut::<UdpSocket>(arena.udp_handle.unwrap());
        if udp.bind(local).is_ok() && udp.send_slice(b"lerux-net", remote).is_ok() {
            self.udp_tx_done = true;
        }
    }

    fn poll_tcp_client(&mut self, sockets: &mut SocketSet<'static>) {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let tcp_cli = sockets.get_mut::<TcpSocket>(arena.tcp_cli_handle.unwrap());
        if !self.tcp_client_sent && tcp_cli.may_send() && tcp_cli.send_slice(b"lerux-tcp").is_ok() {
            self.tcp_client_sent = true;
        }
        if !self.tcp_rx_done && tcp_cli.may_recv() {
            let mut buf = [0u8; 16];
            if let Ok(len) = tcp_cli.recv_slice(&mut buf)
                && len >= 9
                && &buf[..9] == b"lerux-tcp"
            {
                log::info!("virtio-net: TCP RX ok");
                self.tcp_rx_done = true;
            }
        }
    }

    fn update_completion_state(&mut self) {
        if self.udp_tx_done && !self.udp_tx_logged {
            log::info!("virtio-net: TX ok");
            self.udp_tx_logged = true;
        }
        if self.udp_tx_done && self.tcp_rx_done {
            self.done = true;
        }
    }

    pub fn poll(&mut self) {
        if self.done {
            return;
        }
        self.device.poll();
        Self::init_sockets(&mut self.iface);
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let mut sockets = SocketSet::new(&mut arena.storage[..]);
        self.poll_udp_tx(&mut sockets);
        self.poll_tcp_client(&mut sockets);
        self.iface
            .poll(Instant::ZERO, &mut self.device, &mut sockets);
        self.update_completion_state();
    }

    pub fn is_done(&self) -> bool {
        self.done
    }
}
