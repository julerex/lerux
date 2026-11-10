//! Stand-in for Ladybird `Libraries/LibImageDecoders`.
//!
//! [`decode`] reads an 8-bit, non-interlaced PNG with one image header and one
//! data chunk. The color type is RGB or RGBA. Inflate lives in this crate
//! because a protection domain is `no_std` and the plan does not take `image`,
//! `png`, or `flate2`.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod checksum;
mod error;
mod inflate;
mod png;

pub use error::DecodeError;
pub use png::{decode, Image, Pixel};
