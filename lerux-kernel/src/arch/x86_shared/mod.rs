//! Code shared by the 32-bit `x86` and 64-bit `x86_64` ports.
//!
//! The two x86 variants share most of their low-level machinery, so it lives
//! here and each arch module re-exports it. This includes the **GDT** (segment
//! descriptors), **IDT** (interrupt vector table), interrupt/exception entry,
//! inter-processor interrupts ([`ipi`]), paging glue, the SMP [`trampoline`]
//! (assembled from `lerux-kernel/src/asm/` via nasm in `build.rs`),
//! and serial/timer device access.
//!
//! Key terms: the **GDT** and **IDT** are small CPU-mandated tables. The GDT
//! defines memory segments and privilege levels; the IDT maps each interrupt or
//! exception number to the kernel routine that handles it. Both must be set up
//! very early in boot before interrupts can be taken safely.
//!
//! See also: [`docs/kernel/architecture.md`] sections 8 ("Interrupts") and 9
//! ("SMP").
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

/// CPUID wrapper
pub mod cpuid;

/// Debugging support
pub mod debug;

/// Devices
pub mod device;

/// Global descriptor table
pub mod gdt;

/// Interrupt descriptor table
pub mod idt;

/// Interrupt instructions
pub mod interrupt;

/// Inter-processor interrupts
pub mod ipi;

/// Paging
pub mod paging;

/// Page table isolation
pub mod pti;

/// Initialization and start function
pub mod start;

/// Stop function
pub mod stop;

pub mod time;
pub mod trampoline;



#[cfg(target_arch = "x86")]
pub use crate::rmm::x86::X86Arch as CurrentRmmArch;

#[cfg(target_arch = "x86_64")]
pub use crate::rmm::x86_64::X8664Arch as CurrentRmmArch;

// Flags
pub mod flags {
    pub const FLAG_SINGLESTEP: usize = 1 << 8;
}
