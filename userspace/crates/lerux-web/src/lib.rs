//! CSS subset, block-flow layout, and RGB888 paint for `web-content` (Phase 75).
//!
//! `#![no_std]` + `alloc`. Not Servo, not Ladybird LibWeb. Supported properties:
//! `color`, `background-color`, `font-size`, `display: block|inline|none`,
//! `margin`, `padding`, `width`. Author `<style>` plus a tiny UA sheet.

#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

mod css;
mod font;
mod layout;
mod paint;
mod style;

pub use paint::Bitmap;

use layout::layout_document;
use paint::paint as paint_tree;

/// Committed paint fixture (`support/browser/paint.html`).
pub const PAINT_FIXTURE: &str = include_str!("../../../../support/browser/paint.html");

/// Parse, lay out, and paint `html` into `bitmap`.
pub fn render(html: &str, bitmap: &mut Bitmap<'_>) {
    let doc = lerux_html::parse(html);
    let tree = layout_document(&doc, bitmap.width() as i32, bitmap.height() as i32);
    paint_tree(&tree, bitmap);
}

/// True when [`PAINT_FIXTURE`] produced the expected signature pixels.
///
/// Layout: 32px red `h1`, then a 200px-content yellow `p` with 8px padding
/// (padding box starts at y=32). Span text is green.
pub fn fixture_signatures_ok(bitmap: &Bitmap<'_>) -> bool {
    const YELLOW: u32 = 0x00FF_FF00;
    const RED: u32 = 0x00CC_0000;
    const BLUE: u32 = 0x0000_3399;
    const GREEN: u32 = 0x0000_6600;
    const WHITE: u32 = 0x00FF_FFFF;
    bitmap.width() >= 800
        && bitmap.height() >= 600
        && bitmap.pixel(4, 36) == YELLOW
        && bitmap.pixel(799, 0) == WHITE
        && bitmap.contains_color(RED)
        && bitmap.contains_color(BLUE)
        && bitmap.contains_color(GREEN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{vec, vec::Vec};

    fn browser_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../support/browser")
    }

    fn paint(html: &str, w: u32, h: u32) -> Vec<u8> {
        let stride = w * 4;
        let mut data = vec![0u8; (stride * h) as usize];
        {
            let mut bmp = Bitmap::new(&mut data, w, h, stride).expect("bitmap");
            render(html, &mut bmp);
        }
        data
    }

    fn pixel(data: &[u8], w: u32, x: u32, y: u32) -> u32 {
        let off = (y * w * 4 + x * 4) as usize;
        u32::from_le_bytes(data[off..off + 4].try_into().unwrap())
    }

    fn contains(data: &[u8], color: u32) -> bool {
        data.chunks_exact(4)
            .any(|c| u32::from_le_bytes(c.try_into().unwrap()) == color)
    }

    #[test]
    fn paint_fixture_on_disk_matches_include_str() {
        let src = std::fs::read_to_string(browser_dir().join("paint.html")).expect("fixture");
        assert_eq!(src, PAINT_FIXTURE);
    }

    #[test]
    fn render_should_fill_inline_background() {
        let data = paint(
            "<body style=\"margin:0;padding:8px;background-color:#ff0000\">x</body>",
            64,
            32,
        );
        assert_eq!(pixel(&data, 64, 0, 0), 0x00FF_0000);
    }

    #[test]
    fn render_should_skip_display_none() {
        let data = paint(
            "<body style=\"margin:0;color:#0000ff\"><title>hid</title>Hi</body>",
            128,
            32,
        );
        assert!(contains(&data, 0x0000_00FF), "body text should be blue");
    }

    #[test]
    fn render_should_paint_signature_pixels_on_fixture() {
        let mut data = paint(PAINT_FIXTURE, 800, 600);
        let bmp = Bitmap::new(&mut data, 800, 600, 800 * 4).expect("bitmap");
        assert!(
            fixture_signatures_ok(&bmp),
            "yellow={} white={} has_red={} has_blue={} has_green={}",
            bmp.pixel(4, 36),
            bmp.pixel(799, 0),
            bmp.contains_color(0x00CC_0000),
            bmp.contains_color(0x0000_3399),
            bmp.contains_color(0x0000_6600),
        );
    }

    #[test]
    fn encode_ppm_should_write_p6_header_and_payload() {
        let mut data = paint(PAINT_FIXTURE, 800, 600);
        let bmp = Bitmap::new(&mut data, 800, 600, 800 * 4).expect("bitmap");
        let ppm = bmp.encode_ppm();
        assert!(ppm.starts_with(b"P6\n800 600\n255\n"));
        let header = b"P6\n800 600\n255\n".len();
        assert_eq!(ppm.len(), header + 800 * 600 * 3);
    }

    #[test]
    fn subset_fixture_should_layout_without_panic() {
        let src = std::fs::read_to_string(browser_dir().join("subset.html")).expect("subset");
        let _ = paint(&src, 400, 300);
    }
}
