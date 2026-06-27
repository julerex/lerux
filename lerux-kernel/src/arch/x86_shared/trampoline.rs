//! SMP AP bring-up trampoline for x86 and x86_64.
//!
//! This module provides the raw binary image that is copied to physical address 0x8000
//! and executed in real mode by Application Processors after receiving a SIPI.
//!
//! # Build
//!
//! Assembled from `lerux-kernel/src/asm/{x86,x86_64}/trampoline.asm` via **nasm** in
//! `build.rs`. The binary is included at compile time from `OUT_DIR/trampoline`.
//!
//! The data area (4 x u64 at offsets 8,16,24,32) is patched at runtime by the BSP
//! (see acpi/madt/arch/x86.rs).

#![allow(dead_code)]

/// The trampoline binary for the current x86 target.
///
/// Flat binary linked at physical address 0x8000 (16-bit real mode → long/protected mode).
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub static TRAMPOLINE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/trampoline"));

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
pub static TRAMPOLINE: &[u8] = &[]; // Never used; other arches do not have this trampoline.

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn x86_64_trampoline_has_expected_size() {
        assert_eq!(super::TRAMPOLINE.len(), 202);
        assert_eq!(&super::TRAMPOLINE[8..40], &[0u8; 32]);
    }

    #[test]
    #[cfg(target_arch = "x86")]
    fn x86_trampoline_has_expected_size() {
        assert_eq!(super::TRAMPOLINE.len(), 175);
        assert_eq!(&super::TRAMPOLINE[8..40], &[0u8; 32]);
    }
}