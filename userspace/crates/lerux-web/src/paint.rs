//! Paint layout boxes into an XRGB8888 bitmap (`0x00RRGGBB`).

use alloc::vec::Vec;

use crate::{font::glyph, layout::LayoutBox, style::WHITE};

/// Shared bitmap view. Bytes are little-endian XRGB8888.
pub struct Bitmap<'a> {
    data: &'a mut [u8],
    width: u32,
    height: u32,
    stride: u32,
}

impl<'a> Bitmap<'a> {
    /// `data` must cover `stride * height` bytes; `stride` is at least `width * 4`.
    pub fn new(data: &'a mut [u8], width: u32, height: u32, stride: u32) -> Option<Self> {
        let need = stride as usize * height as usize;
        if width == 0 || height == 0 || stride < width.saturating_mul(4) || data.len() < need {
            return None;
        }
        Some(Self {
            data,
            width,
            height,
            stride,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Native-endian XRGB8888 pixel, or 0 if out of bounds.
    pub fn pixel(&self, x: u32, y: u32) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        let off = (y * self.stride + x * 4) as usize;
        u32::from_le_bytes(self.data[off..off + 4].try_into().unwrap_or([0; 4]))
    }

    pub fn contains_color(&self, color: u32) -> bool {
        for y in 0..self.height {
            for x in 0..self.width {
                if self.pixel(x, y) == color {
                    return true;
                }
            }
        }
        false
    }

    /// Binary PPM (`P6`) of the visible RGB pixels.
    pub fn encode_ppm(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let header = alloc::format!("P6\n{} {}\n255\n", self.width, self.height);
        out.extend_from_slice(header.as_bytes());
        for y in 0..self.height {
            for x in 0..self.width {
                let px = self.pixel(x, y);
                out.push(((px >> 16) & 0xFF) as u8);
                out.push(((px >> 8) & 0xFF) as u8);
                out.push((px & 0xFF) as u8);
            }
        }
        out
    }

    fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x < 0 || y < 0 {
            return;
        }
        let x = x as u32;
        let y = y as u32;
        if x >= self.width || y >= self.height {
            return;
        }
        let off = (y * self.stride + x * 4) as usize;
        self.data[off..off + 4].copy_from_slice(&color.to_le_bytes());
    }

    fn fill(&mut self, color: u32) {
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                self.set_pixel(x, y, color);
            }
        }
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        if w <= 0 || h <= 0 {
            return;
        }
        let x1 = x.saturating_add(w);
        let y1 = y.saturating_add(h);
        for py in y..y1 {
            for px in x..x1 {
                self.set_pixel(px, py, color);
            }
        }
    }

    fn paint_text(&mut self, x: i32, y: i32, text: &str, color: u32, font_size: i32) {
        let size = font_size.max(1);
        let mut cx = x;
        for ch in text.chars() {
            let g = glyph(if ch.is_ascii() { ch as u8 } else { b'?' });
            for gy in 0i32..8 {
                for gx in 0i32..8 {
                    if g[gy as usize] & (0x80 >> gx) == 0 {
                        continue;
                    }
                    let px0 = gx * size / 8;
                    let py0 = gy * size / 8;
                    let px1 = ((gx + 1) * size / 8).max(px0 + 1);
                    let py1 = ((gy + 1) * size / 8).max(py0 + 1);
                    for oy in py0..py1 {
                        for ox in px0..px1 {
                            self.set_pixel(cx + ox, y + oy, color);
                        }
                    }
                }
            }
            cx += size;
        }
    }
}

pub(crate) fn paint(root: &LayoutBox, bitmap: &mut Bitmap<'_>) {
    bitmap.fill(WHITE);
    paint_box(root, bitmap);
}

fn paint_box(b: &LayoutBox, bitmap: &mut Bitmap<'_>) {
    if let Some(bg) = b.background {
        bitmap.fill_rect(b.x, b.y, b.width, b.height, bg);
    }
    for run in &b.runs {
        bitmap.paint_text(run.x, run.y, &run.text, run.color, run.font_size);
    }
    for r in &b.replaced {
        bitmap.fill_rect(r.x, r.y, r.width, r.height, r.color);
    }
    for child in &b.children {
        paint_box(child, bitmap);
    }
}
