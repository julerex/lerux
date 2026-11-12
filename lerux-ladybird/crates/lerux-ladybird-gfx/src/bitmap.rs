//! Frame buffer. Bytes are tightly packed, little-endian B, G, R, A.
//!
//! Verus checks that [`FRAME_BYTES`] is `320 * 64 * 4`, that
//! [`Bitmap::set_pixel`] writes one pixel or leaves the buffer unchanged, and
//! that [`Bitmap::pixel_line`] is the lowercase hex spelling of the first
//! pixel that is not opaque white. Those proofs are erased unless the `verus`
//! feature is on.
// The verifier's expansion names `vstd` and `int`. A normal build erases that
// expansion, so the imports are unused unless `verus` is verifying.
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use verus_builtin::*;
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// Width of the page bitmap, in pixels.
pub const FRAME_WIDTH: u32 = 320;

/// Height of the page bitmap, in pixels.
pub const FRAME_HEIGHT: u32 = 64;

/// Byte length of the shared bitmap region. Stride is `FRAME_WIDTH * 4`.
pub const FRAME_BYTES: usize = 320 * 64 * 4;

/// Serial spelling of one sampled pixel. Fifteen bytes: `pixel `, eight
/// lowercase hex digits, and a newline.
pub const PIXEL_LINE_LEN: usize = 15;

/// The frame is 320 by 64 pixels, four bytes each, and the sample line is 15 bytes.
pub proof fn lemma_frame_bytes()
    ensures
        FRAME_WIDTH == 320,
        FRAME_HEIGHT == 64,
        FRAME_BYTES == FRAME_WIDTH as int * FRAME_HEIGHT as int * 4,
        FRAME_BYTES == 320 * 64 * 4,
        PIXEL_LINE_LEN == 15,
{
    assert(FRAME_WIDTH == 320);
    assert(FRAME_HEIGHT == 64);
    assert(FRAME_BYTES == 320 * 64 * 4);
}

/// One pixel. Stored order is B, G, R, A.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Color {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

impl Color {
    /// Opaque black. The fixture ink samples as this color.
    pub const BLACK: Self = Self { b: 0x00, g: 0x00, r: 0x00, a: 0xff };

    /// Opaque white. The sample skips this pixel.
    pub const WHITE: Self = Self { b: 0xff, g: 0xff, r: 0xff, a: 0xff };
}

/// Opaque white is the four bytes `ff ff ff ff`.
pub proof fn lemma_white_bytes()
    ensures
        Color::WHITE.b == 0xff,
        Color::WHITE.g == 0xff,
        Color::WHITE.r == 0xff,
        Color::WHITE.a == 0xff,
{
}

/// `x` and `y` are inside a bitmap of this width and height.
pub open spec fn spec_inside(x: i32, y: i32, width: u32, height: u32) -> bool {
    0 <= (x as int) && (x as int) < (width as int) && 0 <= (y as int) && (y as int) < (
    height as int)
}

/// Byte offset of pixel `(x, y)` in a tightly packed frame of `width`.
pub open spec fn spec_offset(x: int, y: int, width: int) -> int {
    (y * width + x) * 4
}

/// Integer value of one lowercase hex digit.
pub open spec fn hex_value(nibble: int) -> int
    recommends
        0 <= nibble < 16,
{
    if nibble < 10 {
        (b'0' as int) + nibble
    } else {
        (b'a' as int) + nibble - 10
    }
}

/// `byte` is `0-9` or `a-f`.
pub open spec fn is_lower_hex_digit(byte: u8) -> bool {
    let value = byte as int;
    (b'0' as int <= value && value <= b'9' as int) || (b'a' as int <= value && value <= b'f' as int)
}

/// Four bytes at `off` are opaque white.
pub open spec fn white_bytes(bytes: Seq<u8>, off: int) -> bool
    recommends
        0 <= off && off + 4 <= bytes.len(),
{
    bytes[off] == 0xff && bytes[off + 1] == 0xff && bytes[off + 2] == 0xff && bytes[off + 3] == 0xff
}

/// Pixel `pixel` is opaque white.
pub open spec fn white_at(bytes: Seq<u8>, pixel: int) -> bool
    recommends
        0 <= pixel && pixel * 4 + 4 <= bytes.len(),
{
    white_bytes(bytes, pixel * 4)
}

/// Every whole pixel in `bytes` is opaque white.
pub open spec fn all_white(bytes: Seq<u8>) -> bool {
    bytes.len() % 4 == 0 && forall|pixel: int|
        0 <= pixel && pixel * 4 + 4 <= bytes.len() ==> #[trigger] white_at(bytes, pixel)
}

