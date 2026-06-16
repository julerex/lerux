#![no_std]
//! RMM — Redox Memory Manager.
//!
//! This crate provides the core physical memory management primitives used by the
//! Redox / lerux kernel:
//!
//! * Typed physical / virtual address wrappers ([`PhysicalAddress`], [`VirtualAddress`]).
//! * The [`Arch`] trait abstracting architecture-specific page-table and MMU operations.
//! * Frame allocators ([`BumpAllocator`], [`BuddyAllocator`]) implementing [`FrameAllocator`].
//! * Page table entry / flag / mapper abstractions (re-exported from `page`).
//!
//! The crate is `no_std` by default but can be compiled with the `std` feature for
//! host-side testing (see the paging `is_canonical` tests and allocator tests).
//!
//! Many items are re-exported at the crate root for convenience.

#![allow(clippy::new_without_default)]

pub use crate::{allocator::*, arch::*, page::*};

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
