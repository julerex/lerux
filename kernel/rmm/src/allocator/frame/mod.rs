//! Frame allocator traits and implementations (bump + buddy).
//!
//! These provide the low-level physical frame (page) allocation used by the
//! higher-level page table mapper and the kernel memory subsystem.
//!
//! The primary trait is [`FrameAllocator`]. Two concrete implementations are
//! provided: [`BumpAllocator`] (simple, one-way during early boot) and
//! [`BuddyAllocator`] (full coalescing allocator for normal operation).

use crate::PhysicalAddress;

pub use self::{buddy::*, bump::*};

mod buddy;
mod bump;

/// Number of frames (in units of `PAGE_SIZE`).
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct FrameCount(usize);

impl FrameCount {
    /// Construct a frame count.
    pub fn new(count: usize) -> Self {
        Self(count)
    }

    /// Return the raw count.
    pub fn data(&self) -> usize {
        self.0
    }
}

/// Tracks used vs. total frames for an allocator.
#[derive(Debug)]
pub struct FrameUsage {
    used: FrameCount,
    total: FrameCount,
}

impl FrameUsage {
    /// Create a usage snapshot.
    pub fn new(used: FrameCount, total: FrameCount) -> Self {
        Self { used, total }
    }

    /// Frames currently allocated.
    pub fn used(&self) -> FrameCount {
        self.used
    }

    /// Frames still free.
    pub fn free(&self) -> FrameCount {
        FrameCount(self.total.0 - self.used.0)
    }

    /// Total frames managed by the allocator.
    pub fn total(&self) -> FrameCount {
        self.total
    }
}

/// Core physical frame allocator interface.
///
/// # Safety
///
/// Implementors must ensure that:
/// - `allocate` returns distinct, previously unallocated physical frames (or `None`).
/// - `free` releases frames that were previously returned by `allocate` on this allocator
///   and are not double-freed.
/// - The returned addresses are page-aligned and within the addressable physical memory
///   for the platform.
pub unsafe trait FrameAllocator {
    /// Allocate `count` contiguous frames. Returns the base physical address or `None`.
    fn allocate(&mut self, count: FrameCount) -> Option<PhysicalAddress>;

    /// Free `count` contiguous frames starting at `address`.
    ///
    /// # Safety
    /// The caller must guarantee that the frames were previously allocated from this
    /// allocator via `allocate` (or `allocate_one`) and have not already been freed.
    unsafe fn free(&mut self, address: PhysicalAddress, count: FrameCount);

    /// Allocate a single frame (convenience wrapper).
    fn allocate_one(&mut self) -> Option<PhysicalAddress> {
        self.allocate(FrameCount::new(1))
    }

    /// Free a single frame (convenience wrapper).
    ///
    /// # Safety
    /// See [`FrameAllocator::free`].
    unsafe fn free_one(&mut self, address: PhysicalAddress) {
        unsafe {
            self.free(address, FrameCount::new(1));
        }
    }

    /// Return current usage statistics.
    fn usage(&self) -> FrameUsage;
}

unsafe impl<T> FrameAllocator for &mut T
where
    T: FrameAllocator,
{
    fn allocate(&mut self, count: FrameCount) -> Option<PhysicalAddress> {
        T::allocate(self, count)
    }
    unsafe fn free(&mut self, address: PhysicalAddress, count: FrameCount) {
        unsafe { T::free(self, address, count) }
    }
    fn allocate_one(&mut self) -> Option<PhysicalAddress> {
        T::allocate_one(self)
    }
    unsafe fn free_one(&mut self, address: PhysicalAddress) {
        unsafe { T::free_one(self, address) }
    }
    fn usage(&self) -> FrameUsage {
        T::usage(self)
    }
}