/// `px` is pixel `pixel`, and every earlier pixel is opaque white.
pub open spec fn names_first_ink(bytes: Seq<u8>, pixel: int, px: Seq<u8>) -> bool {
    &&& 0 <= pixel
    &&& pixel * 4 + 4 <= bytes.len()
    &&& px.len() == 4
    &&& px[0] == bytes[pixel * 4]
    &&& px[1] == bytes[pixel * 4 + 1]
    &&& px[2] == bytes[pixel * 4 + 2]
    &&& px[3] == bytes[pixel * 4 + 3]
    &&& !white_at(bytes, pixel)
    &&& forall|earlier: int| 0 <= earlier < pixel ==> #[trigger] white_at(bytes, earlier)
}

/// The four bytes of `pixel` agree in `next` and `prev`.
pub open spec fn same_pixel(next: Seq<u8>, prev: Seq<u8>, pixel: int) -> bool
    recommends
        0 <= pixel && pixel * 4 + 4 <= prev.len() && next.len() == prev.len(),
{
    next[pixel * 4] == prev[pixel * 4] && next[pixel * 4 + 1] == prev[pixel * 4 + 1] && next[pixel
        * 4 + 2] == prev[pixel * 4 + 2] && next[pixel * 4 + 3] == prev[pixel * 4 + 3]
}

/// `next` is `prev` with B, G, R, A written at the aligned offset `off`.
pub open spec fn writes_one_pixel(prev: Seq<u8>, next: Seq<u8>, off: int, color: Color) -> bool {
    &&& next.len() == prev.len()
    &&& 0 <= off && off + 4 <= prev.len()
    &&& off % 4 == 0
    &&& next[off] == color.b
    &&& next[off + 1] == color.g
    &&& next[off + 2] == color.r
    &&& next[off + 3] == color.a
    &&& forall|i: int|
        0 <= i < prev.len() && !(off <= i < off + 4) ==> #[trigger] next[i] == prev[i]
    &&& forall|pixel: int|
        0 <= pixel && pixel * 4 + 4 <= prev.len() && pixel * 4 != off ==> #[trigger] same_pixel(
            next,
            prev,
            pixel,
        )
}

/// Result of one [`Bitmap::set_pixel`] on a buffer of this width and height.
pub open spec fn set_pixel_effect(
    prev: Seq<u8>,
    next: Seq<u8>,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    color: Color,
) -> bool {
    if spec_inside(x, y, width, height) {
        writes_one_pixel(prev, next, spec_offset(x as int, y as int, width as int), color)
    } else {
        next == prev
    }
}

/// `line` is `pixel `, eight lowercase hex digits of `px` in B, G, R, A order, and a newline.
pub open spec fn line_spells(line: Seq<u8>, px: Seq<u8>) -> bool {
    &&& line.len() == 15
    &&& px.len() == 4
    &&& line[0] == b'p'
    &&& line[1] == b'i'
    &&& line[2] == b'x'
    &&& line[3] == b'e'
    &&& line[4] == b'l'
    &&& line[5] == b' '
    &&& line[14] == b'\n'
    &&& forall|channel: int|
        0 <= channel < 4 ==> #[trigger] line[6 + channel * 2] as int == hex_value(
            (px[channel] as int) / 16,
        ) && line[6 + channel * 2 + 1] as int == hex_value((px[channel] as int) % 16)
            && is_lower_hex_digit(line[6 + channel * 2]) && is_lower_hex_digit(
            line[6 + channel * 2 + 1],
        )
}

/// A non-negative `i32` survives a cast to `u32`.
proof fn lemma_i32_nonneg(value: i32)
    requires
        value >= 0,
    ensures
        value as u32 as int == value as int,
{
    assert((value as u32) as i32 == value) by (bit_vector)
        requires
            value >= 0,
    ;
    assert(value as u32 as int == value as int);
}

/// A `u32` index fits in `usize` without changing its value.
proof fn lemma_u32_as_usize(value: u32)
    ensures
        value as usize as int == value as int,
{
    assert(value as usize as int == value as int) by (bit_vector);
}

/// A slice's view length is its `len`, so it fits in `usize`.
proof fn lemma_slice_in_usize(bytes: &[u8])
    ensures
        bytes@.len() == bytes.len() as int,
        0 <= bytes@.len() <= usize::MAX as int,
{
    assert(bytes@.len() == bytes.len() as int);
    assert(bytes.len() as int <= usize::MAX as int);
}

/// `(y * width + x) * 4` is inside a buffer of `width * height` pixels.
proof fn lemma_offset_fits(x: int, y: int, width: int, height: int, len: int)
    requires
        0 <= x < width,
        0 <= y < height,
        len == width * height * 4,
        0 <= len <= usize::MAX as int,
    ensures
        0 <= y * width,
        y * width <= usize::MAX as int,
        y * width + x <= usize::MAX as int,
        spec_offset(x, y, width) <= usize::MAX as int,
        spec_offset(x, y, width) + 4 <= len,
        spec_offset(x, y, width) % 4 == 0,
{
    assert(spec_offset(x, y, width) + 4 <= len) by (nonlinear_arith)
        requires
            0 <= x < width,
            0 <= y < height,
            len == width * height * 4,
    ;
    assert(y * width <= usize::MAX as int) by (nonlinear_arith)
        requires
            0 <= y < height,
            width > 0,
            len == width * height * 4,
            len <= usize::MAX as int,
    ;
    assert(y * width + x <= usize::MAX as int) by (nonlinear_arith)
        requires
            0 <= x < width,
            0 <= y < height,
            len == width * height * 4,
            len <= usize::MAX as int,
    ;
    vstd::arithmetic::div_mod::lemma_mod_multiples_basic(y * width + x, 4);
}

