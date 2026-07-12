//! Phase 43: unified DMA layout for sDDF-shaped maps.
//!
//! One physical region (`virtio_net_driver_dma`) is split:
//! - `[0, HAL_SIZE)` — virtio-drivers HalImpl (device rings/buffers)
//! - `[HAL_SIZE, HAL_SIZE+BOUNCE_SIZE)` — shared bounce with the stack PD
//!
//! The NIC driver therefore has **no separate client_dma map**. The bounce half
//! is the trusted subsystem data region (driver + net-server / net-virt only),
//! matching sDDF “DMA region” rather than untrusted client data.
//!
//! Each `var!` symbol must be declared exactly once (the macro expands to a
//! `#[no_mangle]` static).

use core::ptr::{self, NonNull};

use sel4_microkit::var;
use sel4_shared_memory::SharedMemoryRef;

use crate::config;

/// Size of the HalImpl sub-region at the base of `virtio_net_driver_dma`.
pub const HAL_SIZE: usize = 0x100_000;

/// Size of the shared bounce sub-region (was a separate client_dma MR).
pub const BOUNCE_SIZE: usize = 0x100_000;

const _: () = assert!(config::VIRTIO_NET_DRIVER_DMA_SIZE >= HAL_SIZE + BOUNCE_SIZE);

/// Single definition site for driver DMA vaddr/paddr symbols.
fn driver_dma_base() -> (usize, usize) {
    let vaddr = *var!(virtio_net_driver_dma_vaddr: usize = 0);
    let paddr = *var!(virtio_net_driver_dma_paddr: usize = 0);
    (vaddr, paddr)
}

/// Initialize HalImpl on the low half of the driver DMA region.
pub fn init_hal_unified() {
    let (vaddr, paddr) = driver_dma_base();
    sel4_virtio_hal_impl::HalImpl::init(HAL_SIZE, vaddr, paddr);
}

/// Bounce buffer shared with the stack PD (high half of driver DMA).
pub fn bounce_region() -> SharedMemoryRef<'static, [u8]> {
    let (vaddr, _) = driver_dma_base();
    let ptr = NonNull::new(ptr::slice_from_raw_parts_mut(
        (vaddr + HAL_SIZE) as *mut u8,
        BOUNCE_SIZE,
    ))
    .expect("net bounce region");
    unsafe { SharedMemoryRef::new(ptr) }
}
