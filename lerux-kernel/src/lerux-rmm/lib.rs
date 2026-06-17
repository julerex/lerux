//! RMM — the Redox Memory Manager (inlined as `lerux-rmm`).
//!
//! This is the **lowest** layer of the kernel's memory system: the part that
//! actually understands the page-table format of each CPU architecture and
//! tracks raw physical frames. Everything above it ([`crate::memory`],
//! [`crate::context::memory`]) builds policy on top of these primitives.
//!
//! ## Why it is inlined
//!
//! Upstream this is the external `rmm` crate. lerux copies it into the tree (as
//! `lerux-rmm`, wired in via `#[path]` in `main.rs`) so the kernel has zero
//! external runtime dependencies — part of the "Only Rust" goal. The code is
//! otherwise upstream; treat it as part of the kernel.
//!
//! ## What it provides
//!
//! - [`PhysicalAddress`] / [`VirtualAddress`] — newtypes that keep the two
//!   kinds of address from being confused (a frequent source of OS bugs). A
//!   physical address names a byte of real RAM; a virtual address is what a
//!   program uses and must be translated through page tables.
//! - [`TableKind`] — whether a page table is for userspace or the kernel.
//! - [`MemoryArea`] — a contiguous physical region (base + size).
//! - The architecture-specific `Arch`/`PageMapper`/`PageFlags` machinery
//!   (re-exported from the `arch` submodule) that reads and writes the actual
//!   page tables, plus the frame allocators in the `allocator` submodule.
//!
//! See also: [`docs/kernel/architecture.md`] section 4, and `docs/kernel/rmm.md`.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

pub use self::{allocator::*, arch::*, page::*};

mod allocator;
mod arch;
mod page;

pub const KILOBYTE: usize = 1024;
pub const MEGABYTE: usize = KILOBYTE * 1024;
pub const GIGABYTE: usize = MEGABYTE * 1024;
#[cfg(target_pointer_width = "64")]
pub const TERABYTE: usize = GIGABYTE * 1024;

/// Specific table to be used, needed on some architectures
//TODO: Use this throughout the code
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TableKind {
    /// Userspace page table
    User,
    /// Kernel page table
    Kernel,
}

/// Physical memory address
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct PhysicalAddress(usize);

impl PhysicalAddress {
    #[inline(always)]
    pub const fn new(address: usize) -> Self {
        Self(address)
    }

    #[inline(always)]
    pub fn data(&self) -> usize {
        self.0
    }

    #[expect(clippy::should_implement_trait)]
    #[inline(always)]
    pub fn add(self, offset: usize) -> Self {
        Self(self.0 + offset)
    }
}

impl core::fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[phys {:#0x}]", self.data())
    }
}

/// Virtual memory address
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct VirtualAddress(usize);

impl VirtualAddress {
    #[inline(always)]
    pub const fn new(address: usize) -> Self {
        Self(address)
    }

    #[inline(always)]
    pub fn data(&self) -> usize {
        self.0
    }

    #[expect(clippy::should_implement_trait)]
    #[inline(always)]
    pub fn add(self, offset: usize) -> Self {
        Self(self.0 + offset)
    }

    #[inline(always)]
    pub fn kind(&self) -> TableKind {
        if (self.0 as isize) < 0 {
            TableKind::Kernel
        } else {
            TableKind::User
        }
    }
}

impl core::fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[virt {:#0x}]", self.data())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MemoryArea {
    pub base: PhysicalAddress,
    pub size: usize,
}
