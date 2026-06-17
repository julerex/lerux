//! The kernel heap allocator.
//!
//! Until this is set up, the kernel cannot use `Box`, `Vec`, `String`, or
//! anything else from the `alloc` crate — there is simply nowhere to put
//! heap-allocated data. This module reserves a fixed virtual range, maps real
//! physical frames behind it, and registers a `#[global_allocator]` (see
//! [`Allocator`]) so the rest of the kernel can allocate normally.
//!
//! ## How it fits together
//!
//! - [`init`] is called once during early boot (after paging is up) to map the
//!   heap pages and initialize the underlying allocator.
//! - The actual allocation algorithm is a **linked-list allocator**: free
//!   memory is threaded together as a linked list of holes, and allocations
//!   carve chunks out of it. See the `linked_list` submodule (a thin wrapper
//!   over the inlined `lerux-linked-list-allocator` crate).
//!
//! See also: [`docs/kernel/architecture.md`] section 3 ("Boot").
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

use crate::{
    memory::{KernelMapper, Page, PageFlags, VirtualAddress},
    rmm::{Flusher, FrameAllocator, PageFlushAll},
};

pub use self::linked_list::Allocator;
mod linked_list;

/// Size of kernel heap
const KERNEL_HEAP_SIZE: usize = crate::rmm::MEGABYTE;

/// Map physical frames behind the virtual heap range `[offset, offset + size)`.
///
/// One frame is allocated and mapped per page. The mappings are writable and
/// (unless `pti` is enabled) global so they stay resident across address-space
/// switches. The `PageFlushAll` batches TLB invalidations so we flush once at
/// the end rather than per page.
///
/// # Safety
///
/// `mapper` must own the kernel address space, and the range must not already
/// be mapped. Intended to be called only from [`init`].
unsafe fn map_heap(mapper: &mut KernelMapper<true>, offset: usize, size: usize) {
    let mut flush_all = PageFlushAll::new();

    let heap_start_page = Page::containing_address(VirtualAddress::new(offset));
    let heap_end_page = Page::containing_address(VirtualAddress::new(offset + size - 1));
    for page in Page::range_inclusive(heap_start_page, heap_end_page) {
        let phys = mapper
            .allocator_mut()
            .allocate_one()
            .expect("failed to allocate kernel heap");
        let flush = unsafe {
            mapper
                .map_phys(
                    page.start_address(),
                    phys,
                    PageFlags::new()
                        .write(true)
                        .global(cfg!(not(feature = "pti"))),
                )
                .expect("failed to map kernel heap")
        };
        flush_all.consume(flush);
    }

    flush_all.flush();
}

/// Map and initialize the kernel heap. Call exactly once during early boot,
/// after the kernel page tables and frame allocator are ready but before any
/// heap allocation happens.
///
/// # Safety
///
/// Must run once, single-threaded, before the first use of `alloc`.
pub unsafe fn init() {
    unsafe {
        let offset = crate::kernel_heap_offset();
        let size = KERNEL_HEAP_SIZE;

        // Map heap pages
        map_heap(&mut KernelMapper::lock_rw(), offset, size);

        // Initialize global heap
        Allocator::init(offset, size);
    }
}
