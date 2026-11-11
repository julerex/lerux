//! Stand-in for Ladybird `Libraries/LibGfx`.
//!
//! The frame is 320 by 64, tightly packed, little-endian B, G, R, A.
//! [`Bitmap`] fills a rectangle, blits a rectangle, sets one pixel, and
//! samples the first pixel that is not opaque white. The bitmap font covers
//! the fixture strings `lerux`, `lerux-http-fixture`, and `Lenna`. One set
//! bit is one pixel.
//!
//! [`ControlPage`] is the shared control region: a state, a title length, and
//! a title buffer. The bitmap is a separate region of [`FRAME_BYTES`] bytes.
//! `web_content` paints the bitmap and writes the title. `browser_ui` reads both.

#![no_std]

mod bitmap;
mod control;
mod font;

pub use bitmap::{Bitmap, Color, PixelLine, Rect, FRAME_BYTES, FRAME_HEIGHT, FRAME_WIDTH};
pub use control::{
    control_page_zeroed, ControlPage, BITMAP_VADDR, BROWSER_UI_GPU_CHANNEL,
    BROWSER_UI_PAINT_CHANNEL, CONTROL_REGION_SIZE, CONTROL_STATE_IDLE, CONTROL_STATE_PAINTED,
    CONTROL_VADDR, TITLE_CAP, WEB_CONTENT_PAINT_CHANNEL,
};
pub use font::{glyph, Glyph, GLYPH_HEIGHT, GLYPH_WIDTH};
