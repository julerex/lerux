use lerux_virtio_hal::HalImpl;
use lerux_virtio_pci::create_pci_transport_ecam_bars_programmed;
use sel4_microkit::var;
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::{
        pci::{bus::DeviceFunction, PciTransport},
        DeviceType, Transport,
    },
};

use crate::config;

pub fn init_hal() {
    HalImpl::init(
        config::VIRTIO_BLK_DRIVER_DMA_SIZE,
        *var!(virtio_blk_driver_dma_vaddr: usize = 0),
        *var!(virtio_blk_driver_dma_paddr: usize = 0),
        config::pci::BLK_BAR_REGIONS,
    );
}

pub fn device_function() -> DeviceFunction {
    config::pci::BLK_DEVICE
}

pub fn create_virtio_blk() -> VirtIOBlk<HalImpl, PciTransport> {
    let transport = create_pci_transport_ecam_bars_programmed(
        *var!(pci_ecam_vaddr: usize = 0),
        device_function(),
    )
    .expect("virtio-blk PCI transport");
    assert_eq!(transport.device_type(), DeviceType::Block);
    VirtIOBlk::<HalImpl, PciTransport>::new(transport).unwrap()
}
