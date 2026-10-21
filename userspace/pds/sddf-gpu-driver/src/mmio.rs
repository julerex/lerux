use core::ptr::NonNull;

use sel4_microkit::var;
use sel4_virtio_hal_impl::HalImpl;
use virtio_drivers::{
    device::gpu::VirtIOGpu,
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        DeviceType, Transport,
    },
};

use lerux_machine::{virtio, Arch, VirtioAttach, VirtioRole};
use lerux_sddf::gpu_image::{GPU_DRIVER_DMA_SIZE, GPU_VIRTIO_MMIO_SIZE};

const MMIO_INIT_ATTEMPTS: usize = 10_000;

pub type GpuDev = VirtIOGpu<HalImpl, MmioTransport<'static>>;

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
    unsafe { MmioTransport::new(header, mmio_size) }.expect("virtio-gpu MMIO transport")
}

pub fn init_hal() {
    HalImpl::init(
        usize::try_from(GPU_DRIVER_DMA_SIZE).expect("dma size fits"),
        *var!(virtio_gpu_driver_dma_vaddr: usize = 0),
        *var!(virtio_gpu_driver_dma_paddr: usize = 0),
    );
}

fn gpu_mmio_offset() -> usize {
    let attach = virtio(Arch::TARGET, VirtioRole::Gpu).expect("browser image includes virtio-gpu");
    match attach {
        VirtioAttach::Mmio { offset } => usize::try_from(offset).expect("offset fits"),
        VirtioAttach::Pci { .. } => panic!("virtio-gpu is memory-mapped on this board"),
    }
}

pub fn create_virtio_gpu() -> GpuDev {
    let header = NonNull::new(
        (*var!(virtio_gpu_mmio_vaddr: usize = 0) + gpu_mmio_offset()) as *mut VirtIOHeader,
    )
    .expect("virtio-gpu mmio vaddr");
    let transport = wait_for_mmio_transport(header, GPU_VIRTIO_MMIO_SIZE);
    assert_eq!(transport.device_type(), DeviceType::GPU);
    VirtIOGpu::<HalImpl, MmioTransport>::new(transport).expect("virtio-gpu device")
}
