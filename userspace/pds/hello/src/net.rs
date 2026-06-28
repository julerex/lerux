use lerux_logging::log;
use sel4_abstract_allocator::WithAlignmentBound;
use sel4_abstract_allocator::basic::BasicAllocator;
use sel4_driver_interfaces::net::MacAddress;
use sel4_microkit::memory_region_symbol;
use sel4_microkit::Channel;
use sel4_shared_memory::SharedMemoryRef;
use sel4_shared_ring_buffer::RingBuffers;
use sel4_shared_ring_buffer_smoltcp::DeviceImpl;
use smoltcp::iface::{Config, Interface, SocketSet, SocketStorage};
use smoltcp::phy::{DeviceCapabilities, Medium};
use smoltcp::socket::udp::{PacketBuffer, PacketMetadata, Socket as UdpSocket};
use smoltcp::time::Instant;
use smoltcp::wire::{
    EthernetAddress, HardwareAddress, IpAddress, IpCidr, IpEndpoint, IpListenEndpoint, Ipv4Address,
};

use crate::config;

const NET_DRIVER: Channel = Channel::new(1);

pub struct NetTx {
    device: DeviceImpl<WithAlignmentBound<BasicAllocator>>,
    iface: Interface,
    send_attempted: bool,
    done: bool,
}

impl NetTx {
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
                .push(IpCidr::new(
                    IpAddress::Ipv4(Ipv4Address::new(10, 0, 2, 15)),
                    24,
                ))
                .expect("guest IPv4 address");
        });

        Self {
            device,
            iface,
            send_attempted: false,
            done: false,
        }
    }

    pub fn poll(&mut self) {
        if self.done {
            return;
        }

        self.device.poll();

        let mut socket_storage = [SocketStorage::EMPTY];
        let mut udp_rx_meta = [PacketMetadata::EMPTY];
        let mut udp_rx_payload = [0u8; 128];
        let mut udp_tx_meta = [PacketMetadata::EMPTY];
        let mut udp_tx_payload = [0u8; 128];
        let mut sockets = SocketSet::new(&mut socket_storage[..]);
        let udp_socket = UdpSocket::new(
            PacketBuffer::new(&mut udp_rx_meta[..], &mut udp_rx_payload[..]),
            PacketBuffer::new(&mut udp_tx_meta[..], &mut udp_tx_payload[..]),
        );
        let handle = sockets.add(udp_socket);

        if !self.send_attempted {
            let local = IpListenEndpoint::from((
                IpAddress::Ipv4(Ipv4Address::new(10, 0, 2, 15)),
                4242,
            ));
            let remote = IpEndpoint::new(IpAddress::Ipv4(Ipv4Address::new(10, 0, 2, 2)), 12345);
            let socket = sockets.get_mut::<UdpSocket>(handle);
            if socket.bind(local).is_ok() && socket.send_slice(b"lerux-net", remote).is_ok() {
                self.send_attempted = true;
            }
        }

        self.iface
            .poll(Instant::ZERO, &mut self.device, &mut sockets);

        if self.send_attempted {
            log::info!("virtio-net: TX ok");
            self.done = true;
        }
    }

    pub fn is_done(&self) -> bool {
        self.done
    }
}