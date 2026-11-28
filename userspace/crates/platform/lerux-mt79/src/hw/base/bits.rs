//! Unsigned helpers used by transcribed function-like macros.
#![allow(clippy::cast_possible_truncation)] // shift counts are narrowed for wrapping shifts
#![allow(clippy::integer_division)] // unsigned division matches the C macros

#[must_use]
pub const fn bit(amount: u64) -> u64 {
    1u64.wrapping_shl(amount as u32)
}

#[must_use]
pub const fn genmask(high: u64, low: u64) -> u64 {
    let top = u64::MAX.wrapping_shr((63u64 - high) as u32);
    let bottom = u64::MAX.wrapping_shl(low as u32);
    top & bottom
}

#[must_use]
pub const fn shl(value: u64, amount: u64) -> u64 {
    value.wrapping_shl(amount as u32)
}

#[must_use]
pub const fn shr(value: u64, amount: u64) -> u64 {
    value.wrapping_shr(amount as u32)
}

/// Divides `left` by `right`.
///
/// # Panics
///
/// Panics when `right` is zero.
#[must_use]
pub const fn div(left: u64, right: u64) -> u64 {
    left / right
}

/// Remainder of `left` divided by `right`.
///
/// # Panics
///
/// Panics when `right` is zero.
#[must_use]
pub const fn rem(left: u64, right: u64) -> u64 {
    left % right
}
