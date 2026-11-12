//! Fixed scanout frame for the browser image.
//!
//! Width, height, and byte count live here. The buffer is tightly packed:
//! stride is `WIDTH * 4`, and there is no stride field to set. Bytes are
//! little-endian B, G, R, A after [`set_opaque`]. The graphics driver copies
//! them with no swizzle.
//!
//! Verus checks the sampled serial line. The sample is the first 4-byte chunk
//! that is not opaque white. The line is `pixel `, eight lowercase hex digits
//! of those bytes in that order, and a newline. For the fixture that is
//! `pixel 000000ff`.
use alloc::{vec, vec::Vec};

// The verifier's expansion names `vstd` and `int`. A normal build erases that
// expansion, so the imports are unused unless `verus` is verifying. The shell
// build leaves the feature off.
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use verus_builtin::*;
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::prelude::*;

use crate::render;

verus! {

/// Serial spelling of the sampled pixel. Fifteen bytes: `pixel `, eight
/// lowercase hex digits, `\n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelLine {
    bytes: [u8; 15],
}

impl PixelLine {
    #[must_use]
    pub fn as_bytes(&self) -> (out: &[u8])
        ensures
            out@.len() == 15,
    {
        self.bytes.as_slice()
    }
}

/// One chunk of `[0xff, 0xff, 0xff, 0xff]`, the opaque-white pixel.
pub open spec fn spec_chunk_white(bytes: Seq<u8>, i: int) -> bool
    recommends
        0 <= i && i + 4 <= bytes.len(),
{
    bytes[i] == 0xffu8 && bytes[i + 1] == 0xffu8 && bytes[i + 2] == 0xffu8 && bytes[i + 3] == 0xffu8
}

/// Lowercase hex digit of one nibble. `0` is `0` and `15` is `f`.
pub open spec fn spec_hex_digit(nibble: u8) -> u8
    recommends
        nibble < 16,
{
    if nibble < 10 {
        (b'0' + nibble) as u8
    } else {
        (b'a' + (nibble - 10)) as u8
    }
}

/// `0`–`9` or `a`–`f`.
pub open spec fn is_lower_hex(digit: u8) -> bool {
    (b'0' <= digit && digit <= b'9') || (b'a' <= digit && digit <= b'f')
}

/// Every complete 4-byte chunk at `i`, `i + 4`, … is opaque white.
///
/// The scan stops when fewer than four bytes remain, so an empty buffer and a
/// buffer of only opaque white are both true.
pub open spec fn all_white_from(bytes: Seq<u8>, i: int) -> bool
    decreases
            (if 0 <= i && i + 4 <= bytes.len() {
                bytes.len() - i
            } else {
                0int
            }),
{
    if !(0 <= i && i + 4 <= bytes.len()) {
        true
    } else {
        spec_chunk_white(bytes, i) && all_white_from(bytes, i + 4)
    }
}

/// Chunks at `0`, `4`, … strictly before `end` are opaque white.
pub open spec fn skipped_white(bytes: Seq<u8>, end: int) -> bool
    decreases
            (if end > 0 {
                end
            } else {
                0int
            }),
{
    if end <= 0 {
        true
    } else {
        skipped_white(bytes, end - 4) && spec_chunk_white(bytes, end - 4)
    }
}

/// Index of the first chunk that is not opaque white, or `-1` when there is
/// no such chunk. The step is four bytes, the same groups as `chunks_exact`.
pub open spec fn spec_first_ink(bytes: Seq<u8>, i: int) -> int
    decreases
            (if 0 <= i && i + 4 <= bytes.len() {
                bytes.len() - i
            } else {
                0int
            }),
{
    if !(0 <= i && i + 4 <= bytes.len()) {
        -1
    } else if !spec_chunk_white(bytes, i) {
        i
    } else {
        spec_first_ink(bytes, i + 4)
    }
}