/// A length that is a multiple of four is exactly `4 * (len / 4)`.
proof fn lemma_div4_exact(len: int)
    requires
        len >= 0,
        len % 4 == 0,
    ensures
        4 * (len / 4) == len,
        (len / 4) * 4 == len,
{
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(len, 4);
    assert(len == 4 * (len / 4) + (len % 4));
    vstd::arithmetic::mul::lemma_mul_is_commutative(4, len / 4);
}

/// `width * height * 4` is a multiple of four.
proof fn lemma_area_aligned(width: int, height: int, len: int)
    requires
        len == width * height * 4,
    ensures
        len % 4 == 0,
{
    vstd::arithmetic::div_mod::lemma_mod_multiples_basic(width * height, 4);
}

/// Pixel `pixel` of a `count`-pixel buffer starts at a byte that fits in `usize`.
proof fn lemma_pixel_off_fits(pixel: int, count: int)
    requires
        0 <= pixel < count,
        count * 4 <= usize::MAX as int,
    ensures
        0 <= pixel * 4,
        pixel * 4 <= usize::MAX as int,
        pixel * 4 + 4 <= count * 4,
{
    assert(pixel * 4 + 4 <= count * 4) by (nonlinear_arith)
        requires
            0 <= pixel < count,
    ;
    assert(pixel * 4 <= usize::MAX as int) by (nonlinear_arith)
        requires
            0 <= pixel < count,
            count * 4 <= usize::MAX as int,
    ;
}

/// A pixel index whose byte range ends at or before `len` is below `count`.
proof fn lemma_pixel_before_count(pixel: int, count: int, len: int)
    requires
        0 <= pixel,
        pixel * 4 + 4 <= len,
        count * 4 == len,
        0 <= count,
    ensures
        pixel < count,
{
    vstd::arithmetic::mul::lemma_mul_is_distributive_add_other_way(4, pixel, 1);
    vstd::arithmetic::mul::lemma_mul_basics_4(4);
    assert((pixel + 1) * 4 == pixel * 4 + 4);
    assert((pixel + 1) * 4 <= count * 4);
    vstd::arithmetic::mul::lemma_mul_inequality_converse(pixel + 1, count, 4);
}

/// A located pixel satisfies [`Bitmap::ink_seq`].
proof fn lemma_ink_seq(bytes: Seq<u8>, at: int, px: Seq<u8>)
    requires
        names_first_ink(bytes, at, px),
    ensures
        Bitmap::ink_seq(bytes, Some(px)),
{
    reveal(Bitmap::ink_seq);
    assert(Some(px) is Some);
    assert((Some(px))->0 == px);
    assert(names_first_ink(bytes, at, (Some(px))->0));
    assert(exists|pixel: int| names_first_ink(bytes, pixel, (Some(px))->0));
}

/// The white prefix grows by the pixel just scanned.
proof fn lemma_extend_white(bytes: Seq<u8>, pixel: int)
    requires
        forall|earlier: int| 0 <= earlier < pixel ==> white_at(bytes, earlier),
        white_at(bytes, pixel),
    ensures
        forall|earlier: int| 0 <= earlier < pixel + 1 ==> #[trigger] white_at(bytes, earlier),
{
    assert forall|earlier: int| 0 <= earlier < pixel + 1 implies white_at(bytes, earlier) by {
        if earlier < pixel {
        } else {
            assert(earlier == pixel);
        }
    };
}

/// After the scan, every pixel was opaque white.
proof fn lemma_scan_done(bytes: Seq<u8>, count: int)
    requires
        count >= 0,
        count * 4 == bytes.len(),
        bytes.len() % 4 == 0,
        forall|earlier: int| 0 <= earlier < count ==> white_at(bytes, earlier),
    ensures
        all_white(bytes),
{
    assert forall|pixel: int| 0 <= pixel && pixel * 4 + 4 <= bytes.len() implies white_at(
        bytes,
        pixel,
    ) by {
        lemma_pixel_before_count(pixel, count, bytes.len() as int);
    };
}

