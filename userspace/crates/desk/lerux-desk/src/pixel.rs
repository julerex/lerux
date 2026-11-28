//! 8×16 text on the firmware framebuffer.
//!
//! The shell band is 240 columns by [`CONSOLE_ROWS`] rows. Device programs paint
//! on the rows below that band, in the same gray-on-black glyphs.

use crate::{
    font::glyph,
    palette::{vga_rgb, DEFAULT_ATTR},
};

/// Glyph width in pixels.
pub(crate) const CELL_W: usize = 8;
/// Glyph height in pixels. One cell is [`CELL_W`] by this many pixels.
pub(crate) const CELL_H: usize = 16;
/// Columns of the Swift shell band.
pub const CONSOLE_COLS: usize = 240;
/// Rows of the Swift shell band. Device text starts at the next row.
pub const CONSOLE_ROWS: usize = 40;

/// Command-word row. It is the first row below the shell band.
pub const ROW_CMD: usize = CONSOLE_ROWS;
/// Exclusion-range row.
pub const ROW_EXCL: usize = ROW_CMD + 1;
/// First of the six Wi-Fi rows. The stage line is the last of them.
pub const ROW_WIFI: usize = ROW_EXCL + 1;
/// Slot, command, descriptor, doorbell, fetch, and the stage line.
pub(crate) const WIFI_ROWS: usize = 6;
/// Disk row.
pub const ROW_NVME: usize = ROW_WIFI + WIFI_ROWS;
/// Bluetooth row.
pub const ROW_BT: usize = ROW_NVME + 1;
/// First device-name row.
pub const ROW_PCI: usize = ROW_BT + 1;

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

/// Fill the mapped frame with black.
pub fn fill_background(frame: &mut [u8], pen: Pen) {
    if pen.pitch < 4 {
        return;
    }
    let bg = pack(pen, [0, 0, 0]).to_le_bytes();
    let rows = pen.height.min(frame.len() / pen.pitch);
    for y in 0..rows {
        let start = y * pen.pitch;
        let row = &mut frame[start..start + pen.pitch];
        for pixel in row.chunks_exact_mut(4) {
            pixel.copy_from_slice(&bg);
        }
    }
}

/// Draw `text` on one cell row in the default gray on black. Bytes past the row are skipped.
pub fn paint_text_row(frame: &mut [u8], pen: Pen, row: usize, text: &[u8]) {
    for (col, &ch) in text.iter().enumerate() {
        paint_cell(frame, pen, col, row, ch, DEFAULT_ATTR);
    }
}

/// Draw one VGA cell. `attr` is the hardware attribute byte. Cells past the frame are skipped.
pub fn paint_cell(frame: &mut [u8], pen: Pen, col: usize, row: usize, ch: u8, attr: u8) {
    blit(frame, pen, col, row, glyph(ch), attr);
}

/// Two-pixel underline in the cell's foreground colour. The glyph above it stays.
pub fn paint_underline(frame: &mut [u8], pen: Pen, col: usize, row: usize, attr: u8) {
    let fg = pack(pen, vga_rgb(attr & 0x0f));
    let origin_x = col * CELL_W;
    let origin_y = row * CELL_H;
    if pen.pitch < 4 || origin_x + CELL_W > pen.width || origin_y + CELL_H > pen.height {
        return;
    }
    for gy in (CELL_H - 2)..CELL_H {
        for gx in 0..CELL_W {
            let at = (origin_y + gy) * pen.pitch + (origin_x + gx) * 4;
            let Some(slot) = frame.get_mut(at..at + 4) else {
                return;
            };
            slot.copy_from_slice(&fg.to_le_bytes());
        }
    }
}

fn blit(frame: &mut [u8], pen: Pen, col: usize, row: usize, rows: [u8; CELL_H], attr: u8) {
    let fg = pack(pen, vga_rgb(attr & 0x0f));
    let bg = pack(pen, vga_rgb(attr >> 4));
    let origin_x = col * CELL_W;
    let origin_y = row * CELL_H;
    if pen.pitch < 4 || origin_x + CELL_W > pen.width || origin_y + CELL_H > pen.height {
        return;
    }
    for (gy, bits) in rows.iter().enumerate() {
        for gx in 0..CELL_W {
            let on = bits & (0x80u8 >> gx) != 0;
            let color = if on { fg } else { bg };
            let x = origin_x + gx;
            let y = origin_y + gy;
            let at = y * pen.pitch + x * 4;
            let Some(slot) = frame.get_mut(at..at + 4) else {
                return;
            };
            slot.copy_from_slice(&color.to_le_bytes());
        }
    }
}

fn pack(pen: Pen, rgb: [u8; 3]) -> u32 {
    (u32::from(rgb[0]) << pen.red)
        | (u32::from(rgb[1]) << pen.green)
        | (u32::from(rgb[2]) << pen.blue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::glyph;

    fn pen(width: usize, height: usize) -> Pen {
        Pen {
            pitch: width * 4,
            width,
            height,
            red: 16,
            green: 8,
            blue: 0,
        }
    }

    fn gray() -> [u8; 4] {
        pack(
            Pen {
                pitch: 4,
                width: 1,
                height: 1,
                red: 16,
                green: 8,
                blue: 0,
            },
            [0xaa, 0xaa, 0xaa],
        )
        .to_le_bytes()
    }

    #[test]
    fn fill_background_paints_black() {
        let mut frame = [0xffu8; 16];
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
        assert_eq!(frame, [0u8; 16]);
    }

    #[test]
    fn default_glyph_ink_is_gray() {
        let mut frame = [0u8; 16 * 32];
        paint_cell(&mut frame, pen(8, 16), 0, 0, b'A', DEFAULT_ATTR);
        // UniVGA 'A' row 2 is 0x10, so column 3 is ink.
        let at = 2 * 32 + 3 * 4;
        assert_eq!(glyph(b'A')[2], 0x10);
        assert_eq!(&frame[at..at + 4], &gray());
        assert_eq!(frame[0], 0x00);
    }

    #[test]
    fn capital_a_is_taller_than_lowercase() {
        let mut upper = [0u8; 16 * 32];
        let mut lower = [0u8; 16 * 32];
        paint_cell(&mut upper, pen(8, 16), 0, 0, b'A', DEFAULT_ATTR);
        paint_cell(&mut lower, pen(8, 16), 0, 0, b'a', DEFAULT_ATTR);
        let at = 2 * 32 + 3 * 4;
        assert_eq!(&upper[at..at + 4], &gray());
        assert_eq!(&lower[at..at + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn device_rows_follow_the_shell_band() {
        let rows = [
            ROW_CMD,
            ROW_EXCL,
            ROW_WIFI,
            ROW_WIFI + 1,
            ROW_WIFI + 2,
            ROW_WIFI + 3,
            ROW_WIFI + 4,
            ROW_WIFI + 5,
            ROW_NVME,
            ROW_BT,
            ROW_PCI,
        ];
        assert!(rows[0] >= CONSOLE_ROWS);
        for pair in rows.windows(2) {
            assert!(pair[0] < pair[1]);
        }
        assert_eq!(ROW_WIFI + WIFI_ROWS, ROW_NVME);
        let rows = usize::try_from(crate::FRAMEBUFFER_HEIGHT).expect("height") / CELL_H;
        assert!(ROW_PCI + 6 <= rows);
    }
}
