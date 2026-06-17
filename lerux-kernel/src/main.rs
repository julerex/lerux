#![allow(uninhabited_static)]
#![allow(static_mut_refs)] // uninhabited HandleMap statics in schemes (L1 + RwLock pattern)
//! # The lerux kernel (a Redox-derived microkernel)
//!
//! This crate is the kernel: the first Rust code that owns the machine after
//! boot. It is a **microkernel**, meaning it keeps only the parts that *must* be
//! privileged — memory mapping, scheduling, interrupts, and inter-process
//! communication — while drivers and filesystems run as ordinary userspace
//! programs. It supports `x86_64` (the primary target) plus in-progress
//! `aarch64`/`riscv64` ports, and exposes Unix-like syscalls to primarily Rust
//! applications.
//!
//! New to operating systems? Read [`docs/kernel/architecture.md`] first — it is
//! a beginner's tour of every subsystem mentioned below and assumes no prior OS
//! knowledge.
//!
//! [`docs/kernel/architecture.md`]: ../../../docs/kernel/architecture.md
//!
//! ## Where to start reading
//!
//! This file ([`main.rs`]) is the crate root: it declares every module and wires
//! in the inlined dependency crates. A good reading order for newcomers:
//!
//! 1. [`startup`] — how the kernel comes up, from the bootloader handoff
//!    ([`startup::KernelArgs`]) to `kmain`. lerux adds a `startup::direct_boot`
//!    mode for booting under `qemu -kernel` with no bootloader.
//! 2. [`memory`] — physical RAM and frame allocation (a thin layer over the
//!    inlined [`rmm`] crate), then [`context::memory`] for per-process virtual
//!    address spaces, `mmap`, grants, and page faults.
//! 3. [`context`] — what a process/thread ("context") is, and the round-robin
//!    [`context::switch`] scheduler.
//! 4. [`syscall`] — the user/kernel boundary: how syscalls are dispatched and
//!    how user memory is safely copied.
//! 5. [`scheme`] — the "everything is a scheme" layer that lets userspace
//!    drivers and filesystems plug in (the core microkernel IPC mechanism).
//!
//! [`main.rs`]: crate
//!
//! ## lerux zero-dep vendoring note
//!
//! This is the self-contained lerux kernel (zero runtime Cargo dependencies).
//! All crates that were previously external (bitflags, hashbrown, spin, rmm,
//! redox_syscall as "syscall", redox_path, object, rustc-demangle, linked_list_allocator,
//! slab, smallvec, arrayvec, bitfield, fdt, raw-cpuid, x86, plus needed transitives
//! and the former build-dep toml) have been inlined from their vendor/ snapshots
//! into lerux-kernel/src/lerux-*/ (with original module names rebound via #[path]
//! so the rest of the kernel code is unchanged).
//! The working sources are under lerux-kernel/; pristine references stay in vendor/.
//! Inlining them this way is part of the "Only Rust" goal: the kernel builds with
//! no external runtime crates. See `docs/vendored.md` and `vendor/README.md`, and
//! the "inlined crates" section of `docs/kernel/architecture.md`.

#![feature(int_roundings)]
#![feature(str_split_remainder)]
#![cfg_attr(dtb, feature(iter_next_chunk))]
#![feature(sync_unsafe_cell)]
#![feature(btree_cursors)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(clippy::new_without_default)]

#[macro_use]
extern crate alloc;

// Inlined crates from vendor/ (all under lerux-kernel/src/lerux-*/).
// Original names are used for the modules so that the rest of the kernel
// code (hundreds of use xxx:: / xxx:: / bitflags! / extern crate syscall; sites)
// requires no further changes. The physical dirs use the lerux-* prefix
// as requested.

// Put the ones that the kernel's own modules (memory, arch, scheme, acpi, etc.)
// depend on *first* so name resolution sees them when those modules are processed.
#[path = "lerux-bitflags/lib.rs"]
#[macro_use]
mod bitflags;

// Re-export bitflags items at the crate root so $crate:: paths emitted by the inlined
// bitflags! macro resolve correctly (bitflags is inlined via #[path], not a separate crate).
#[doc(hidden)]
pub use bitflags::__private;
pub use bitflags::{iter, parser, BitFlags, Bits, Flag, Flags};
#[path = "lerux-rmm/lib.rs"]
mod rmm;

#[path = "lerux-syscall/lib.rs"]
mod __redox_syscall;