/// Two aligned pixels that are not the same pixel do not share bytes.
proof fn lemma_aligned_gap(low: int, high: int)
    requires
        0 <= low < high,
        low % 4 == 0,
        high % 4 == 0,
    ensures
        low + 4 <= high,
{
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(low, 4);
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(high, 4);
    assert(low == 4 * (low / 4) + (low % 4));
    assert(high == 4 * (high / 4) + (high % 4));
    assert(low == 4 * (low / 4));
    assert(high == 4 * (high / 4));
    vstd::arithmetic::mul::lemma_mul_is_commutative(4, low / 4);
    vstd::arithmetic::mul::lemma_mul_is_commutative(4, high / 4);
    assert((low / 4) * 4 < (high / 4) * 4);
    vstd::arithmetic::mul::lemma_mul_strict_inequality_converse(low / 4, high / 4, 4);
    assert(low / 4 + 1 <= high / 4);
    vstd::arithmetic::mul::lemma_mul_inequality(low / 4 + 1, high / 4, 4);
    vstd::arithmetic::mul::lemma_mul_is_distributive_add_other_way(4, low / 4, 1);
    vstd::arithmetic::mul::lemma_mul_basics_4(4);
    assert((low / 4 + 1) * 4 == (low / 4) * 4 + 4);
    assert(low + 4 <= high);
}

/// Writing four aligned bytes leaves every other pixel's bytes alone.
proof fn lemma_other_pixel(prev: Seq<u8>, next: Seq<u8>, off: int, pixel: int)
    requires
        0 <= off && off % 4 == 0 && off + 4 <= prev.len(),
        forall|i: int|
            0 <= i < prev.len() && !(off <= i < off + 4) ==> #[trigger] next[i] == prev[i],
        0 <= pixel && pixel * 4 + 4 <= prev.len() && pixel * 4 != off,
    ensures
        same_pixel(next, prev, pixel),
{
    let start = pixel * 4;
    vstd::arithmetic::div_mod::lemma_mod_multiples_basic(pixel, 4);
    assert(start % 4 == 0);
    if start < off {
        lemma_aligned_gap(start, off);
        assert(start + 4 <= off);
    } else {
        lemma_aligned_gap(off, start);
        assert(off + 4 <= start);
    }
    assert(next[start] == prev[start]);
    assert(next[start + 1] == prev[start + 1]);
    assert(next[start + 2] == prev[start + 2]);
    assert(next[start + 3] == prev[start + 3]);
}

/// Four updates at `off` are one pixel write.
proof fn lemma_one_pixel(prev: Seq<u8>, next: Seq<u8>, off: int, color: Color)
    requires
        0 <= off,
        off % 4 == 0,
        off + 4 <= prev.len(),
        next == prev.update(off, color.b).update(off + 1, color.g).update(off + 2, color.r).update(
            off + 3,
            color.a,
        ),
    ensures
        writes_one_pixel(prev, next, off, color),
{
    assert(next.len() == prev.len());
    assert(next[off] == color.b);
    assert(next[off + 1] == color.g);
    assert(next[off + 2] == color.r);
    assert(next[off + 3] == color.a);
    assert forall|i: int| 0 <= i < prev.len() && !(off <= i < off + 4) implies next[i]
        == prev[i] by {};
    assert forall|pixel: int|
        0 <= pixel && pixel * 4 + 4 <= prev.len() && pixel * 4 != off implies same_pixel(
        next,
        prev,
        pixel,
    ) by {
        lemma_other_pixel(prev, next, off, pixel);
    };
}

/// The eight hex digits of `px` are lowercase and match the nibbles.
proof fn lemma_line_channels(line: Seq<u8>, px: Seq<u8>)
    requires
        line.len() == 15,
        px.len() == 4,
        line[0] == b'p',
        line[1] == b'i',
        line[2] == b'x',
        line[3] == b'e',
        line[4] == b'l',
        line[5] == b' ',
        line[14] == b'\n',
        line[6] as int == hex_value(px[0] as int / 16),
        line[7] as int == hex_value(px[0] as int % 16),
        line[8] as int == hex_value(px[1] as int / 16),
        line[9] as int == hex_value(px[1] as int % 16),
        line[10] as int == hex_value(px[2] as int / 16),
        line[11] as int == hex_value(px[2] as int % 16),
        line[12] as int == hex_value(px[3] as int / 16),
        line[13] as int == hex_value(px[3] as int % 16),
        is_lower_hex_digit(line[6]),
        is_lower_hex_digit(line[7]),
        is_lower_hex_digit(line[8]),
        is_lower_hex_digit(line[9]),
        is_lower_hex_digit(line[10]),
        is_lower_hex_digit(line[11]),
        is_lower_hex_digit(line[12]),
        is_lower_hex_digit(line[13]),
    ensures
        line_spells(line, px),
{
    assert forall|channel: int| 0 <= channel < 4 implies line[6 + channel * 2] as int == hex_value(
        (px[channel] as int) / 16,
    ) && line[6 + channel * 2 + 1] as int == hex_value((px[channel] as int) % 16)
        && is_lower_hex_digit(line[6 + channel * 2]) && is_lower_hex_digit(
        line[6 + channel * 2 + 1],
    ) by {
        if channel == 0 {
            assert(6 + channel * 2 == 6);
        } else if channel == 1 {
            assert(6 + channel * 2 == 8);
        } else if channel == 2 {
            assert(6 + channel * 2 == 10);
        } else {
            assert(channel == 3);
            assert(6 + channel * 2 == 12);
        }
    };
}

