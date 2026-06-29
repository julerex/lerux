//! Serial driver IPC logging sink for client protection domains.

use core::cell::UnsafeCell;

use embedded_hal_nb::serial::Write;
use log::SetLoggerError;
use sel4_logging::{LevelFilter, Logger, LoggerBuilder};
use sel4_microkit::Channel;
use sel4_microkit_driver_adapters::serial::client::Client as SerialClient;

use crate::default_filter;

const LOG_LEVEL: LevelFilter = LevelFilter::Info;

struct SerialSlot(UnsafeCell<Option<SerialClient>>);

// Single-threaded Microkit PDs: the client is installed once in init().
unsafe impl Sync for SerialSlot {}

static SERIAL: SerialSlot = SerialSlot(UnsafeCell::new(None));

fn serial_write(s: &str) {
    // SAFETY: Microkit PDs are single-threaded; init() runs before any log call.
    unsafe {
        if let Some(client) = &mut *SERIAL.0.get() {
            for b in s.bytes() {
                let _ = client.write(b);
            }
        }
    }
}

static LOGGER: Logger = LoggerBuilder::const_default()
    .level_filter(LOG_LEVEL)
    .filter(default_filter)
    .write(serial_write)
    .build();

/// Route log output through the serial driver PD on `channel`.
pub fn init(channel: Channel) -> Result<(), SetLoggerError> {
    // SAFETY: called once from the PD entry point before other threads run.
    unsafe {
        *SERIAL.0.get() = Some(SerialClient::new(channel));
    }
    LOGGER.set()
}