/// `line` is the 15-byte serial spelling of the chunk at `i`, bytes in order.
pub open spec fn spec_line_at(bytes: Seq<u8>, i: int, line: Seq<u8>) -> bool {
    &&& line.len() == 15
    &&& 0 <= i && i + 4 <= bytes.len()
    &&& !spec_chunk_white(bytes, i)
    &&& skipped_white(bytes, i)
    &&& line[0] == b'p'
    &&& line[1] == b'i'
    &&& line[2] == b'x'
    &&& line[3] == b'e'
    &&& line[4] == b'l'
    &&& line[5] == b' '
    &&& line[6] == spec_hex_digit((bytes[i] >> 4) as u8)
    &&& line[7] == spec_hex_digit((bytes[i] & 0x0fu8) as u8)
    &&& line[8] == spec_hex_digit((bytes[i + 1] >> 4) as u8)
    &&& line[9] == spec_hex_digit((bytes[i + 1] & 0x0fu8) as u8)
    &&& line[10] == spec_hex_digit((bytes[i + 2] >> 4) as u8)
    &&& line[11] == spec_hex_digit((bytes[i + 2] & 0x0fu8) as u8)
    &&& line[12] == spec_hex_digit((bytes[i + 3] >> 4) as u8)
    &&& line[13] == spec_hex_digit((bytes[i + 3] & 0x0fu8) as u8)
    &&& line[14] == b'\n'
}

proof fn lemma_hex_is_lower(nibble: u8)
    requires
        nibble < 16,
    ensures
        is_lower_hex(spec_hex_digit(nibble)),
{
    if nibble < 10 {
        assert(b'0' + nibble >= b'0' && b'0' + nibble <= b'9') by (bit_vector)
            requires
                nibble < 10,
        ;
    } else {
        assert(b'a' + (nibble - 10) >= b'a' && b'a' + (nibble - 10) <= b'f') by (bit_vector)
            requires
                nibble >= 10,
                nibble < 16,
        ;
    }
}

/// `b'0' + nibble` and `b'a' + (nibble - 10)` fit in a byte.
proof fn lemma_digit_fits(nibble: u8)
    requires
        nibble < 16,
    ensures
        nibble < 10 ==> b'0' as int + nibble as int <= 255,
        nibble >= 10 ==> nibble as int >= 10 && b'a' as int + (nibble as int - 10) <= 255,
{
    assert(nibble < 10 ==> b'0' as int + nibble as int <= 255) by (bit_vector)
        requires
            nibble < 16,
    ;
    assert(nibble >= 10 ==> nibble as int >= 10 && b'a' as int + (nibble as int - 10) <= 255)
        by (bit_vector)
        requires
            nibble < 16,
    ;
}

/// The two nibbles are below 16, and the three sample bytes spell `00`, `ff`,
/// and `0a`.
proof fn lemma_nibbles(byte: u8, high: u8, low: u8)
    requires
        high == byte >> 4,
        low == byte & 0x0fu8,
    ensures
        high < 16,
        low < 16,
        high == (byte >> 4) as u8,
        low == (byte & 0x0fu8) as u8,
        byte == 0x00 ==> spec_hex_digit(high) == b'0' && spec_hex_digit(low) == b'0',
        byte == 0xff ==> spec_hex_digit(high) == b'f' && spec_hex_digit(low) == b'f',
        byte == 0x0a ==> spec_hex_digit(high) == b'0' && spec_hex_digit(low) == b'a',
{
    assert(high == (byte >> 4) as u8 && low == (byte & 0x0fu8) as u8 && high < 16 && low < 16)
        by (bit_vector)
        requires
            high == byte >> 4,
            low == byte & 0x0fu8,
    ;
    assert(byte == 0x00 ==> high == 0 && low == 0) by (bit_vector)
        requires
            high == byte >> 4,
            low == byte & 0x0fu8,
    ;
    assert(byte == 0xff ==> high == 15 && low == 15) by (bit_vector)
        requires
            high == byte >> 4,
            low == byte & 0x0fu8,
    ;
    assert(byte == 0x0a ==> high == 0 && low == 10) by (bit_vector)
        requires
            high == byte >> 4,
            low == byte & 0x0fu8,
    ;
    assert(b'0' + 0u8 == b'0' && b'a' + 0u8 == b'a' && b'a' + 5u8 == b'f') by (bit_vector);
    assert((10u8 - 10) == 0 && (15u8 - 10) == 5) by (bit_vector);
}

proof fn lemma_chunk_bounds(len: usize, i: usize)
    requires
        i <= len,
        len - i >= 4,
    ensures
        i + 3 < len,
        i + 4 <= len,
{
    assert(i + 3 < len && i + 4 <= len) by (bit_vector)
        requires
            i <= len,
            len - i >= 4,
    ;
}

