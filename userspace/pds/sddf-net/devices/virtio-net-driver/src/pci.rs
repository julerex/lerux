use lerux_virtio_hal::HalImpl;
use lerux_virtio_pci::{create_pci_transport_ioport, program_device_bars_ioport};
use sel4_microkit::var;
use virtio_drivers::{
    device::net::VirtIONet,
    transport::{
        pci::{bus::DeviceFunction, PciTransport},
        DeviceType, Transport,
    },
};

use crate::config;

const NET_QUEUE_SIZE: usize = 16;
const NET_BUFFER_LEN: usize = 2048;

pub fn init_hal() {
    HalImpl::init(
        config::VIRTIO_NET_DRIVER_DMA_SIZE,
        *var!(virtio_net_driver_dma_vaddr: usize = 0),
        *var!(virtio_net_driver_dma_paddr: usize = 0),
        config::pci::NET_BAR_REGIONS,
    );
}

pub fn device_function() -> DeviceFunction {
    config::pci::NET_DEVICE
}

pub fn create_virtio_net() -> VirtIONet<HalImpl, PciTransport, NET_QUEUE_SIZE> {
    let ioport_id = *var!(pci_config_ioport_id: usize = 0) as u32;
    let ioport_addr = *var!(pci_config_ioport_addr: usize = 0) as u16;
    #[cfg(feature = "board-x86_64_generic_virtio")]
    program_device_bars_ioport(
        ioport_id,
        ioport_addr,
        config::pci::BLK_DEVICE,
        config::pci::BLK_BAR_PADDRS,
    );
    let transport = create_pci_transport_ioport(
        ioport_id,
        ioport_addr,
        device_function(),
        config::pci::NET_BAR_PADDRS,
    )
    .expect("virtio-net PCI transport");
    assert_eq!(transport.device_type(), DeviceType::Network);
    VirtIONet::<HalImpl, PciTransport, NET_QUEUE_SIZE>::new(transport, NET_BUFFER_LEN).unwrap()
}
