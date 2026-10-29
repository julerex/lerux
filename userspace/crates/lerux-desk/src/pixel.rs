//! 8×8 glyphs for a firmware framebuffer. Two pixels per font bit.

const SCALE: usize = 2;

struct Glyph {
    ch: u8,
    row: [u8; 8],
}

const GLYPHS: &[Glyph] = &[
    Glyph {
        ch: b' ',
        row: [0, 0, 0, 0, 0, 0, 0, 0],
    },
    Glyph {
        ch: b'0',
        row: [0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0],
    },
    Glyph {
        ch: b'1',
        row: [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7E, 0],
    },
    Glyph {
        ch: b'2',
        row: [0x3C, 0x66, 0x06, 0x0C, 0x18, 0x30, 0x7E, 0],
    },
    Glyph {
        ch: b'3',
        row: [0x3C, 0x66, 0x06, 0x1C, 0x06, 0x66, 0x3C, 0],
    },
    Glyph {
        ch: b'4',
        row: [0x0C, 0x1C, 0x3C, 0x6C, 0x7E, 0x0C, 0x0C, 0],
    },
    Glyph {
        ch: b'5',
        row: [0x7E, 0x60, 0x7C, 0x06, 0x06, 0x66, 0x3C, 0],
    },
    Glyph {
        ch: b'6',
        row: [0x3C, 0x66, 0x60, 0x7C, 0x66, 0x66, 0x3C, 0],
    },
    Glyph {
        ch: b'7',
        row: [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x30, 0x30, 0],
    },
    Glyph {
        ch: b'8',
        row: [0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x3C, 0],
    },
    Glyph {
        ch: b'9',
        row: [0x3C, 0x66, 0x66, 0x3E, 0x06, 0x66, 0x3C, 0],
    },
    Glyph {
        ch: b'a',
        row: [0x00, 0x00, 0x3C, 0x06, 0x3E, 0x66, 0x3E, 0],
    },
    Glyph {
        ch: b'b',
        row: [0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0],
    },
    Glyph {
        ch: b'c',
        row: [0x00, 0x00, 0x3C, 0x66, 0x60, 0x66, 0x3C, 0],
    },
    Glyph {
        ch: b'd',
        row: [0x06, 0x06, 0x3E, 0x66, 0x66, 0x66, 0x3E, 0],
    },
    Glyph {
        ch: b'e',
        row: [0x00, 0x00, 0x3C, 0x66, 0x7E, 0x60, 0x3C, 0],
    },
    Glyph {
        ch: b'f',
        row: [0x1C, 0x36, 0x30, 0x78, 0x30, 0x30, 0x30, 0],
    },
    Glyph {
        ch: b'g',
        row: [0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x7C],
    },
    Glyph {
        ch: b'h',
        row: [0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0],
    },
    Glyph {
        ch: b'i',
        row: [0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x3C, 0],
    },
    Glyph {
        ch: b'l',
        row: [0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0],
    },
    Glyph {
        ch: b'm',
        row: [0x00, 0x00, 0x66, 0x7F, 0x7F, 0x6B, 0x63, 0],
    },
    Glyph {
        ch: b'n',
        row: [0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x66, 0],
    },
    Glyph {
        ch: b'o',
        row: [0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x3C, 0],
    },
    Glyph {
        ch: b'q',
        row: [0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x06],
    },
    Glyph {
        ch: b'r',
        row: [0x00, 0x00, 0x6C, 0x76, 0x60, 0x60, 0x60, 0],
    },
    Glyph {
        ch: b's',
        row: [0x00, 0x00, 0x3E, 0x60, 0x3C, 0x06, 0x7C, 0],
    },
    Glyph {
        ch: b't',
        row: [0x30, 0x30, 0x7C, 0x30, 0x30, 0x36, 0x1C, 0],
    },
    Glyph {
        ch: b'u',
        row: [0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x3E, 0],
    },
    Glyph {
        ch: b'v',
        row: [0x00, 0x00, 0x66, 0x66, 0x66, 0x3C, 0x18, 0],
    },
    Glyph {
        ch: b'w',
        row: [0x00, 0x00, 0x63, 0x6B, 0x7F, 0x3E, 0x36, 0],
    },
    Glyph {
        ch: b'x',
        row: [0x00, 0x00, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0],
    },
    Glyph {
        ch: b'y',
        row: [0x00, 0x00, 0x66, 0x66, 0x66, 0x3E, 0x06, 0x7C],
    },
    Glyph {
        ch: b'>',
        row: [0xC0, 0x60, 0x30, 0x18, 0x30, 0x60, 0xC0, 0],
    },
    Glyph {
        ch: b'k',
        row: [0x60, 0x60, 0x66, 0x6C, 0x78, 0x6C, 0x66, 0],
    },
    Glyph {
        ch: b'p',
        row: [0x00, 0x00, 0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60],
    },
    Glyph {
        ch: b'.',
        row: [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0],
    },
    Glyph {
        ch: b':',
        row: [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0],
    },
    Glyph {
        ch: b'-',
        row: [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0],
    },
];

