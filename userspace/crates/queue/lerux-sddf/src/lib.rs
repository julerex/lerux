//! Shared memory structures from `LionsOS` 0.4.0 and the seL4 Device Driver Framework it pins.
//!
//! The C headers are the specification. These types use the same names, fields, and
//! `#[repr(C)]` layout. Guest code does not link the C. A host test compiles the
//! headers and compares size and field offset. Run `lerux fetch` before that test.

#![cfg_attr(not(test), no_std)]
#![expect(
    non_camel_case_types,
    reason = "type names match the LionsOS and driver-framework C headers"
)]

mod block;
mod files;
mod graphics;
mod network;
mod rest;

pub use block::{blk, blk_image, blk_queue};
pub use files::{fs, fs_image, fs_queue};
pub use graphics::{gpu, gpu_image, gpu_queue};
pub use network::{net, net_image, net_queue};
pub use rest::{
    agent,
    link::{serial, serial_image, serial_queue},
    sched, timer,
};

pub use agent::*;
pub use blk::*;
pub use blk_queue::*;
pub use fs::*;
pub use fs_queue::*;
pub use gpu::*;
pub use gpu_queue::*;
pub use net::*;
pub use net_queue::*;
pub use sched::*;
pub use serial::*;
pub use serial_queue::*;
pub use timer::*;
