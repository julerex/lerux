//! Kernel debug-print logging sink (`sel4::debug_print!`).
//!
//! sel4-logging may invoke the write callback with short chunks. We coalesce
//! until a newline and print one line per syscall burst so concurrent PDs do
//! not splice tokens (e.g. `virtio-net: MAC`) mid-message.

use log::SetLoggerError;

use sel4_logging::{LevelFilter, Logger, LoggerBuilder};

use crate::{default_filter, linebuf::StaticLineBuf};

const LOG_LEVEL: LevelFilter = LevelFilter::Info;

static LINE: StaticLineBuf = StaticLineBuf::new();

fn debug_write(s: &str) {
    // SAFETY: Microkit PDs are single-threaded; init() runs before any log call.
    unsafe {
        LINE.push(s, |line| sel4::debug_print!("{line}\n"));
    }
}

static LOGGER: Logger = LoggerBuilder::const_default()
    .level_filter(LOG_LEVEL)
    .filter(default_filter)
    .write(debug_write)
    .build();

/// Install the static debug-print logger at [`LOG_LEVEL`].
pub fn init() -> Result<(), SetLoggerError> {
    LOGGER.set()
}
