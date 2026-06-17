//! 64-bit interrupt and syscall entry.
//!
//! When an interrupt, exception, or `syscall` instruction occurs, the CPU
//! transfers control here. The `handler` submodule defines the low-level entry
//! stubs that save the interrupted program's registers into an
//! [`InterruptStack`], call the appropriate Rust handler, and then restore
//! state and resume. The `syscall` submodule sets up and handles the fast
//! `syscall`/`sysret` path specifically. Most of the generic logic is shared
//! with 32-bit x86 via [`x86_shared::interrupt`](crate::arch).
//!
//! See also: [`docs/kernel/architecture.md`] sections 6 and 8.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

pub use crate::arch::x86_shared::interrupt::*;

#[macro_use]
pub mod handler;

pub mod syscall;

pub use self::handler::InterruptStack;
