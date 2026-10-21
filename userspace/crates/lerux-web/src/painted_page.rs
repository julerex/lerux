//! Fixed scanout frame for the browser image.
//!
//! Width, height, and byte count live here. The buffer is tightly packed:
//! stride is `WIDTH * 4`, and there is no stride field to set. Bytes are
//! little-endian B, G, R, A after [`set_opaque`]. The graphics driver copies
//! them with no swizzle.

use alloc::{vec, vec::Vec};

use crate::render;

/// Serial spelling of the sampled pixel. Fifteen bytes: `pixel `, eight
/// lowercase hex digits, `\n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelLine {
    bytes: [u8; 15],
}

impl PixelLine {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Why [`PaintedPage::render`] produced no frame.
#[derive(Debug, PartialEq, Eq)]
pub enum FrameError {
    /// `html` is not UTF-8.
    Utf8,
    /// Every pixel is opaque white, so there is no ink to sample.
    Blank,
}

/// The rendered document and the pixel the serial line names.
///
/// Constructed only by [`Self::render`].
pub struct PaintedPage {
    bytes: Vec<u8>,
    line: PixelLine,
}

impl PaintedPage {
    pub const WIDTH: u32 = 320;
    pub const HEIGHT: u32 = 64;
    pub const BYTES: usize = (Self::WIDTH as usize) * (Self::HEIGHT as usize) * 4;

    /// Parse, lay out at [`Self::WIDTH`] by [`Self::HEIGHT`], paint, set alpha.
    ///
    /// The sample is the first pixel that is not opaque white, in row-major
    /// order, taken after alpha is set.
    pub fn render(html: &[u8]) -> Result<Self, FrameError> {
        let text = core::str::from_utf8(html).map_err(|_| FrameError::Utf8)?;
        let mut bytes = vec![0u8; Self::BYTES];
        let mut bitmap = crate::Bitmap::new(&mut bytes, Self::WIDTH, Self::HEIGHT, Self::WIDTH * 4)
            .ok_or(FrameError::Blank)?;
        render(text, &mut bitmap);
        set_opaque(&mut bytes);
        let line = sample_line(&bytes).ok_or(FrameError::Blank)?;
        Ok(Self { bytes, line })
    }

    pub fn width(&self) -> u32 {
        Self::WIDTH
    }

    pub fn height(&self) -> u32 {
        Self::HEIGHT
    }

    /// Tightly packed BGRA. Length is [`Self::BYTES`].
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn pixel_line(&self) -> &PixelLine {
        &self.line
    }
}

fn set_opaque(bytes: &mut [u8]) {
    for i in (0..bytes.len()).step_by(4) {
        bytes[i + 3] = 0xff;
    }
}

const OPAQUE_WHITE: [u8; 4] = [0xff, 0xff, 0xff, 0xff];

fn sample_line(bytes: &[u8]) -> Option<PixelLine> {
    for chunk in bytes.chunks_exact(4) {
        if chunk == OPAQUE_WHITE {
            continue;
        }
        let mut out = [0u8; 15];
        out[..6].copy_from_slice(b"pixel ");
        write_hex_byte(&mut out[6..8], chunk[0]);
        write_hex_byte(&mut out[8..10], chunk[1]);
        write_hex_byte(&mut out[10..12], chunk[2]);
        write_hex_byte(&mut out[12..14], chunk[3]);
        out[14] = b'\n';
        return Some(PixelLine { bytes: out });
    }
    None
}

fn write_hex_byte(dst: &mut [u8], byte: u8) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    dst[0] = HEX[(byte >> 4) as usize];
    dst[1] = HEX[(byte & 0x0f) as usize];
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn browser_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../support/browser")
    }

    fn fixture_html() -> Vec<u8> {
        std::fs::read(browser_dir().join("fixture.html")).expect("fixture")
    }

    fn pixel_bgra(data: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let off = ((y * width + x) * 4) as usize;
        data[off..off + 4].try_into().expect("pixel")
    }

    fn ink_bounds(data: &[u8], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut any = false;
        for y in 0..height {
            for x in 0..width {
                if pixel_bgra(data, width, x, y) != OPAQUE_WHITE {
                    any = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
        any.then_some((min_x, min_y, max_x, max_y))
    }

    fn render_raw(html: &str, w: u32, h: u32) -> Vec<u8> {
        let stride = w * 4;
        let mut bytes = vec![0u8; (stride * h) as usize];
        let mut bitmap = crate::Bitmap::new(&mut bytes, w, h, stride).expect("bitmap");
        render(html, &mut bitmap);
        set_opaque(&mut bytes);
        bytes
    }

    #[test]
    fn fixture_should_sample_first_black_pixel() {
        let page = PaintedPage::render(&fixture_html()).expect("fixture");
        assert_eq!(page.pixel_line().as_bytes(), b"pixel 000000ff\n");
        assert_eq!(page.width(), 320);
        assert_eq!(page.height(), 64);
        assert_eq!(page.bytes().len(), PaintedPage::BYTES);
    }

    #[test]
    fn fixture_top_margin_should_be_opaque_white() {
        let page = PaintedPage::render(&fixture_html()).expect("fixture");
        for y in 0..24 {
            for x in 0..page.width() {
                assert_eq!(
                    pixel_bgra(page.bytes(), page.width(), x, y),
                    OPAQUE_WHITE,
                    "row {y} col {x}"
                );
            }
        }
        let mut found_ink_below = false;
        for y in 24..page.height() {
            for x in 0..page.width() {
                if pixel_bgra(page.bytes(), page.width(), x, y) == [0x00, 0x00, 0x00, 0xff] {
                    found_ink_below = true;
                    break;
                }
            }
            if found_ink_below {
                break;
            }
        }
        assert!(
            found_ink_below,
            "paragraph ink should sit below the top margin"
        );
    }

    #[test]
    fn set_opaque_should_turn_xrgb_white_into_bgra_opaque_white() {
        let html = r#"<body style="margin:0;background:#ffffff">x</body>"#;
        let page = PaintedPage::render(html.as_bytes()).expect("white page with text");
        assert_eq!(pixel_bgra(page.bytes(), page.width(), 0, 0), OPAQUE_WHITE);
    }

    #[test]
    fn public_frame_ink_box_should_match_taller_reference_layout() {
        let fixture = fixture_html();
        let html = core::str::from_utf8(&fixture).expect("utf8");
        let public = PaintedPage::render(&fixture).expect("public size");
        let reference = render_raw(html, 320, 128);
        let public_box = ink_bounds(public.bytes(), PaintedPage::WIDTH, PaintedPage::HEIGHT)
            .expect("fixture ink");
        let reference_box = ink_bounds(&reference, 320, 128).expect("reference ink");
        assert_eq!(public_box, reference_box);
    }
}
