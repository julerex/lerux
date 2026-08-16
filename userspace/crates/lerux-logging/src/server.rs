//! Log server IPC sink for client protection domains (Phase 36 / 57).
//!
//! PDs send `LogRequest::Append` to the log-server PD (instead of raw serial
//! writes). This allows a central ring buffer, `dmesg` filters, and tags.
//!
//! sel4-logging may invoke the write callback with short chunks; we coalesce
//! until a newline so each Append is one logical line.

use core::cell::UnsafeCell;

use log::SetLoggerError;
use sel4_logging::{LevelFilter, Logger, LoggerBuilder};
use sel4_microkit::Channel;

use lerux_interface_types::{
    LogRequest, LogResponse, LOG_LEVEL_DEBUG, LOG_LEVEL_ERROR, LOG_LEVEL_INFO, LOG_LEVEL_WARN,
    MAX_LOG_MSG, MAX_LOG_TAG,
};
use lerux_ipc::call;

use crate::{default_filter, linebuf::StaticLineBuf};

/// Accept debug+ so log.level policy can filter at the log-server.
const LOG_LEVEL: LevelFilter = LevelFilter::Debug;

struct LogServerSlot(UnsafeCell<Option<Channel>>);
struct TagSlot(UnsafeCell<[u8; MAX_LOG_TAG]>);
struct TagLenSlot(UnsafeCell<u8>);

unsafe impl Sync for LogServerSlot {}
unsafe impl Sync for TagSlot {}
unsafe impl Sync for TagLenSlot {}

static LOG_SERVER: LogServerSlot = LogServerSlot(UnsafeCell::new(None));
static PD_TAG: TagSlot = TagSlot(UnsafeCell::new([0u8; MAX_LOG_TAG]));
static PD_TAG_LEN: TagLenSlot = TagLenSlot(UnsafeCell::new(0));
static LINE: StaticLineBuf<{ MAX_LOG_MSG }> = StaticLineBuf::new();

/// Infer level from sel4-logging's formatted prefix (`ERROR`/`WARN`/`INFO`/`DEBUG`).
fn level_from_formatted(s: &str) -> u8 {
    let head = s.trim_start();
    if head.starts_with("ERROR") || head.starts_with("error") {
        LOG_LEVEL_ERROR
    } else if head.starts_with("WARN") || head.starts_with("warn") {
        LOG_LEVEL_WARN
    } else if head.starts_with("DEBUG")
        || head.starts_with("debug")
        || head.starts_with("TRACE")
        || head.starts_with("trace")
    {
        LOG_LEVEL_DEBUG
    } else {
        LOG_LEVEL_INFO
    }
}

fn send_line(s: &str) {
    // SAFETY: single-threaded PD; slots are set once in init.
    unsafe {
        let Some(ch) = *LOG_SERVER.0.get() else {
            return;
        };
        let tag_len = *PD_TAG_LEN.0.get();
        let tag = *PD_TAG.0.get();
        let tag_slice = &tag[..tag_len as usize];
        let level = level_from_formatted(s);
        let req = LogRequest::append_tagged(level, tag_slice, s.as_bytes());
        let _ = call::<LogRequest, LogResponse>(ch, req);
    }
}

fn log_server_write(s: &str) {
    // SAFETY: single-threaded PD.
    unsafe {
        LINE.push(s, send_line);
    }
}

static LOGGER: Logger = LoggerBuilder::const_default()
    .level_filter(LOG_LEVEL)
    .filter(default_filter)
    .write(log_server_write)
    .build();

/// Route log output through the log-server PD on `channel` (empty tag).
pub fn init(channel: Channel) -> Result<(), SetLoggerError> {
    init_with_tag(channel, b"")
}

/// Route log output through log-server with a fixed PD tag (Phase 57).
///
/// Tag is at most [`MAX_LOG_TAG`] bytes (truncated). Empty tag lets log-server
/// map the Microkit channel to a default name.
pub fn init_with_tag(channel: Channel, tag: &[u8]) -> Result<(), SetLoggerError> {
    // SAFETY: called once from the PD entry point before other threads run.
    unsafe {
        *LOG_SERVER.0.get() = Some(channel);
        let mut t = [0u8; MAX_LOG_TAG];
        let n = tag.len().min(MAX_LOG_TAG);
        t[..n].copy_from_slice(&tag[..n]);
        *PD_TAG.0.get() = t;
        *PD_TAG_LEN.0.get() = n as u8;
    }
    LOGGER.set()
}