/// The next chunk is still the first ink, and the skipped prefix grows by one
/// white chunk.
proof fn lemma_skip_white(bytes: &[u8], i: usize)
    requires
        i <= bytes.len(),
        bytes.len() - i >= 4,
        spec_chunk_white(bytes@, i as int),
        skipped_white(bytes@, i as int),
    ensures
        i + 4 <= bytes.len(),
        (i + 4) as int == i as int + 4,
        bytes.len() - (i + 4) < bytes.len() - i,
        (i + 4) as int == i as int + 4,
        skipped_white(bytes@, i as int + 4),
        all_white_from(bytes@, i as int) == all_white_from(bytes@, i as int + 4),
        spec_first_ink(bytes@, i as int) == spec_first_ink(bytes@, i as int + 4),
{
    let len = bytes.len();
    assert(i + 4 <= len && len - (i + 4) < len - i) by (bit_vector)
        requires
            i <= len,
            len - i >= 4,
    ;
    assert((i + 4) as int == i as int + 4);
    assert(skipped_white(bytes@, i as int + 4) == (skipped_white(bytes@, i as int)
        && spec_chunk_white(bytes@, i as int)));
    assert(all_white_from(bytes@, i as int) == (spec_chunk_white(bytes@, i as int)
        && all_white_from(bytes@, i as int + 4)));
    assert(spec_first_ink(bytes@, i as int) == spec_first_ink(bytes@, i as int + 4));
}

proof fn lemma_found_ink(bytes: &[u8], i: usize)
    requires
        i <= bytes.len(),
        bytes.len() - i >= 4,
        !spec_chunk_white(bytes@, i as int),
    ensures
        spec_first_ink(bytes@, i as int) == i as int,
        bytes@.len() >= 4,
{
    let len = bytes.len();
    assert(i + 4 <= len && len >= 4) by (bit_vector)
        requires
            i <= len,
            len - i >= 4,
    ;
    assert(bytes@.len() == bytes.len() as int);
}

/// No complete chunk remains, so the rest of the buffer is white.
proof fn lemma_scan_done(bytes: &[u8], i: usize)
    requires
        i <= bytes.len(),
        !(i <= bytes.len() && bytes.len() - i >= 4),
    ensures
        all_white_from(bytes@, i as int),
        spec_first_ink(bytes@, i as int) == -1,
{
    let len = bytes.len();
    assert(len - i < 4) by (bit_vector)
        requires
            i <= len,
            !(i <= len && len - i >= 4),
    ;
    assert(bytes@.len() == bytes.len() as int);
}

fn hex_digit(nibble: u8) -> (digit: u8)
    requires
        nibble < 16,
    ensures
        digit == spec_hex_digit(nibble),
        is_lower_hex(digit),
{
    proof {
        lemma_hex_is_lower(nibble);
        lemma_digit_fits(nibble);
    }
    if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + (nibble - 10)
    }
}

/// Two lowercase hex digits of `byte`. `0x00` is `00`, `0xff` is `ff`, and
/// `0x0a` is `0a`.
fn write_hex_byte(dst: &mut [u8], byte: u8)
    requires
        dst@.len() >= 2,
    ensures
        final(dst)@.len() == old(dst)@.len(),
        final(dst)@[0] == spec_hex_digit((byte >> 4) as u8),
        final(dst)@[1] == spec_hex_digit((byte & 0x0fu8) as u8),
        is_lower_hex(final(dst)@[0]),
        is_lower_hex(final(dst)@[1]),
        byte == 0x00 ==> final(dst)@[0] == b'0' && final(dst)@[1] == b'0',
        byte == 0xff ==> final(dst)@[0] == b'f' && final(dst)@[1] == b'f',
        byte == 0x0a ==> final(dst)@[0] == b'0' && final(dst)@[1] == b'a',
{
    let high = byte >> 4;
    let low = byte & 0x0fu8;
    proof {
        lemma_nibbles(byte, high, low);
    }
    dst[0] = hex_digit(high);
    dst[1] = hex_digit(low);
}

fn chunk_is_white(bytes: &[u8], i: usize) -> (white: bool)
    requires
        i <= bytes.len(),
        bytes.len() - i >= 4,
    ensures
        white == spec_chunk_white(bytes@, i as int),
{
    proof {
        lemma_chunk_bounds(bytes.len(), i);
    }
    bytes[i] == 0xff && bytes[i + 1] == 0xff && bytes[i + 2] == 0xff && bytes[i + 3] == 0xff
}