#[path = "lerux-spin/lib.rs"]
mod spin;

#[path = "lerux-scopeguard/lib.rs"]
#[macro_use]
mod scopeguard;

#[path = "lerux-lock-api/lib.rs"]
mod lock_api;

#[path = "lerux-spinning-top/lib.rs"]
mod spinning_top;

#[path = "lerux-cfg-if/lib.rs"]
#[macro_use]
mod cfg_if;

#[path = "lerux-hashbrown/lib.rs"]
mod hashbrown;

#[path = "lerux-ahash/lib.rs"]
mod ahash;

#[path = "lerux-arrayvec/lib.rs"]
mod arrayvec;

#[path = "lerux-bitfield/lib.rs"]
mod bitfield;

#[path = "lerux-bit-field/lib.rs"]
mod bit_field;

#[path = "lerux-fdt/lib.rs"]
mod fdt;

#[path = "lerux-linked-list-allocator/lib.rs"]
mod linked_list_allocator;

#[path = "lerux-object/lib.rs"]
mod object;

#[path = "lerux-memchr/lib.rs"]
mod memchr;

#[path = "lerux-raw-cpuid/lib.rs"]
mod raw_cpuid;

#[path = "lerux-redox-path/lib.rs"]
mod redox_path;

#[path = "lerux-rustc-demangle/lib.rs"]
mod rustc_demangle;

#[path = "lerux-slab/lib.rs"]
mod slab;

#[path = "lerux-smallvec/lib.rs"]
mod smallvec;

#[path = "lerux-x86/lib.rs"]
mod x86;

// Build-time toml (and its closure) are also inlined under lerux-*/ but
// are only referenced from build.rs (via include in the build script context).

use core::sync::atomic::{AtomicU32, Ordering};

#[macro_use]
/// Shared data structures
mod common;

#[macro_use]
mod macros;

/// Architecture-dependent stuff
#[macro_use]
#[allow(dead_code)] // TODO
mod arch;
use crate::arch::{consts::*, ipi, stop, CurrentRmmArch};
/// Offset of physmap
#[cfg_attr(any(target_arch = "x86", target_arch = "x86_64"), expect(dead_code))]
const PHYS_OFFSET: usize = <arch::CurrentRmmArch as crate::rmm::Arch>::PHYS_OFFSET;

/// Heap allocators
mod allocator;

/// ACPI table parsing
mod acpi;

mod dtb;

/// Logical CPU ID and bitset types
mod cpu_set;

/// Stats for the CPUs
mod cpu_stats;

/// Context management
mod context;

/// Debugger
#[cfg(feature = "debugger")]
mod debugger;

/// Architecture-independent devices
mod devices;

/// Event handling
mod event;

/// Logging
mod log;

/// Memory management
mod memory;

/// Panic
mod panic;

mod percpu;

/// Process tracing
mod ptrace;

/// Performance profiling of the kernel
mod profiling;

/// Schemes, filesystem handlers
mod scheme;

/// Early init
mod startup;

/// Synchronization primitives
mod sync;

/// Syscall handlers
mod syscall;

/// Time
mod time;

#[cfg_attr(not(test), global_allocator)]
static ALLOCATOR: allocator::Allocator = allocator::Allocator;

/// Get the current CPU's scheduling ID
#[inline(always)]
fn cpu_id() -> crate::cpu_set::LogicalCpuId {
    crate::percpu::PercpuBlock::current().cpu_id
}

/// The count of all CPUs that can have work scheduled
static CPU_COUNT: AtomicU32 = AtomicU32::new(1);

/// Get the number of CPUs currently active
#[inline(always)]
fn cpu_count() -> u32 {
    CPU_COUNT.load(Ordering::Relaxed)
}

macro_rules! linker_offsets(
    ($($name:ident),*) => {
        $(
        #[inline(always)]
        #[allow(non_snake_case)]
        pub fn $name() -> usize {
            unsafe extern "C" {
                // TODO: UnsafeCell?
                static $name: u8;
            }
            (&raw const $name) as usize
        }
        )*
    }
);
mod kernel_executable_offsets {
    linker_offsets!(
        KERNEL_OFFSET,
        __text_start,
        __text_end,
        __rodata_start,
        __rodata_end,
        __usercopy_start,
        __usercopy_end
    );

    #[cfg(target_arch = "x86_64")]
    linker_offsets!(__altrelocs_start, __altrelocs_end);
}
