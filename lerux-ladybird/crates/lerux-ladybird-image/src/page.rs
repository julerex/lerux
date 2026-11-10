//! The Lenna test page. The `h1` text sits on a white strip above the picture.

use alloc::vec::Vec;

use lerux_ladybird_gfx::{glyph, Bitmap, Color, GLYPH_HEIGHT, GLYPH_WIDTH};
use lerux_ladybird_html::{Children, Document, Element, Node};

use crate::{decode_png, PngError};

const PAGE_HTML: &str = include_str!("../../../../support/browser/lenna.html");
/// The tokenizer drops attributes, so `src` is never stored. The page's one
/// `img` element is this file.
const PAGE_PNG: &[u8] = include_bytes!("../../../../support/browser/lenna.png");
const STRIP_ROWS: u32 = GLYPH_HEIGHT.unsigned_abs();

/// Bit position of each 8-bit channel in a little-endian 32-bit pixel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Shifts {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// Why [`paint_page`] wrote nothing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageError {
    /// The document has no `h1` or no `img`.
    Document,
    /// The picture did not decode.
    Png(PngError),
    /// The buffer, pitch, or shifts cannot hold the page.
    Frame,
}

/// Paint the Lenna page at the top left of `frame`.
///
/// The page is as wide as the picture. The `h1` text is black on a white
/// strip `GLYPH_HEIGHT` rows tall, and the picture starts on the next row.
/// `pitch` is the byte distance between rows. Pixels outside the page keep
/// their bytes.
///
/// # Errors
///
/// Returns [`PageError`] before writing any pixel.
pub fn paint_page(frame: &mut [u8], pitch: usize, shifts: Shifts) -> Result<(), PageError> {
    let document = Document::parse(PAGE_HTML);
    let heading = find(document.children(), "h1").ok_or(PageError::Document)?;
    find(document.children(), "img").ok_or(PageError::Document)?;
    let image = decode_png(PAGE_PNG).map_err(PageError::Png)?;
    let height = image
        .height()
        .checked_add(STRIP_ROWS)
        .ok_or(PageError::Frame)?;
    let mut target = Target::new(frame, pitch, shifts, image.width(), height)?;
    let strip = heading_strip(&heading, image.width())?;
    target.push_rows(&strip);
    target.push_rows(image.pixels());
    Ok(())
}

fn find<'a>(mut nodes: Children<'a>, name: &str) -> Option<Element<'a>> {
    nodes.find_map(|node| match node {
        Node::Element(element) if element.name() == name => Some(element),
        Node::Element(element) => find(element.children(), name),
        Node::Text(_) => None,
    })
}

fn text(node: Node<'_>) -> Option<&str> {
    match node {
        Node::Text(text) => Some(text.data()),
        Node::Element(_) => None,
    }
}

/// B, G, R, A rows of the white strip with the heading's glyphs in black.
fn heading_strip(heading: &Element<'_>, width: u32) -> Result<Vec<u8>, PageError> {
    let pixels = usize::try_from(width.saturating_mul(STRIP_ROWS)).map_err(|_| PageError::Frame)?;
    let mut bytes = Color::WHITE.to_bytes().repeat(pixels);
    let mut strip = Bitmap::new(&mut bytes, width, STRIP_ROWS).ok_or(PageError::Frame)?;
    let mut x = 0;
    for ch in heading.children().filter_map(text).flat_map(str::chars) {
        if let Some(cell) = glyph(ch) {
            cell.paint(&mut strip, x, 0, Color::BLACK);
        }
        x = x.saturating_add(GLYPH_WIDTH);
    }
    Ok(bytes)
}

/// A caller frame already checked to hold the whole page. Rows are written
/// top to bottom.
struct Target<'a> {
    bytes: &'a mut [u8],
    pitch: usize,
    shifts: Shifts,
    row_bytes: usize,
    next_row: usize,
}

impl<'a> Target<'a> {
    fn new(
        bytes: &'a mut [u8],
        pitch: usize,
        shifts: Shifts,
        width: u32,
        height: u32,
    ) -> Result<Self, PageError> {
        let row_bytes = usize::try_from(width)
            .ok()
            .and_then(|width| width.checked_mul(4))
            .ok_or(PageError::Frame)?;
        let needed = height
            .checked_sub(1)
            .and_then(|last| usize::try_from(last).ok())
            .and_then(|last| last.checked_mul(pitch))
            .and_then(|start| start.checked_add(row_bytes))
            .ok_or(PageError::Frame)?;
        let shifts_fit = [shifts.red, shifts.green, shifts.blue]
            .iter()
            .all(|&shift| shift <= 24);
        if pitch < row_bytes || bytes.len() < needed || !shifts_fit {
            return Err(PageError::Frame);
        }
        Ok(Self {
            bytes,
            pitch,
            shifts,
            row_bytes,
            next_row: 0,
        })
    }

    /// Write B, G, R, A rows below the rows already written.
    fn push_rows(&mut self, bgra: &[u8]) {
        for row in bgra.chunks_exact(self.row_bytes) {
            let start = self.next_row * self.pitch;
            let out = &mut self.bytes[start..start + self.row_bytes];
            for (pixel, bgra) in out.chunks_exact_mut(4).zip(row.chunks_exact(4)) {
                pixel.copy_from_slice(&pack(bgra, self.shifts).to_le_bytes());
            }
            self.next_row += 1;
        }
    }
}

fn pack(bgra: &[u8], shifts: Shifts) -> u32 {
    (u32::from(bgra[2]) << shifts.red)
        | (u32::from(bgra[1]) << shifts.green)
        | (u32::from(bgra[0]) << shifts.blue)
}
