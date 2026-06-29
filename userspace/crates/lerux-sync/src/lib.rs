//! seL4 notification-based synchronization for lerux protection domains.
//!
//! Re-exports upstream [`sel4-sync`]. Use when a PD has multiple threads or a
//! synchronized global allocator.

#![no_std]

pub use lock_api;
pub use sel4_sync::{RawDeferredNotificationMutex, RawLazyNotificationMutex, RawNotificationMutex};

use lock_api::Mutex;
use sel4_sync::RawNotificationMutex;

/// Notification-backed mutex for shared state in multi-threaded PDs.
pub type NotificationMutex<T> = Mutex<RawNotificationMutex, T>;
