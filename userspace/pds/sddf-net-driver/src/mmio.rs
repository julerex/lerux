use core::ptr::NonNull;

use sel4_microkit::var;
use sel4_virtio_hal_impl::HalImpl;
use virtio_drivers::{
    device::net::VirtIONet,
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        DeviceType, Transport,
    },
};

use lerux_sddf::net_image::{NET_DRIVER_DMA_SIZE, NET_VIRTIO_MMIO_OFFSET, NET_VIRTIO_MMIO_SIZE};

/// Virtio ring size. Sixteen buffers match the shared queue capacity.
pub const QUEUE_SIZE: usize = 16;

/// Bytes reserved for one virtio receive buffer, including the virtio header.
/// The ethernet frame that follows the header fits in a 2048-byte shared buffer.
pub const BUFFER_LEN: usize = 2048;

const MMIO_INIT_ATTEMPTS: usize = 10_000;

pub type NetDev = VirtIONet<HalImpl, MmioTransport<'static>, QUEUE_SIZE>;

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
    unsafe { MmioTransport::new(header, mmio_size) }.expect("virtio-net MMIO transport")
}

pub fn init_hal() {
    HalImpl::init(
        usize::try_from(NET_DRIVER_DMA_SIZE).expect("dma size fits"),
        *var!(virtio_net_driver_dma_vaddr: usize = 0),
        *var!(virtio_net_driver_dma_paddr: usize = 0),
    );
}

pub fn create_virtio_net() -> NetDev {
    let header = NonNull::new(
        (*var!(virtio_net_mmio_vaddr: usize = 0) + NET_VIRTIO_MMIO_OFFSET) as *mut VirtIOHeader,
    )
    .expect("virtio-net mmio vaddr");
    let transport = wait_for_mmio_transport(header, NET_VIRTIO_MMIO_SIZE);
    assert_eq!(transport.device_type(), DeviceType::Network);
    VirtIONet::<HalImpl, MmioTransport, QUEUE_SIZE>::new(transport, BUFFER_LEN)
        .expect("virtio-net device")
}
