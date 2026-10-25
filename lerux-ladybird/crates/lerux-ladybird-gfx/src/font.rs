//! Built-in bitmap font for the fixture strings.
//!
//! The cell is 8 by 8. Bit 7 of each row is the leftmost pixel. There is no
//! scale: one set bit is one pixel. The set is the characters in `lerux` and
//! `lerux-http-fixture`.

use crate::{Bitmap, Color};

/// Cell width. Layout advances by this many pixels.
pub const GLYPH_WIDTH: i32 = 8;
/// Cell height.
pub const GLYPH_HEIGHT: i32 = 8;

const GLYPH_ROWS: usize = 8;

/// One cell. Rows are private so a caller paints through [`Glyph::paint`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Glyph {
    rows: [u8; GLYPH_ROWS],
}

struct Entry {
    ch: u8,
    rows: [u8; GLYPH_ROWS],
}

/// Sorted by code point. `l` row 0 is `0x30`, so its first ink is column 2.
const GLYPHS: &[Entry] = &[
    Entry {
        ch: b'-',
        rows: [0x00, 0x00, 0x00, 0x7C, 0x00, 0x00, 0x00, 0x00],
    },
    Entry {
        ch: b'e',
        rows: [0x00, 0x00, 0x38, 0x44, 0x7C, 0x40, 0x38, 0x00],
    },
    Entry {
        ch: b'f',
        rows: [0x08, 0x10, 0x38, 0x10, 0x10, 0x10, 0x10, 0x00],
    },
    Entry {
        ch: b'h',
        rows: [0x40, 0x40, 0x78, 0x44, 0x44, 0x44, 0x44, 0x00],
    },
    Entry {
        ch: b'i',
        rows: [0x10, 0x00, 0x30, 0x10, 0x10, 0x10, 0x38, 0x00],
    },
    Entry {
        ch: b'l',
        rows: [0x30, 0x10, 0x10, 0x10, 0x10, 0x10, 0x38, 0x00],
    },
    Entry {
        ch: b'p',
        rows: [0x00, 0x00, 0x78, 0x44, 0x78, 0x40, 0x40, 0x00],
    },
    Entry {
        ch: b'r',
        rows: [0x00, 0x00, 0x58, 0x64, 0x40, 0x40, 0x40, 0x00],
    },
    Entry {
        ch: b't',
        rows: [0x10, 0x10, 0x38, 0x10, 0x10, 0x10, 0x08, 0x00],
    },
    Entry {
        ch: b'u',
        rows: [0x00, 0x00, 0x44, 0x44, 0x44, 0x44, 0x3C, 0x00],
    },
    Entry {
        ch: b'x',
        rows: [0x00, 0x00, 0x44, 0x28, 0x10, 0x28, 0x44, 0x00],
    },
];

const _: () = assert!(GLYPHS.len() == 11);
const _: () = {
    let mut index = 1;
    while index < GLYPHS.len() {
        assert!(GLYPHS[index - 1].ch < GLYPHS[index].ch);
        index += 1;
    }
};

impl Glyph {
    /// Paint this cell with its top-left at `(x, y)`. Bits outside the frame
    /// are clipped.
    pub fn paint(self, bitmap: &mut Bitmap<'_>, x: i32, y: i32, color: Color) {
        for (gy, row) in self.rows.iter().enumerate() {
            let Ok(gy) = i32::try_from(gy) else {
                continue;
            };
            let Some(py) = y.checked_add(gy) else {
                continue;
            };
            for gx in 0..GLYPH_ROWS {
                if row & (0x80u8 >> gx) == 0 {
                    continue;
                }
                let Ok(gx) = i32::try_from(gx) else {
                    continue;
                };
                let Some(px) = x.checked_add(gx) else {
                    continue;
                };
                bitmap.set_pixel(px, py, color);
            }
        }
    }
}

/// The glyph for `ch`, when the fixture set includes it.
pub fn glyph(ch: char) -> Option<Glyph> {
    let Ok(ch) = u8::try_from(u32::from(ch)) else {
        return None;
    };
    GLYPHS
        .iter()
        .find(|entry| entry.ch == ch)
        .map(|entry| Glyph { rows: entry.rows })
}
