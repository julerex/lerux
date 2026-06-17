//! [`KernelMapper`]: the guarded handle for editing the kernel's page tables.
//!
//! The top half of every address space (the upper 128 TiB on x86_64) belongs to
//! the kernel and is shared by all processes. Editing those mappings is
//! dangerous if two CPUs do it at once, so this type wraps the kernel
//! `PageMapper` behind a global, re-entrant spinlock.
//!
//! The `const RW: bool` parameter encodes intent in the type: `KernelMapper<false>`
//! (via [`KernelMapper::lock_ro`]) only reads mappings, while
//! `KernelMapper<true>` (via [`KernelMapper::lock_rw`]) may modify them. Asking
//! for write access while the lock is already held read-only is a bug and panics.
//!
//! Important: because expanding the kernel heap itself needs this lock, you must
//! never hold a `KernelMapper` across a heap allocation, or you can deadlock.
//!
//! See also: [`docs/kernel/architecture.md`] section 4 ("Memory model").
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

use crate::rmm::{PageMapper, TableKind};
use core::sync::{
    atomic,
    atomic::{AtomicU32, AtomicUsize, Ordering},
};

const NO_PROCESSOR: u32 = !0;
static LOCK_OWNER: AtomicU32 = AtomicU32::new(NO_PROCESSOR);
static LOCK_COUNT: AtomicUsize = AtomicUsize::new(0);

// TODO: Support, perhaps via const generics, embedding address checking in PageMapper, thereby
// statically enforcing that the kernel mapper can only map things in the kernel half, and vice
// versa.
/// A guard to the global lock protecting the upper 128 TiB of kernel address space.
///
/// NOTE: Use this with great care! Since heap allocations may also require this lock when the heap
/// needs to be expended, it must not be held while memory allocations are done!
// TODO: Make the lock finer-grained so that e.g. the heap part can be independent from e.g.
// PHYS_PML4?
pub struct KernelMapper<const RW: bool> {
    mapper: crate::memory::PageMapper,
}

impl KernelMapper<false> {
    pub fn lock_ro() -> Self {
        KernelMapper::lock()
    }
}

impl KernelMapper<true> {
    pub fn lock_rw() -> Self {
        KernelMapper::lock()
    }
}

impl<const RW: bool> KernelMapper<RW> {
    fn lock() -> Self {
        let mapper =
            unsafe { PageMapper::current(TableKind::Kernel, crate::memory::TheFrameAllocator) };

        let current_processor = crate::cpu_id();
        loop {
            match LOCK_OWNER.compare_exchange_weak(
                NO_PROCESSOR,
                current_processor.get(),
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                // already owned by this hardware thread
                Err(id) if id == current_processor.get() => break,
                // either CAS failed, or some other hardware thread holds the lock
                Err(_) => core::hint::spin_loop(),
            }
        }

        let prev_count = LOCK_COUNT.fetch_add(1, Ordering::Relaxed);
        atomic::compiler_fence(Ordering::Acquire);

        let ro = prev_count > 0;

        if RW && ro {
            panic!("KernelMapper locked re-entrant when write access is requested");
        }

        Self { mapper }
    }
}

impl<const RW: bool> core::ops::Deref for KernelMapper<RW> {
    type Target = crate::memory::PageMapper;

    fn deref(&self) -> &Self::Target {
        &self.mapper
    }
}

impl core::ops::DerefMut for KernelMapper<true> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mapper
    }
}

impl<const RW: bool> Drop for KernelMapper<RW> {
    fn drop(&mut self) {
        if LOCK_COUNT.fetch_sub(1, Ordering::Relaxed) == 1 {
            LOCK_OWNER.store(NO_PROCESSOR, Ordering::Release);
        }
        atomic::compiler_fence(Ordering::Release);
    }
}
