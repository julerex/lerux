//! Architecture-specific code, selected at compile time.
//!
//! Everything that depends on the exact CPU — how interrupts and syscalls are
//! entered, the page-table format, segment/descriptor tables, timers, SMP
//! startup — lives under here. Exactly one of the per-architecture submodules
//! (`x86_64`, `x86`, `aarch64`, `riscv64`) is compiled, chosen by
//! `#[cfg(target_arch = ...)]`, and re-exported at this module's root so the
//! rest of the kernel can write `crate::arch::...` without caring which CPU it
//! is building for.
//!
//! `x86` and `x86_64` additionally share a large `x86_shared` submodule, since
//! the two are closely related.
//!
//! x86_64 is lerux's primary target; the other ports are in varying stages of
//! bring-up (see `docs/kernel/arm-port-outline.md`).
//!
//! See also: [`docs/kernel/architecture.md`] sections 3, 8, 9.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

#[cfg(target_arch = "aarch64")]
#[macro_use]
pub mod aarch64;
#[cfg(target_arch = "aarch64")]
pub use self::aarch64::*;

#[cfg(target_arch = "x86")]
#[macro_use]
pub mod x86;
#[cfg(target_arch = "x86")]
pub use self::x86::*;

#[cfg(target_arch = "x86_64")]
#[macro_use]
pub mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use self::x86_64::*;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[macro_use]
mod x86_shared;

#[cfg(target_arch = "riscv64")]
#[macro_use]
pub mod riscv64;
#[cfg(target_arch = "riscv64")]
pub use self::riscv64::*;
