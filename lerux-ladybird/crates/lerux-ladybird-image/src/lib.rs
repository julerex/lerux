//! Stand-in for Ladybird `Libraries/LibImageDecoders`.
//!
//! [`decode_png`] reads an 8-bit, non-interlaced RGB or RGBA PNG into tightly
//! packed B, G, R, A. Inflate and the scanline filters are written here.
//! [`paint_page`] paints `support/browser/lenna.html` into a caller's 32-bit
//! framebuffer.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod inflate;
mod page;
mod png;

pub use page::{paint_page, PageError, Shifts};
pub use png::{decode_png, Image, PngError};
