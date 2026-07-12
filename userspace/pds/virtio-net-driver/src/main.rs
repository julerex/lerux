#![no_std]
#![no_main]

use lerux_logging::{debug, log};
use sel4_microkit::{memory_region_symbol, protection_domain};
use sel4_microkit_driver_adapters::net::driver::HandlerImpl;
use sel4_shared_memory::SharedMemoryRef;
use sel4_shared_ring_buffer::{roles::Use, RingBuffers};
use sel4_virtio_net::DeviceWrapper;

mod config;

#[cfg(feature = "unified-dma")]
mod dma;

#[cfg(not(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_http"
)))]
mod mmio;

#[cfg(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_http"
))]
mod pci;

use config::channels;

#[cfg(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_http"
))]
type DriverHal = lerux_virtio_hal::HalImpl;

#[cfg(not(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_http"
)))]
type DriverHal = sel4_virtio_hal_impl::HalImpl;

#[cfg(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_http"
))]
type NetTransport = virtio_drivers::transport::pci::PciTransport;

#[cfg(not(any(
    feature = "board-x86_64_generic_virtio",
    feature = "board-x86_64_generic_http"
)))]
type NetTransport = virtio_drivers::transport::mmio::MmioTransport<'static>;

type NetRingBuffers = (
    RingBuffers<'static, Use, fn()>,
    RingBuffers<'static, Use, fn()>,
);

#[cfg(not(feature = "unified-dma"))]
fn create_client_region() -> SharedMemoryRef<'static, [u8]> {
    unsafe {
        SharedMemoryRef::<'static, _>::new(memory_region_symbol!(
            virtio_net_client_dma_vaddr: *mut [u8],
            n = config::VIRTIO_NET_CLIENT_DMA_SIZE
        ))
    }
}

#[cfg(feature = "unified-dma")]
fn create_client_region() -> SharedMemoryRef<'static, [u8]> {
    // High half of driver_dma — no separate client_dma map (Phase 43).
    dma::bounce_region()
}

fn create_net_ring_buffers(notify_client: fn()) -> NetRingBuffers {
    let rx_ring_buffers =
        RingBuffers::<'_, Use, fn()>::from_ptrs_using_default_initialization_strategy_for_role(
            unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_rx_free: *mut _)) },
            unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_rx_used: *mut _)) },
            notify_client,
        );
    let tx_ring_buffers =
        RingBuffers::<'_, Use, fn()>::from_ptrs_using_default_initialization_strategy_for_role(
            unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_tx_free: *mut _)) },
            unsafe { SharedMemoryRef::new(memory_region_symbol!(virtio_net_tx_used: *mut _)) },
            notify_client,
        );
    (rx_ring_buffers, tx_ring_buffers)
}

#[protection_domain(heap_size = 512 * 1024)]
fn init() -> HandlerImpl<DeviceWrapper<DriverHal, NetTransport>> {
    debug::init().unwrap();
    #[cfg(feature = "unified-dma")]
    {
        dma::init_hal_unified();
        log::info!("virtio-net: unified-dma (no client_dma map)");
    }
    #[cfg(all(
        not(feature = "unified-dma"),
        any(
            feature = "board-x86_64_generic_virtio",
            feature = "board-x86_64_generic_http"
        )
    ))]
    pci::init_hal();
    #[cfg(all(
        not(feature = "unified-dma"),
        not(any(
            feature = "board-x86_64_generic_virtio",
            feature = "board-x86_64_generic_http"
        ))
    ))]
    mmio::init_hal();
    let mut dev = {
        #[cfg(any(
            feature = "board-x86_64_generic_virtio",
            feature = "board-x86_64_generic_http"
        ))]
        {
            pci::create_virtio_net()
        }
        #[cfg(not(any(
            feature = "board-x86_64_generic_virtio",
            feature = "board-x86_64_generic_http"
        )))]
        {
            mmio::create_virtio_net()
        }
    };
    let client_region = create_client_region();
    let notify_client: fn() = || channels::CLIENT.notify();
    let (rx_ring_buffers, tx_ring_buffers) = create_net_ring_buffers(notify_client);
    dev.ack_interrupt();
    channels::DEVICE.irq_ack().unwrap();
    log::info!("virtio-net driver ready");
    HandlerImpl::new(
        DeviceWrapper::new(dev),
        client_region,
        rx_ring_buffers,
        tx_ring_buffers,
        channels::DEVICE,
        channels::CLIENT,
    )
}
