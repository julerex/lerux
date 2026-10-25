//! Stand-in for Ladybird `Libraries/LibGfx`.
//!
//! The frame is 320 by 64, tightly packed, little-endian B, G, R, A.
//! [`Bitmap`] fills a rectangle, blits a rectangle, sets one pixel, and
//! samples the first pixel that is not opaque white. The bitmap font covers
//! the fixture strings `lerux` and `lerux-http-fixture`. One set bit is one
//! pixel.
//!
//! [`ControlPage`] is the shared control region: a state, a title length, and
//! a title buffer. The bitmap is a separate region of [`FRAME_BYTES`] bytes.
//! Milestone 9 is the first one that maps either region.

#![no_std]

mod bitmap;
mod control;
mod font;

pub use bitmap::{Bitmap, Color, PixelLine, Rect, FRAME_BYTES, FRAME_HEIGHT, FRAME_WIDTH};
pub use control::{
    control_page_zeroed, ControlPage, CONTROL_REGION_SIZE, CONTROL_STATE_IDLE,
    CONTROL_STATE_PAINTED, TITLE_CAP,
};
pub use font::{glyph, Glyph, GLYPH_HEIGHT, GLYPH_WIDTH};
