//! Shared memory structures from LionsOS 0.4.0 and the seL4 Device Driver Framework it pins.
//!
//! The C headers are the specification. These types use the same names, fields, and
//! `#[repr(C)]` layout. Guest code does not link the C. A host test compiles the
//! headers and compares size and field offset. Run `lerux fetch` before that test.

#![cfg_attr(not(test), no_std)]
#![expect(
    non_camel_case_types,
    reason = "type names match the LionsOS and driver-framework C headers"
)]

pub mod blk;
pub mod blk_image;
pub mod blk_queue;
pub mod fs;
pub mod fs_image;
pub mod fs_queue;
pub mod net;
pub mod net_image;
pub mod net_queue;
pub mod serial;
pub mod serial_image;
pub mod serial_queue;

pub use blk::*;
pub use blk_queue::*;
pub use fs::*;
pub use fs_queue::*;
pub use net::*;
pub use net_queue::*;
pub use serial::*;
pub use serial_queue::*;
