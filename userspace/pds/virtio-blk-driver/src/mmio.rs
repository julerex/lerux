use core::ptr::NonNull;

use sel4_microkit::var;
use sel4_virtio_hal_impl::HalImpl;
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        DeviceType, Transport,
    },
};

use crate::config;

pub fn init_hal() {
    HalImpl::init(
        config::VIRTIO_BLK_DRIVER_DMA_SIZE,
        *var!(virtio_blk_driver_dma_vaddr: usize = 0),
        *var!(virtio_blk_driver_dma_paddr: usize = 0),
    );
}

pub fn create_virtio_blk() -> VirtIOBlk<HalImpl, MmioTransport<'static>> {
    let header = NonNull::new(
        (*var!(virtio_blk_mmio_vaddr: usize = 0) + config::VIRTIO_BLK_MMIO_OFFSET)
            as *mut VirtIOHeader,
    )
    .unwrap();
    let transport = unsafe { MmioTransport::new(header, config::VIRTIO_BLK_MMIO_SIZE) }.unwrap();
    assert_eq!(transport.device_type(), DeviceType::Block);
    VirtIOBlk::<HalImpl, MmioTransport>::new(transport).unwrap()
}
