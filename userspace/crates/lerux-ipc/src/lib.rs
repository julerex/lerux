//! Typed postcard IPC helpers for lerux protection domains.
//!
//! Re-exports upstream [`sel4-microkit-simple-ipc`] for custom RPC between PDs.

#![no_std]

pub use sel4_microkit::Channel;
pub use sel4_microkit_simple_ipc::{
    self, call, recv, send, send_unspecified_error, try_call, try_send, RecvError, TryCallError,
    UNSPECIFIED_ERROR_MESSAGE_LABEL,
};
pub use serde::{Deserialize, Serialize};