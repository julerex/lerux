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

/// PVH 32->64-bit boot stub for direct-boot (lerux: pure-Rust global_asm!, no C toolchain).
/// Upstream used `pvh_boot.S` + `cc`. Not present in upstream when `direct-boot` is disabled.
#[cfg(all(target_arch = "x86_64", feature = "direct-boot"))]
pub mod pvh_boot;

#[cfg(target_arch = "x86")]
pub use ::rmm::x86::X86Arch as CurrentRmmArch;

#[cfg(target_arch = "x86_64")]
pub use ::rmm::x86_64::X8664Arch as CurrentRmmArch;

// Flags
pub mod flags {
    pub const FLAG_SINGLESTEP: usize = 1 << 8;
}