fn glyph(ch: u8) -> [u8; 8] {
    let ch = if ch.is_ascii_uppercase() {
        ch.to_ascii_lowercase()
    } else {
        ch
    };
    for item in GLYPHS {
        if item.ch == ch {
            return item.row;
        }
    }
    [0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x7E, 0]
}

fn pack(red: u8, green: u8, blue: u8, white: bool) -> u32 {
    if white {
        (0xffu32 << red) | (0xffu32 << green) | (0xffu32 << blue)
    } else {
        0xaau32 << blue
    }
}

/// Pixel geometry for one firmware framebuffer.
#[derive(Clone, Copy)]
pub struct Pen {
    pub pitch: usize,
    pub width: usize,
    pub height: usize,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// Fill the mapped frame with the console blue. Cells paint over this.
pub fn fill_background(frame: &mut [u8], pen: Pen) {
    if pen.pitch < 4 {
        return;
    }
    let bg = pack(pen.red, pen.green, pen.blue, false).to_le_bytes();
    let rows = pen.height.min(frame.len() / pen.pitch);
    for y in 0..rows {
        let start = y * pen.pitch;
        let row = &mut frame[start..start + pen.pitch];
        for pixel in row.chunks_exact_mut(4) {
            pixel.copy_from_slice(&bg);
        }
    }
}

/// Draw `text` on one cell row. Bytes past the row are skipped.
pub fn paint_text_row(frame: &mut [u8], pen: Pen, row: usize, text: &[u8]) {
    for (col, &ch) in text.iter().enumerate() {
        paint_cell(frame, pen, col, row, ch);
    }
}

/// Draw one VGA cell. `ch` is the low byte. Cells past the frame are skipped.
pub fn paint_cell(frame: &mut [u8], pen: Pen, col: usize, row: usize, ch: u8) {
    let fg = pack(pen.red, pen.green, pen.blue, true);
    let bg = pack(pen.red, pen.green, pen.blue, false);
    let origin_x = col * 8 * SCALE;
    let origin_y = row * 8 * SCALE;
    if pen.pitch < 4 || origin_x + 8 * SCALE > pen.width || origin_y + 8 * SCALE > pen.height {
        return;
    }
    let rows = glyph(ch);
    for (gy, bits) in rows.iter().enumerate() {
        for gx in 0..8 {
            let on = bits & (0x80u8 >> gx) != 0;
            let color = if on { fg } else { bg };
            for sy in 0..SCALE {
                for sx in 0..SCALE {
                    let x = origin_x + gx * SCALE + sx;
                    let y = origin_y + gy * SCALE + sy;
                    let at = y * pen.pitch + x * 4;
                    let Some(slot) = frame.get_mut(at..at + 4) else {
                        return;
                    };
                    slot.copy_from_slice(&color.to_le_bytes());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_background_paints_console_blue() {
        let mut frame = [0u8; 16];
        fill_background(
            &mut frame,
            Pen {
                pitch: 8,
                width: 2,
                height: 2,
                red: 16,
                green: 8,
                blue: 0,
            },
        );
        assert_eq!(&frame[..4], &0xaau32.to_le_bytes());
        assert_eq!(&frame[12..16], &0xaau32.to_le_bytes());
    }

    #[test]
    fn letter_f_has_ink() {
        let mut frame = [0u8; 64 * 32];
        paint_cell(
            &mut frame,
            Pen {
                pitch: 64,
                width: 16,
                height: 32,
                red: 16,
                green: 8,
                blue: 0,
            },
            0,
            0,
            b'f',
        );
        assert_eq!(frame[26], 0xff);
    }
}
