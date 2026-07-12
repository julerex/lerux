use core::ptr::NonNull;

use sel4_microkit::var;
use virtio_drivers::{
    device::net::*,
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        DeviceType, Transport,
    },
};

use crate::config;

const NET_QUEUE_SIZE: usize = 16;
const NET_BUFFER_LEN: usize = 2048;

#[cfg(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_http"
))]
type NetHal = lerux_virtio_hal::HalImpl;

#[cfg(not(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_http"
)))]
type NetHal = sel4_virtio_hal_impl::HalImpl;

/// Full-size Hal init when DMA is not unified (riscv / legacy maps).
/// Under `unified-dma`, Hal is initialised in `dma::init_hal_unified` so that
/// `var!(virtio_net_driver_dma_*)` is defined only once in the crate.
#[cfg(not(feature = "unified-dma"))]
pub fn init_hal() {
    #[cfg(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http"
    ))]
    NetHal::init(
        config::VIRTIO_NET_DRIVER_DMA_SIZE,
        *var!(virtio_net_driver_dma_vaddr: usize = 0),
        *var!(virtio_net_driver_dma_paddr: usize = 0),
        &[],
    );
    #[cfg(not(any(
        feature = "board-x86_64_generic_virtio",
        feature = "board-x86_64_generic_http"
    )))]
    NetHal::init(
        config::VIRTIO_NET_DRIVER_DMA_SIZE,
        *var!(virtio_net_driver_dma_vaddr: usize = 0),
        *var!(virtio_net_driver_dma_paddr: usize = 0),
    );
}

pub fn create_virtio_net() -> VirtIONet<NetHal, MmioTransport<'static>, NET_QUEUE_SIZE> {
    let header = NonNull::new(
        (*var!(virtio_net_mmio_vaddr: usize = 0) + config::VIRTIO_NET_MMIO_OFFSET)
            as *mut VirtIOHeader,
    )
    .unwrap();
    let transport = unsafe { MmioTransport::new(header, config::VIRTIO_NET_MMIO_SIZE) }.unwrap();
    assert_eq!(transport.device_type(), DeviceType::Network);
    VirtIONet::<NetHal, MmioTransport, NET_QUEUE_SIZE>::new(transport, NET_BUFFER_LEN).unwrap()
}
