//! Inlined vendored crate (`lerux-arrayvec`): fixed-capacity, stack-backed
//! vector and string types.
//!
//! Copied into the kernel tree (wired via `#[path]` in `main.rs`) to keep zero
//! external runtime dependencies — see `docs/vendored.md`. The kernel uses it for
//! bounded buffers that never need the heap (for example context names). The code
//! below is upstream; the original crate docs follow.
//!
//! ---
//!
//! **arrayvec** provides the types [`ArrayVec`] and [`ArrayString`]:
//! array-backed vector and string types, which store their contents inline.

#![no_std]

pub(crate) type LenUint = u32;

macro_rules! assert_capacity_limit {
    ($cap:expr) => {
        if core::mem::size_of::<usize>() > core::mem::size_of::<LenUint>() {
            if $cap > LenUint::MAX as usize {
                panic!("ArrayVec: largest supported capacity is u32::MAX")
            }
        }
    };
}

macro_rules! assert_capacity_limit_const {
    ($cap:expr) => {
        if core::mem::size_of::<usize>() > core::mem::size_of::<LenUint>() {
            if $cap > LenUint::MAX as usize {
                [/*ArrayVec: largest supported capacity is u32::MAX*/][$cap]
            }
        }
    };
}

mod array_string;
mod arrayvec;
mod arrayvec_impl;
mod char;
mod errors;
mod utils;

pub use self::{array_string::ArrayString, errors::CapacityError};

pub use self::arrayvec::{ArrayVec, Drain, IntoIter};
