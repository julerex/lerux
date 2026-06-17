//! Linked-list based kernel heap allocator wrapper.
//!
//! This provides the global allocator implementation for the kernel using
//! `linked_list_allocator`. It supports dynamic heap extension via the kernel
//! mapper when the initial heap is exhausted.
//!
//! The `Allocator` is a zero-sized type implementing `GlobalAlloc`. Initialization
//! must happen early via `init` before any allocations.

use crate::{linked_list_allocator::Heap, memory::KernelMapper, spin::Mutex};
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::NonNull,
};

static HEAP: Mutex<Option<Heap>> = Mutex::new(None);

/// The kernel's global allocator (linked list based).
pub struct Allocator;

impl Allocator {
    /// Initialize the heap at the given offset with the given size (in bytes).
    ///
    /// # Safety
    /// The memory range `[offset, offset + size)` must be valid, writable,
    /// and not used for anything else. Must be called exactly once before
    /// any allocations.
    pub unsafe fn init(offset: usize, size: usize) {
        unsafe {
            *HEAP.lock() = Some(Heap::new(offset, size));
        }
    }
}

unsafe impl GlobalAlloc for Allocator {
    /// Allocate memory with the given layout.
    ///
    /// On OOM for the current heap region, attempts to extend the heap by
    /// mapping more pages and retrying.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            while let Some(ref mut heap) = *HEAP.lock() {
                match heap.allocate_first_fit(layout) {
                    Ok(ptr) => return ptr.as_ptr(),
                    Err(()) => {
                        let size = heap.size();
                        super::map_heap(
                            &mut KernelMapper::lock_rw(),
                            crate::kernel_heap_offset() + size,
                            super::KERNEL_HEAP_SIZE,
                        );
                        heap.extend(super::KERNEL_HEAP_SIZE);
                    }
                }
            }
            panic!("__rust_allocate: heap not initialized");
        }
    }

    /// Deallocate previously allocated memory.
    ///
    /// # Safety
    /// `ptr` must have been allocated with this allocator using the exact
    /// same `layout`, and must not have been deallocated already.
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            HEAP.lock()
                .as_mut()
                .expect("heap not initialized")
                .deallocate(NonNull::new_unchecked(ptr), layout)
        }
    }
}
