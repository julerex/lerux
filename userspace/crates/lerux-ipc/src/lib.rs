//! Typed postcard IPC helpers for lerux protection domains.
//!
//! Re-exports upstream [`sel4-microkit-simple-ipc`] for custom RPC between
//! PDs, and provides typed service clients ([`FsClient`], [`NetClient`],
//! [`BlkClient`]) that own the shared Pending → Poll completion loop.

#![no_std]

mod client;

pub use client::{
    BlkClient, BlkProtocol, FsClient, FsProtocol, NetClient, NetProtocol, PollProtocol,
    ServiceClient,
};
pub use sel4_microkit::Channel;
pub use sel4_microkit_simple_ipc::{
    self, call, recv, send, send_unspecified_error, try_call, try_send, RecvError, TryCallError,
    UNSPECIFIED_ERROR_MESSAGE_LABEL,
};
pub use serde::{Deserialize, Serialize};
