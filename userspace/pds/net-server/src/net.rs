use lerux_interface_types::MAX_NET_UDP_PAYLOAD;
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
    socket::udp::{PacketBuffer, PacketMetadata, Socket as UdpSocket},
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
const LOCAL_UDP_PORT: u16 = 4242;
const REMOTE_UDP_PORT: u16 = 12345;

type NetRingBuffers = (
    RingBuffers<'static, sel4_shared_ring_buffer::roles::Provide, fn()>,
    RingBuffers<'static, sel4_shared_ring_buffer::roles::Provide, fn()>,
);

struct SocketArena {
    storage: [SocketStorage<'static>; 1],
    udp_rx_meta: [PacketMetadata; 1],
    udp_rx_payload: [u8; 128],
    udp_tx_meta: [PacketMetadata; 1],
    udp_tx_payload: [u8; 128],
    udp_handle: Option<SocketHandle>,
    initialized: bool,
}

impl SocketArena {
    const fn empty() -> Self {
        Self {
            storage: [SocketStorage::EMPTY],
            udp_rx_meta: [PacketMetadata::EMPTY],
            udp_rx_payload: [0; 128],
            udp_tx_meta: [PacketMetadata::EMPTY],
            udp_tx_payload: [0; 128],
            udp_handle: None,
            initialized: false,
        }
    }
}

static mut SOCKET_ARENA: SocketArena = SocketArena::empty();

pub struct NetStack {
    device: DeviceImpl<WithAlignmentBound<BasicAllocator>>,
    iface: Interface,
    pending_payload_len: Option<u8>,
    pending_payload: [u8; MAX_NET_UDP_PAYLOAD],
    tx_done: bool,
    tx_logged: bool,
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
            pending_payload_len: None,
            pending_payload: [0; MAX_NET_UDP_PAYLOAD],
            tx_done: false,
            tx_logged: false,
        }
    }

    pub fn queue_udp_tx(&mut self, payload_len: u8, payload: [u8; MAX_NET_UDP_PAYLOAD]) {
        self.pending_payload = payload;
        self.pending_payload_len = Some(payload_len);
        self.tx_done = false;
        self.tx_logged = false;
    }

    pub fn is_tx_done(&self) -> bool {
        self.tx_done
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
        if arena.initialized {
            return;
        }
        arena.udp_handle = Some(Self::init_udp_socket(sockets));
        arena.initialized = true;
    }

    fn try_udp_tx(&mut self, sockets: &mut SocketSet<'static>) {
        if self.tx_done {
            return;
        }
        let Some(payload_len) = self.pending_payload_len else {
            return;
        };
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let Some(udp_handle) = arena.udp_handle else {
            return;
        };
        let local = IpListenEndpoint::from((IpAddress::Ipv4(GUEST_IP), LOCAL_UDP_PORT));
        let remote = IpEndpoint::new(IpAddress::Ipv4(HOST_IP), REMOTE_UDP_PORT);
        let udp = sockets.get_mut::<UdpSocket>(udp_handle);
        let payload = &self.pending_payload[..payload_len as usize];
        if udp.bind(local).is_ok() && udp.send_slice(payload, remote).is_ok() {
            self.tx_done = true;
        }
    }

    fn log_tx_done(&mut self) {
        if self.tx_done && !self.tx_logged {
            log::info!("lerux-net: TX ok");
            self.tx_logged = true;
        }
    }

    pub fn poll(&mut self) {
        self.device.poll();
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let mut sockets = SocketSet::new(&mut arena.storage[..]);
        self.ensure_udp_socket(&mut sockets);
        self.try_udp_tx(&mut sockets);
        self.iface
            .poll(Instant::ZERO, &mut self.device, &mut sockets);
        self.log_tx_done();
    }
}
