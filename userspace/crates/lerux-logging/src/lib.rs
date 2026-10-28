//! Convenience logging setup for lerux protection domains.
//!
//! Wraps upstream [`sel4-logging`] with sinks appropriate for Microkit PDs.

#![no_std]

use log::Metadata;
pub use log::{self, LevelFilter};
pub use sel4_logging::{Logger, LoggerBuilder};

mod linebuf;

#[cfg(feature = "serial")]
pub mod serial;

pub mod debug;

#[cfg(any(feature = "boot-line-gap", test))]
mod gap;

/// Filter out noisy `sel4_sys` targets (matches rust-sel4 http-server example).
#[must_use]
pub fn default_filter(meta: &Metadata) -> bool {
    !meta.target().starts_with("sel4_sys")
}