/// Division and remainder of a byte by 16 are its nibbles.
proof fn lemma_nibbles(byte: u8)
    ensures
        0 <= byte as int / 16 < 16,
        0 <= byte as int % 16 < 16,
{
    vstd::arithmetic::div_mod::lemma_mod_bound(byte as int, 16);
    assert(byte as int <= 255);
    vstd::arithmetic::div_mod::lemma_div_is_ordered(byte as int, 255, 16);
    assert(255int / 16 == 15);
}

/// Tightly packed BGRA frame. The caller owns the bytes.
pub struct Bitmap<'a> {
    bytes: &'a mut [u8],
    width: u32,
    height: u32,
}

impl Bitmap<'_> {
    /// The buffer holds `width * height` pixels and both sides are positive.
    pub closed spec fn well_formed(self) -> bool {
        self.width > 0 && self.height > 0 && self.bytes@.len() == self.width as int
            * self.height as int * 4
    }

    /// Post-state of [`Self::set_pixel`]: dimensions stay, and one pixel changes or none do.
    pub closed spec fn set_pixel_post(self, prev: Self, x: i32, y: i32, color: Color) -> bool {
        &&& self.well_formed()
        &&& self.width == prev.width
        &&& self.height == prev.height
        &&& set_pixel_effect(prev.bytes@, self.bytes@, prev.width, prev.height, x, y, color)
    }

    /// Bytes of this bitmap, as a sequence.
    pub closed spec fn view_bytes(self) -> Seq<u8> {
        self.bytes@
    }

    /// `out` is the first non-white pixel of `bytes`, or `bytes` is all white.
    pub open spec fn ink_seq(bytes: Seq<u8>, out: Option<Seq<u8>>) -> bool {
        &&& out is None ==> all_white(bytes)
        &&& out is Some ==> exists|pixel: int| names_first_ink(bytes, pixel, out->0)
    }

    /// [`Self::first_non_white`] found opaque white, or the first other pixel.
    pub open spec fn first_ink(self, out: Option<[u8; 4]>) -> bool {
        Self::ink_seq(
            self.view_bytes(),
            match out {
                None => None,
                Some(px) => Some(px@),
            },
        )
    }

    /// [`Self::pixel_line`] is absent on an all-white frame, or spells the first other pixel.
    pub closed spec fn sample_line(self, out: Option<PixelLine>) -> bool {
        match out {
            None => all_white(self.bytes@),
            Some(line) => exists|pixel: int, px: Seq<u8>|
                names_first_ink(self.bytes@, pixel, px) && line_spells(line.bytes@, px),
        }
    }

    /// Write `color` at one pixel. A coordinate outside the frame is ignored.
    ///
    /// A negative coordinate, or a coordinate at or past the width or height,
    /// leaves the buffer unchanged. A coordinate inside the frame writes that
    /// color's B, G, R, A at byte offset `(y * width + x) * 4` and does not
    /// write any other pixel.
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Color)
        requires
            self.well_formed(),
        ensures
            final(self).set_pixel_post(*old(self), x, y, color),
    {
        if x < 0 || y < 0 {
            proof {
                assert(!spec_inside(x, y, self.width, self.height));
            }
            return;
        }
        #[expect(clippy::cast_sign_loss, reason = "negative coordinates already returned")]
        let x_u = x as u32;
        #[expect(clippy::cast_sign_loss, reason = "negative coordinates already returned")]
        let y_u = y as u32;
        proof {
            lemma_i32_nonneg(x);
            lemma_i32_nonneg(y);
        }
        if x_u >= self.width || y_u >= self.height {
            proof {
                assert(x_u as int == x as int);
                assert(y_u as int == y as int);
                assert(!spec_inside(x, y, self.width, self.height));
            }
            return;
        }
        let off = self.offset(x_u, y_u);
        proof {
            assert(x_u as int == x as int);
            assert(y_u as int == y as int);
            assert(off as int == spec_offset(x as int, y as int, self.width as int));
        }
        write_bgra(self.bytes, off, color);
    }

    /// Byte offset of an in-range pixel. The result is 4-byte aligned.
    fn offset(&self, x: u32, y: u32) -> (off: usize)
        requires
            self.well_formed(),
            x < self.width,
            y < self.height,
        ensures
            off as int == spec_offset(x as int, y as int, self.width as int),
            off as int + 4 <= old(self.bytes)@.len(),
            off as int % 4 == 0,
    {
        let y_us = y as usize;
        let x_us = x as usize;
        let width_us = self.width as usize;
        proof {
            lemma_slice_in_usize(self.bytes);
            lemma_u32_as_usize(x);
            lemma_u32_as_usize(y);
            lemma_u32_as_usize(self.width);
            lemma_offset_fits(
                x as int,
                y as int,
                self.width as int,
                self.height as int,
                self.bytes@.len() as int,
            );
            assert(y_us as int == y as int);
            assert(x_us as int == x as int);
            assert(width_us as int == self.width as int);
            assert(y_us as int * width_us as int <= usize::MAX as int);
        }
        let row = y_us * width_us;
        proof {
            assert(row as int + x_us as int <= usize::MAX as int);
        }
        let index = row + x_us;
        proof {
            assert(index as int * 4 <= usize::MAX as int);
        }
        index * 4
    }

    /// First pixel that is not [`Color::WHITE`], in row-major order.
    #[must_use]
    pub fn first_non_white(&self) -> (out: Option<[u8; 4]>)
        requires
            self.well_formed(),
        ensures
            self.first_ink(out),
    {
        proof {
            lemma_white_bytes();
            lemma_slice_in_usize(self.bytes);
            lemma_area_aligned(self.width as int, self.height as int, self.bytes@.len() as int);
        }
        let count = pixel_count(self.bytes);
        let mut pixel: usize = 0;
        while pixel < count
            invariant
                self.well_formed(),
                pixel <= count,
                count as int * 4 == self.bytes@.len(),
                count as int * 4 <= usize::MAX as int,
                forall|earlier: int|
                    0 <= earlier < pixel as int ==> #[trigger] white_at(self.bytes@, earlier),
            decreases count - pixel,
        {
            let off = pixel_off(pixel, count);
            if !is_white_pixel(self.bytes, off) {
                let px = read_ink(self.bytes, off, pixel);
                proof {
                    reveal(Bitmap::view_bytes);
                    lemma_ink_seq(self.view_bytes(), pixel as int, px@);
                    assert(self.first_ink(Some(px)));
                }
                return Some(px);
            }
            proof {
                assert(white_at(self.bytes@, pixel as int));
                lemma_extend_white(self.bytes@, pixel as int);
            }
            pixel += 1;
        }
        proof {
            lemma_scan_done(self.bytes@, count as int);
        }
        None
    }

    /// Serial line for [`Self::first_non_white`]. `None` when every pixel is
    /// opaque white (`ff ff ff ff`).
    ///
    /// The bytes are `pixel `, eight lowercase hex digits, and a newline.
    /// [`PIXEL_LINE_LEN`] is 15.
    #[must_use]
    pub fn pixel_line(&self) -> (out: Option<PixelLine>)
        requires
            self.well_formed(),
        ensures
            PIXEL_LINE_LEN == 15,
            self.sample_line(out),
    {
        proof {
            lemma_frame_bytes();
        }
        match self.first_non_white() {
            None => None,
            Some(px) => {
                let line = PixelLine::from_bgra(px);
                proof {
                    assert(exists|pixel: int|
                        names_first_ink(self.bytes@, pixel, px@) && line_spells(line.bytes@, px@));
                }
                Some(line)
            },
        }
    }
}

