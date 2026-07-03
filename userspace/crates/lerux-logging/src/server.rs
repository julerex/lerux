//! Log server IPC sink for client protection domains (Phase 36).
//!
//! PDs send LogRequest::Append to the log-server PD (instead of raw serial writes).
//! This allows central ring buffer + dmesg + future subscribe.

use core::cell::UnsafeCell;

use log::SetLoggerError;
use sel4_logging::{LevelFilter, Logger, LoggerBuilder};
use sel4_microkit::Channel;

use lerux_interface_types::{LogRequest, LogResponse};
use lerux_ipc::call;

use crate::default_filter;

const LOG_LEVEL: LevelFilter = LevelFilter::Info;

struct LogServerSlot(UnsafeCell<Option<Channel>>);

unsafe impl Sync for LogServerSlot {}

static LOG_SERVER: LogServerSlot = LogServerSlot(UnsafeCell::new(None));

fn log_server_write(s: &str) {
    // SAFETY: Microkit PDs are single-threaded; init() runs before any log call.
    unsafe {
        if let Some(ch) = *LOG_SERVER.0.get() {
            let req = LogRequest::append(s.as_bytes());
            // Best-effort; ignore result to avoid recursion or stalls in logging path.
            let _ = call::<LogRequest, LogResponse>(ch, req);
        }
    }
}

static LOGGER: Logger = LoggerBuilder::const_default()
    .level_filter(LOG_LEVEL)
    .filter(default_filter)
    .write(log_server_write)
    .build();

/// Route log output through the log-server PD on `channel`.
pub fn init(channel: Channel) -> Result<(), SetLoggerError> {
    // SAFETY: called once from the PD entry point before other threads run.
    unsafe {
        *LOG_SERVER.0.get() = Some(channel);
    }
    LOGGER.set()
}
