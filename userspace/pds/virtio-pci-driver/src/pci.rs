use lerux_virtio_hal::HalImpl;
use lerux_virtio_pci::create_pci_transport_ioport;
use sel4_microkit::var;
use virtio_drivers::{
    device::{blk::VirtIOBlk, net::VirtIONet},
    transport::pci::PciTransport,
    transport::{DeviceType, Transport},
};

use crate::config;

const NET_QUEUE_SIZE: usize = 16;
const NET_BUFFER_LEN: usize = 2048;

pub fn init_hal() {
    HalImpl::init(
        config::VIRTIO_DRIVER_DMA_SIZE,
        *var!(virtio_blk_driver_dma_vaddr: usize = 0),
        *var!(virtio_blk_driver_dma_paddr: usize = 0),
        config::pci::BAR_REGIONS,
    );
}

pub fn ioport_config() -> (u32, u16) {
    (
        *var!(pci_config_ioport_id: usize = 0) as u32,
        *var!(pci_config_ioport_addr: usize = 0) as u16,
    )
}

pub fn create_virtio_blk(ioport_id: u32, ioport_addr: u16) -> VirtIOBlk<HalImpl, PciTransport> {
    let transport = create_pci_transport_ioport(
        ioport_id,
        ioport_addr,
        config::pci::BLK_DEVICE,
        config::pci::BLK_BAR_PADDRS,
    )
    .expect("virtio-blk PCI transport");
    assert_eq!(transport.device_type(), DeviceType::Block);
    VirtIOBlk::<HalImpl, PciTransport>::new(transport).unwrap()
}

pub fn create_virtio_net(ioport_id: u32, ioport_addr: u16) -> VirtIONet<HalImpl, PciTransport, NET_QUEUE_SIZE> {
    let transport = create_pci_transport_ioport(
        ioport_id,
        ioport_addr,
        config::pci::NET_DEVICE,
        config::pci::NET_BAR_PADDRS,
    )
    .expect("virtio-net PCI transport");
    assert_eq!(transport.device_type(), DeviceType::Network);
    VirtIONet::<HalImpl, PciTransport, NET_QUEUE_SIZE>::new(transport, NET_BUFFER_LEN).unwrap()
}