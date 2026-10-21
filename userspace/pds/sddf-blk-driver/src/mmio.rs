#[cfg(not(target_arch = "x86_64"))]
use core::ptr::NonNull;

#[cfg(not(target_arch = "x86_64"))]
use sel4_microkit::var;
#[cfg(not(target_arch = "x86_64"))]
use sel4_virtio_hal_impl::HalImpl;
#[cfg(not(target_arch = "x86_64"))]
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        DeviceType, Transport,
    },
};

#[cfg(not(target_arch = "x86_64"))]
use lerux_machine::{virtio, Arch, VirtioRole};
#[cfg(not(target_arch = "x86_64"))]
use lerux_sddf::blk_image::BLK_VIRTIO_DMA_SIZE;

#[cfg(not(target_arch = "x86_64"))]
pub const VIRTIO_BLK_MMIO_SIZE: usize = 0x200;

#[cfg(not(target_arch = "x86_64"))]
const fn blk_mmio_offset() -> usize {
    match virtio(Arch::TARGET, VirtioRole::Blk) {
        Some(lerux_machine::VirtioAttach::Mmio { offset }) => offset as usize,
        _ => 0,
    }
}

#[cfg(not(target_arch = "x86_64"))]
const MMIO_INIT_ATTEMPTS: usize = 10_000;

#[cfg(not(target_arch = "x86_64"))]
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

#[cfg(not(target_arch = "x86_64"))]
pub fn init_hal() {
    HalImpl::init(
        usize::try_from(BLK_VIRTIO_DMA_SIZE).expect("dma size fits"),
        *var!(virtio_blk_driver_dma_vaddr: usize = 0),
        *var!(virtio_blk_driver_dma_paddr: usize = 0),
    );
}

#[cfg(not(target_arch = "x86_64"))]
pub type BlkDev = VirtIOBlk<HalImpl, MmioTransport<'static>>;

#[cfg(not(target_arch = "x86_64"))]
pub fn create_virtio_blk() -> BlkDev {
    let header = NonNull::new(
        (*var!(virtio_blk_mmio_vaddr: usize = 0) + blk_mmio_offset()) as *mut VirtIOHeader,
    )
    .expect("virtio-blk mmio vaddr");
    let transport = wait_for_mmio_transport(header, VIRTIO_BLK_MMIO_SIZE);
    assert_eq!(transport.device_type(), DeviceType::Block);
    VirtIOBlk::<HalImpl, MmioTransport>::new(transport).expect("virtio-blk device")
}
