//! SMP AP bring-up trampoline for x86 and x86_64.
//!
//! This module provides the raw binary image that is copied to physical address 0x8000
//! and executed in real mode by Application Processors after receiving a SIPI.
//!
//! # Origin (lerux divergence from upstream Redox)
//!
//! Upstream assembled `src/asm/{x86,x86_64}/trampoline.asm` with **nasm** in
//! `build.rs`. As part of the lerux "Only Rust" goal, that dependency was
//! removed: the exact bytes nasm produced are embedded here as plain data
//! (similar to `res/unifont.font`). See root `VENDORED.md`.
//!
//! # Validation & Regeneration
//!
//! Source of truth: `kernel/validation/trampolines/asm/trampoline_{x86,x86_64}.asm`.
//! Golden binaries: `kernel/validation/trampolines/expected/*.bin`.
//!
//! ```text
//! just validate-trampolines          # byte-for-byte check (requires nasm)
//! cargo test -p trampoline-validation  # host unit tests vs golden files
//! ```
//!
//! After editing the `.asm` files: `./validate-trampolines.sh refresh`, then
//! `./validate-trampolines.sh print-rust` to update the arrays below.
//!
//! The data area (4 x u64 at offsets 8,16,24,32) is patched at runtime by the BSP
//! (see acpi/madt/arch/x86.rs).

#![allow(dead_code)]

/// The trampoline binary for the current x86 target.
///
/// Flat binary linked at physical address 0x8000 (16-bit real mode → long/protected mode).
#[cfg(target_arch = "x86_64")]
pub static TRAMPOLINE: &[u8] =
    include_bytes!("../../../validation/trampolines/expected/trampoline_x86_64.bin");

/// 32-bit x86 (i586) variant of the trampoline.
#[cfg(target_arch = "x86")]
pub static TRAMPOLINE: &[u8] =
    include_bytes!("../../../validation/trampolines/expected/trampoline_x86.bin");

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
pub static TRAMPOLINE: &[u8] = &[]; // Never used; other arches do not have this trampoline.
