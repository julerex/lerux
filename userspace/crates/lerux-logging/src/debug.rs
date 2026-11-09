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
        LINE.push(s, |line| {
            sel4::debug_print!("{line}\n");
            #[cfg(feature = "boot-line-gap")]
            crate::gap::on_line(line);
        });
    }
}

/// Run `hook` after each debug line is printed and before the five-second pause.
///
/// The desk console uses this to copy the line onto the VGA text page. The hook
/// must not log: it runs inside the line buffer.
#[cfg(feature = "boot-line-gap")]
pub fn set_boot_line_hook(hook: fn(&str)) {
    crate::gap::set_hook(hook);
}

/// Wait five seconds. The boot menu calls this after each line, which is not a log line.
#[cfg(feature = "boot-line-gap")]
pub fn after_boot_line() {
    crate::gap::pause();
}

static LOGGER: Logger = LoggerBuilder::const_default()
    .level_filter(LOG_LEVEL)
    .filter(default_filter)
    .write(debug_write)
    .build();

/// Install the static debug-print logger at [`LOG_LEVEL`].
///
/// # Errors
///
/// Returns [`SetLoggerError`] when a logger is already installed.
pub fn init() -> Result<(), SetLoggerError> {
    LOGGER.set()
}
