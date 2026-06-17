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