/// Write the two digits of `byte` at `at` and `at + 1` of the 15-byte line.
fn place_hex(dst: &mut [u8], at: usize, byte: u8)
    requires
        old(dst)@.len() == 15,
        at == 6 || at == 8 || at == 10 || at == 12,
    ensures
        final(dst)@.len() == 15,
        final(dst)@ == old(dst)@.update(at as int, spec_hex_digit((byte >> 4) as u8)).update(
            at as int + 1,
            spec_hex_digit((byte & 0x0fu8) as u8),
        ),
{
    let mut pair = [0u8;2];
    write_hex_byte(&mut pair, byte);
    assert(at + 1 < 15) by (bit_vector)
        requires
            at == 6 || at == 8 || at == 10 || at == 12,
    ;
    dst[at] = pair[0];
    dst[at + 1] = pair[1];
}

fn line_of(bytes: &[u8], i: usize) -> (line: PixelLine)
    requires
        i <= bytes.len(),
        bytes.len() - i >= 4,
        !spec_chunk_white(bytes@, i as int),
        skipped_white(bytes@, i as int),
    ensures
        line.bytes@.len() == 15,
        spec_line_at(bytes@, i as int, line.bytes@),
{
    proof {
        lemma_chunk_bounds(bytes.len(), i);
    }
    let b0 = bytes[i];
    let b1 = bytes[i + 1];
    let b2 = bytes[i + 2];
    let b3 = bytes[i + 3];
    let mut raw = [0u8;15];
    raw[0] = b'p';
    raw[1] = b'i';
    raw[2] = b'x';
    raw[3] = b'e';
    raw[4] = b'l';
    raw[5] = b' ';
    place_hex(&mut raw, 6, b0);
    place_hex(&mut raw, 8, b1);
    place_hex(&mut raw, 10, b2);
    place_hex(&mut raw, 12, b3);
    raw[14] = b'\n';
    PixelLine { bytes: raw }
}

/// First pixel that is not opaque white, spelled for the serial line.
///
/// An empty buffer, or a buffer of only opaque white, returns `None`. A
/// returned line is 15 bytes. The step of four is the same grouping as
/// `chunks_exact(4)`.
fn sample_line(bytes: &[u8]) -> (out: Option<PixelLine>)
    ensures
        bytes@.len() == 0 ==> out is None,
        match out {
            None => all_white_from(bytes@, 0) && spec_first_ink(bytes@, 0) < 0,
            Some(line) => line.bytes@.len() == 15 && spec_line_at(
                bytes@,
                spec_first_ink(bytes@, 0),
                line.bytes@,
            ),
        },
{
    let mut i: usize = 0;
    while i <= bytes.len() && bytes.len() - i >= 4
        invariant
            i <= bytes.len(),
            skipped_white(bytes@, i as int),
            all_white_from(bytes@, 0) == all_white_from(bytes@, i as int),
            spec_first_ink(bytes@, 0) == spec_first_ink(bytes@, i as int),
        decreases bytes.len() - i,
    {
        if !chunk_is_white(bytes, i) {
            let line = line_of(bytes, i);
            proof {
                lemma_found_ink(bytes, i);
            }
            return Some(line);
        }
        let prev = i;
        proof {
            lemma_skip_white(bytes, prev);
        }
        i = prev + 4;
    }
    proof {
        lemma_scan_done(bytes, i);
    }
    None
}

} // verus!
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
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::Utf8`] when `html` is not UTF-8. Returns
    /// [`FrameError::Blank`] when the bitmap cannot be built or every pixel is
    /// opaque white.
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

    #[must_use]
    pub fn width(&self) -> u32 {
        Self::WIDTH
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        Self::HEIGHT
    }

    /// Tightly packed BGRA. Length is [`Self::BYTES`].
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn pixel_line(&self) -> &PixelLine {
        &self.line
    }
}

fn set_opaque(bytes: &mut [u8]) {
    for i in (0..bytes.len()).step_by(4) {
        bytes[i + 3] = 0xff;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    const OPAQUE_WHITE: [u8; 4] = [0xff, 0xff, 0xff, 0xff];

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