/// Number of whole pixels in a buffer whose length is a multiple of four.
fn pixel_count(bytes: &[u8]) -> (count: usize)
    requires
        bytes@.len() % 4 == 0,
        bytes@.len() >= 0,
    ensures
        count as int == bytes@.len() / 4,
        count as int * 4 == bytes@.len(),
{
    proof {
        assert(bytes@.len() == bytes.len() as int);
        lemma_div4_exact(bytes@.len() as int);
    }
    bytes.len() / 4
}

/// Byte offset of pixel `pixel` in a buffer of `count` pixels.
fn pixel_off(pixel: usize, count: usize) -> (off: usize)
    requires
        pixel < count,
        count as int * 4 <= usize::MAX as int,
    ensures
        off as int == pixel as int * 4,
        off as int + 4 <= count as int * 4,
{
    proof {
        lemma_pixel_off_fits(pixel as int, count as int);
        assert((pixel as int) * 4 <= usize::MAX as int);
    }
    let _ = count;
    pixel * 4
}

/// Copy the non-white pixel at `off` and record that it is the first one.
fn read_ink(bytes: &[u8], off: usize, pixel: usize) -> (px: [u8; 4])
    requires
        off as int == pixel as int * 4,
        off as int + 4 <= bytes@.len(),
        !white_at(bytes@, pixel as int),
        forall|earlier: int| 0 <= earlier < pixel as int ==> white_at(bytes@, earlier),
    ensures
        names_first_ink(bytes@, pixel as int, px@),
{
    let _ = pixel;
    let px = [bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]];
    proof {
        assert(px@[0] == bytes@[off as int]);
        assert(px@[1] == bytes@[off as int + 1]);
        assert(px@[2] == bytes@[off as int + 2]);
        assert(px@[3] == bytes@[off as int + 3]);
    }
    px
}

