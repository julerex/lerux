//! Virtual addresses and channel ids for the one-client graphics image.
//!
//! `include/sddf/gpu/queue.h` has no configuration struct and no magic. The
//! system template maps the queues, the event words, and the data regions at
//! these addresses. Each protection domain reads the constants. The graphics
//! headers do not name an interrupt field. The template binds the virtio
//! interrupt to [`DRIVER_IRQ_CHANNEL`].
//!
//! Graphics regions start at `0x7_000_000`, clear of serial at `0x3_000_000`,
//! block at `0x4_000_000`, and network at `0x5_000_000`.

use crate::gpu::{gpu_req_queue_t, gpu_req_t, gpu_resp_queue_t, gpu_resp_t};

/// Requests in one queue. Sixteen 56-byte requests fit in [`GPU_REGION_SIZE`].
pub const GPU_QUEUE_CAPACITY: u32 = 16;

/// Mapped size of one queue, event, or driver data page.
pub const GPU_REGION_SIZE: u64 = 0x1000;

/// Client pixel region at [`GPU_CLIENT_DATA_VADDR`]. Equals `PaintedPage::BYTES` (`81920`).
pub const GPU_CLIENT_DATA_SIZE: u64 = 0x1_4000;

/// Virtio ring direct-memory-access region for the graphics driver.
pub const GPU_DRIVER_DMA_SIZE: u64 = 0x200_000;

/// QEMU `virt` on AArch64 places the third virtio-mmio device at `+0xa00` in
/// the page at `0xa003000`, interrupt 77. Virtio-net occupies `+0xe00` and
/// virtio-blk occupies `+0xc00`. The interrupt is not a header field.
pub const GPU_VIRTIO_MMIO_OFFSET: usize = 0xa00;
pub const GPU_VIRTIO_MMIO_SIZE: usize = 0x200;

pub const GPU_VIRTIO_MMIO_VADDR: u64 = 0x6_000_000_000;
pub const GPU_DRIVER_DMA_VADDR: u64 = 0x8_000_000_000;

pub const GPU_DRIVER_EVENTS_VADDR: u64 = 0x7_000_000;
pub const GPU_DRIVER_REQ_VADDR: u64 = 0x7_001_000;
pub const GPU_DRIVER_RESP_VADDR: u64 = 0x7_002_000;
pub const GPU_DRIVER_DATA_VADDR: u64 = 0x7_003_000;
pub const GPU_CLIENT_EVENTS_VADDR: u64 = 0x7_004_000;
pub const GPU_CLIENT_REQ_VADDR: u64 = 0x7_005_000;
pub const GPU_CLIENT_RESP_VADDR: u64 = 0x7_006_000;
pub const GPU_CLIENT_DATA_VADDR: u64 = 0x7_007_000;

/// Channel on `gpu_driver` for the virtio interrupt.
pub const DRIVER_IRQ_CHANNEL: u8 = 0;
pub const DRIVER_VIRT_CHANNEL: u8 = 1;
pub const VIRT_DRIVER_CHANNEL: u8 = 0;
pub const VIRT_CLIENT_CHANNEL: u8 = 1;
/// The shell already uses channels 0 through 4 for serial, the filesystem, and the network.
pub const SHELL_GPU_CHANNEL: u8 = 5;

const _: () = {
    let req = core::mem::size_of::<gpu_req_queue_t>()
        + GPU_QUEUE_CAPACITY as usize * core::mem::size_of::<gpu_req_t>();
    let resp = core::mem::size_of::<gpu_resp_queue_t>()
        + GPU_QUEUE_CAPACITY as usize * core::mem::size_of::<gpu_resp_t>();
    assert!(req <= GPU_REGION_SIZE as usize);
    assert!(resp <= GPU_REGION_SIZE as usize);
};
