//! Kernel debug-print logging sink (`sel4::debug_print!`).

use log::SetLoggerError;

use sel4_logging::{LevelFilter, Logger, LoggerBuilder};

use crate::default_filter;

const LOG_LEVEL: LevelFilter = LevelFilter::Info;

static LOGGER: Logger = LoggerBuilder::const_default()
    .level_filter(LOG_LEVEL)
    .filter(|meta| default_filter(meta))
    .write(|s| sel4::debug_print!("{s}"))
    .build();

/// Install the static debug-print logger at [`LOG_LEVEL`].
pub fn init() -> Result<(), SetLoggerError> {
    LOGGER.set()
}