/// The four bytes at `off` are opaque white.
fn is_white_pixel(bytes: &[u8], off: usize) -> (white: bool)
    requires
        off as int + 4 <= bytes@.len(),
    ensures
        white == white_bytes(bytes@, off as int),
{
    proof {
        assert(off as int + 3 < bytes@.len());
        assert(bytes@.len() == bytes.len() as int);
    }
    bytes[off] == 0xff && bytes[off + 1] == 0xff && bytes[off + 2] == 0xff && bytes[off + 3] == 0xff
}

/// Write B, G, R, A at an aligned offset. Every other pixel is unchanged.
fn write_bgra(bytes: &mut [u8], off: usize, color: Color)
    requires
        off as int % 4 == 0,
        off as int + 4 <= old(bytes)@.len(),
    ensures
        writes_one_pixel(old(bytes)@, final(bytes)@, off as int, color),
{
    let ghost prev = bytes@;
    bytes[off] = color.b;
    let ghost after_b = bytes@;
    bytes[off + 1] = color.g;
    let ghost after_g = bytes@;
    bytes[off + 2] = color.r;
    let ghost after_r = bytes@;
    bytes[off + 3] = color.a;
    proof {
        assert(after_b == prev.update(off as int, color.b));
        assert(after_g == after_b.update(off as int + 1, color.g));
        assert(after_r == after_g.update(off as int + 2, color.r));
        assert(bytes@ == after_r.update(off as int + 3, color.a));
        lemma_one_pixel(prev, bytes@, off as int, color);
    }
}

/// Lowercase hex digit of a nibble in `0..16`.
fn hex_digit(nibble: u8) -> (digit: u8)
    requires
        nibble < 16,
    ensures
        digit as int == hex_value(nibble as int),
        is_lower_hex_digit(digit),
{
    if nibble < 10 {
        proof {
            assert((b'0' as int) + (nibble as int) < 256);
        }
        let digit = b'0' + nibble;
        proof {
            assert(digit as int == (b'0' as int) + (nibble as int));
            assert(digit as int == hex_value(nibble as int));
            assert(b'0' as int + 9 == b'9' as int);
            assert(is_lower_hex_digit(digit));
        }
        digit
    } else {
        proof {
            assert(nibble >= 10);
            assert((b'a' as int) + (nibble as int) - 10 < 256);
        }
        let digit = b'a' + (nibble - 10);
        proof {
            assert(digit as int == (b'a' as int) + (nibble as int) - 10);
            assert(digit as int == hex_value(nibble as int));
            assert(b'a' as int + 5 == b'f' as int);
            assert(is_lower_hex_digit(digit));
        }
        digit
    }
}

/// Two lowercase hex digits of `byte`, high nibble first.
fn hex_pair(byte: u8) -> (pair: (u8, u8))
    ensures
        pair.0 as int == hex_value(byte as int / 16),
        pair.1 as int == hex_value(byte as int % 16),
        is_lower_hex_digit(pair.0),
        is_lower_hex_digit(pair.1),
{
    proof {
        lemma_nibbles(byte);
    }
    let high = byte / 16;
    let low = byte % 16;
    proof {
        assert(high as int == byte as int / 16);
        assert(low as int == byte as int % 16);
        assert(high < 16);
        assert(low < 16);
    }
    (hex_digit(high), hex_digit(low))
}

/// Serial spelling of one sampled pixel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PixelLine {
    bytes: [u8; PIXEL_LINE_LEN],
}

impl PixelLine {
    /// Bytes of the sample line.
    pub closed spec fn line_bytes(self) -> Seq<u8> {
        self.bytes@
    }

    /// `pixel `, eight lowercase hex digits of `pixel` in stored order, and a newline.
    fn from_bgra(pixel: [u8; 4]) -> (out: Self)
        ensures
            PIXEL_LINE_LEN == 15,
            out.bytes@.len() == 15,
            line_spells(out.bytes@, pixel@),
    {
        let mut bytes = [0u8;PIXEL_LINE_LEN];
        bytes[0] = b'p';
        bytes[1] = b'i';
        bytes[2] = b'x';
        bytes[3] = b'e';
        bytes[4] = b'l';
        bytes[5] = b' ';
        let channel_b = hex_pair(pixel[0]);
        let channel_g = hex_pair(pixel[1]);
        let channel_r = hex_pair(pixel[2]);
        let channel_a = hex_pair(pixel[3]);
        bytes[6] = channel_b.0;
        bytes[7] = channel_b.1;
        bytes[8] = channel_g.0;
        bytes[9] = channel_g.1;
        bytes[10] = channel_r.0;
        bytes[11] = channel_r.1;
        bytes[12] = channel_a.0;
        bytes[13] = channel_a.1;
        bytes[14] = b'\n';
        proof {
            lemma_frame_bytes();
            assert(pixel@.len() == 4);
            assert(bytes@[0] == b'p');
            assert(bytes@[6] as int == hex_value(pixel@[0] as int / 16));
            assert(bytes@[7] as int == hex_value(pixel@[0] as int % 16));
            assert(bytes@[8] as int == hex_value(pixel@[1] as int / 16));
            assert(bytes@[9] as int == hex_value(pixel@[1] as int % 16));
            assert(bytes@[10] as int == hex_value(pixel@[2] as int / 16));
            assert(bytes@[11] as int == hex_value(pixel@[2] as int % 16));
            assert(bytes@[12] as int == hex_value(pixel@[3] as int / 16));
            assert(bytes@[13] as int == hex_value(pixel@[3] as int % 16));
            lemma_line_channels(bytes@, pixel@);
        }
        Self { bytes }
    }

