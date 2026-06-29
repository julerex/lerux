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

const HTTP_RESPONSE: &[u8] =
    b"HTTP/1.1 200 OK\r\nContent-Length: 14\r\nConnection: close\r\n\r\nlerux: HTTP ok";

struct SocketArena {
    storage: [SocketStorage<'static>; 2],
    udp_rx_meta: [PacketMetadata; 1],
    udp_rx_payload: [u8; 128],
    udp_tx_meta: [PacketMetadata; 1],
    udp_tx_payload: [u8; 128],
    tcp_rx: [u8; 1024],
    tcp_tx: [u8; 2048],
    udp_handle: Option<SocketHandle>,
    tcp_handle: Option<SocketHandle>,
    initialized: bool,
    udp_primed: bool,
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
            tcp_tx: [0; 2048],
            udp_handle: None,
            tcp_handle: None,
            initialized: false,
            udp_primed: false,
        }
    }
}

static mut SOCKET_ARENA: SocketArena = SocketArena::empty();

pub struct HttpNet {
    device: DeviceImpl<WithAlignmentBound<BasicAllocator>>,
    iface: Interface,
    listening_logged: bool,
    served: bool,
}

fn create_dma_region() -> SharedMemoryRef<'static, [u8]> {
    unsafe {
        SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
            virtio_net_client_dma_vaddr: *mut [u8],
            n = config::VIRTIO_NET_CLIENT_DMA_SIZE
        ))
    }
}

fn create_net_ring_buffers(
    notify_net: fn(),
) -> (
    RingBuffers<'static, sel4_shared_ring_buffer::roles::Provide, fn()>,
    RingBuffers<'static, sel4_shared_ring_buffer::roles::Provide, fn()>,
) {
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

impl HttpNet {
    pub fn new(mac: MacAddress) -> Self {
        let notify_net: fn() = || NET_DRIVER.notify();
        let dma_region = create_dma_region();
        let (rx_ring_buffers, tx_ring_buffers) = create_net_ring_buffers(notify_net);
        let mut device = create_net_device(dma_region, rx_ring_buffers, tx_ring_buffers);
        let iface = configure_iface(&mut device, mac);
        Self {
            device,
            iface,
            listening_logged: false,
            served: false,
        }
    }

    fn init_udp_socket(sockets: &mut SocketSet<'static>) -> SocketHandle {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let udp = UdpSocket::new(
            PacketBuffer::new(&mut arena.udp_rx_meta[..], &mut arena.udp_rx_payload[..]),
            PacketBuffer::new(&mut arena.udp_tx_meta[..], &mut arena.udp_tx_payload[..]),
        );
        sockets.add(udp)
    }

    fn prime_udp_tx(&mut self, sockets: &mut SocketSet<'static>) {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        if arena.udp_primed {
            return;
        }
        let Some(udp_handle) = arena.udp_handle else {
            return;
        };
        let local = IpListenEndpoint::from((IpAddress::Ipv4(GUEST_IP), 4242));
        let remote = IpEndpoint::new(IpAddress::Ipv4(HOST_IP), 12345);
        let udp = sockets.get_mut::<UdpSocket>(udp_handle);
        if udp.bind(local).is_ok() && udp.send_slice(b"lerux-http", remote).is_ok() {
            arena.udp_primed = true;
        }
    }

    fn init_http_socket(sockets: &mut SocketSet<'static>) -> Option<SocketHandle> {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let mut tcp = TcpSocket::new(
            TcpSocketBuffer::new(&mut arena.tcp_rx[..]),
            TcpSocketBuffer::new(&mut arena.tcp_tx[..]),
        );
        let endpoint = IpListenEndpoint {
            addr: None,
            port: config::HTTP_PORT,
        };
        if tcp.listen(endpoint).is_ok() {
            Some(sockets.add(tcp))
        } else {
            None
        }
    }

    fn ensure_listening_socket(&mut self, sockets: &mut SocketSet<'static>) {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        if !arena.initialized {
            arena.udp_handle = Some(Self::init_udp_socket(sockets));
            if let Some(handle) = Self::init_http_socket(sockets) {
                arena.tcp_handle = Some(handle);
                arena.initialized = true;
            }
            return;
        }
        let tcp = sockets.get_mut::<TcpSocket>(arena.tcp_handle.unwrap());
        if !tcp.is_open() {
            let endpoint = IpListenEndpoint {
                addr: None,
                port: config::HTTP_PORT,
            };
            let _ = tcp.listen(endpoint);
        }
    }

    fn log_listening(&mut self) {
        if !self.listening_logged {
            log::info!("lerux-http: listening on :{}", config::HTTP_PORT);
            self.listening_logged = true;
        }
    }

    fn serve_http_request(&mut self, sockets: &mut SocketSet<'static>) {
        let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
        let Some(handle) = arena.tcp_handle else {
            return;
        };
        let tcp = sockets.get_mut::<TcpSocket>(handle);
        if !tcp.may_recv() {
            return;
        }
        let mut buf = [0u8; 256];
        if let Ok(len) = tcp.recv_slice(&mut buf) {
            if len >= 4 && &buf[..4] == b"GET " && tcp.may_send() {
                let _ = tcp.send_slice(HTTP_RESPONSE);
                log::info!("lerux-http: served GET /");
                self.served = true;
            }
        }
    }

    pub fn poll(&mut self) {
        loop {
            self.device.poll();
            let arena = unsafe { &mut *core::ptr::addr_of_mut!(SOCKET_ARENA) };
            let mut sockets = SocketSet::new(&mut arena.storage[..]);
            self.ensure_listening_socket(&mut sockets);
            if arena.initialized {
                self.log_listening();
                self.prime_udp_tx(&mut sockets);
                if !self.served {
                    self.serve_http_request(&mut sockets);
                }
            }
            self.iface
                .poll(Instant::ZERO, &mut self.device, &mut sockets);
            if self.served {
                break;
            }
            if !self.device.poll() {
                break;
            }
        }
    }

    pub fn is_served(&self) -> bool {
        self.served
    }
}