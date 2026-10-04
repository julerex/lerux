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

use lerux_sddf::blk_image::BLK_VIRTIO_DMA_SIZE;

/// QEMU `virt` puts virtio-blk at `+0xc00` when a virtio-net device occupies the first slot.
pub const VIRTIO_BLK_MMIO_OFFSET: usize = 0xc00;
pub const VIRTIO_BLK_MMIO_SIZE: usize = 0x200;

const MMIO_INIT_ATTEMPTS: usize = 10_000;

fn wait_for_mmio_transport(
    header: NonNull<VirtIOHeader>,
    mmio_size: usize,
) -> MmioTransport<'static> {
    for _ in 0..MMIO_INIT_ATTEMPTS {
        // SAFETY: `header` points at the board-mapped virtio-mmio region.
        if let Ok(transport) = unsafe { MmioTransport::new(header, mmio_size) } {
            return transport;
        }
        core::hint::spin_loop();
    }
    // SAFETY: same region as the loop above.
    unsafe { MmioTransport::new(header, mmio_size) }.expect("virtio-blk MMIO transport")
}

pub fn init_hal() {
    HalImpl::init(
        usize::try_from(BLK_VIRTIO_DMA_SIZE).expect("dma size fits"),
        *var!(virtio_blk_driver_dma_vaddr: usize = 0),
        *var!(virtio_blk_driver_dma_paddr: usize = 0),
    );
}

pub fn create_virtio_blk() -> VirtIOBlk<HalImpl, MmioTransport<'static>> {
    let header = NonNull::new(
        (*var!(virtio_blk_mmio_vaddr: usize = 0) + VIRTIO_BLK_MMIO_OFFSET) as *mut VirtIOHeader,
    )
    .expect("virtio-blk mmio vaddr");
    let transport = wait_for_mmio_transport(header, VIRTIO_BLK_MMIO_SIZE);
    assert_eq!(transport.device_type(), DeviceType::Block);
    VirtIOBlk::<HalImpl, MmioTransport>::new(transport).expect("virtio-blk device")
}