    #[must_use]
    pub fn as_bytes(&self) -> (out: &[u8])
        ensures
            out@ == self.line_bytes(),
    {
        &self.bytes
    }
}

} // verus!
impl Color {
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 4] {
        [self.b, self.g, self.r, self.a]
    }

    #[must_use]
    pub const fn from_bytes(bytes: [u8; 4]) -> Self {
        Self {
            b: bytes[0],
            g: bytes[1],
            r: bytes[2],
            a: bytes[3],
        }
    }
}

const _: () = assert!(FRAME_BYTES == 320 * 64 * 4);
const _: () = assert!(FRAME_BYTES.is_multiple_of(0x1000));

/// A rectangle in pixel coordinates. A non-positive size covers nothing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    #[must_use]
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

impl<'a> Bitmap<'a> {
    /// `bytes` must hold `width * height * 4` bytes. A zero side is rejected.
    pub fn new(bytes: &'a mut [u8], width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        let needed = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        let bytes = bytes.get_mut(..needed)?;
        Some(Self {
            bytes,
            width,
            height,
        })
    }

    /// The page frame. `bytes` must hold [`FRAME_BYTES`].
    pub fn frame(bytes: &'a mut [u8]) -> Option<Self> {
        Self::new(bytes, FRAME_WIDTH, FRAME_HEIGHT)
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }

    #[must_use]
    pub fn pixel(&self, x: i32, y: i32) -> Option<Color> {
        let x = u32::try_from(x).ok()?;
        let y = u32::try_from(y).ok()?;
        if x >= self.width || y >= self.height {
            return None;
        }
        let off = self.offset(x, y);
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&self.bytes[off..off + 4]);
        Some(Color::from_bytes(bytes))
    }

    /// Fill `rect`, clipped to the frame. A non-positive size fills nothing.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let Some((x0, x1)) = visible_range(rect.x, rect.width, self.width) else {
            return;
        };
        let Some((y0, y1)) = visible_range(rect.y, rect.height, self.height) else {
            return;
        };
        for y in y0..y1 {
            for x in x0..x1 {
                self.set_pixel(x, y, color);
            }
        }
    }

    /// Copy pixels from `src` into this frame. There is no scale.
    ///
    /// The copied size is the smaller of the two rectangles. Coordinates
    /// outside either frame are skipped. `src` is a different bitmap: the
    /// borrow checker rejects a blit onto itself.
    pub fn blit_rect(&mut self, dst: Rect, src: &Bitmap<'_>, src_rect: Rect) {
        let width = dst.width.min(src_rect.width);
        let height = dst.height.min(src_rect.height);
        let Some((x0, x1)) = visible_range(dst.x, width, self.width) else {
            return;
        };
        let Some((y0, y1)) = visible_range(dst.y, height, self.height) else {
            return;
        };
        for y in y0..y1 {
            for x in x0..x1 {
                let Some(col) = x.checked_sub(dst.x) else {
                    continue;
                };
                let Some(row) = y.checked_sub(dst.y) else {
                    continue;
                };
                let Some(sx) = src_rect.x.checked_add(col) else {
                    continue;
                };
                let Some(sy) = src_rect.y.checked_add(row) else {
                    continue;
                };
                let Some(color) = src.pixel(sx, sy) else {
                    continue;
                };
                self.set_pixel(x, y, color);
            }
        }
    }

    /// Set every pixel's alpha to `0xff`. The sample runs after this.
    pub fn set_opaque(&mut self) {
        for pixel in self.bytes.chunks_exact_mut(4) {
            pixel[3] = 0xff;
        }
    }
}

/// Half-open column or row range that lies inside `0..limit`.
fn visible_range(origin: i32, length: i32, limit: u32) -> Option<(i32, i32)> {
    if length <= 0 {
        return None;
    }
    let limit = i32::try_from(limit).unwrap_or(i32::MAX);
    let start = i64::from(origin);
    let end = start + i64::from(length);
    let start = start.max(0);
    let end = end.min(i64::from(limit));
    if start >= end {
        return None;
    }
    let start = i32::try_from(start).ok()?;
    let end = i32::try_from(end).ok()?;
    Some((start, end))
}
