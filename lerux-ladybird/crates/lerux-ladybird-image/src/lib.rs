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
mod slot;

pub use page::{paint_page, PageError, Shifts};
pub use png::{decode_png, Image, PngError};
pub use slot::{
    ImagePage, ENCODED_CAP, ENCODED_REGION, ENCODED_VADDR, IMAGE_DECODER_CHANNEL, IMAGE_REGION,
    IMAGE_REGION_SIZE, IMAGE_STATE_ASK, IMAGE_STATE_ERR, IMAGE_STATE_IDLE, IMAGE_STATE_OK,
    IMAGE_VADDR, PIXEL_CAP, PIXEL_REGION, PIXEL_VADDR, WEB_CONTENT_IMAGE_CHANNEL,
};
