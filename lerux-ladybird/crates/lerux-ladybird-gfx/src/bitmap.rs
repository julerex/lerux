//! Frame buffer. Bytes are tightly packed, little-endian B, G, R, A.

/// Width of the page bitmap, in pixels.
pub const FRAME_WIDTH: u32 = 320;
/// Height of the page bitmap, in pixels.
pub const FRAME_HEIGHT: u32 = 64;
/// Byte length of the shared bitmap region. Stride is `FRAME_WIDTH * 4`.
pub const FRAME_BYTES: usize = (FRAME_WIDTH as usize) * (FRAME_HEIGHT as usize) * 4;

const _: () = assert!(FRAME_BYTES == 320 * 64 * 4);
const _: () = assert!(FRAME_BYTES.is_multiple_of(0x1000));

/// Serial spelling of one sampled pixel. Fifteen bytes: `pixel `, eight
/// lowercase hex digits, and a newline.
pub const PIXEL_LINE_LEN: usize = 15;

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
    pub const BLACK: Self = Self {
        b: 0x00,
        g: 0x00,
        r: 0x00,
        a: 0xff,
    };
    /// Opaque white. The sample skips this pixel.
    pub const WHITE: Self = Self {
        b: 0xff,
        g: 0xff,
        r: 0xff,
        a: 0xff,
    };

    pub const fn to_bytes(self) -> [u8; 4] {
        [self.b, self.g, self.r, self.a]
    }

    pub const fn from_bytes(bytes: [u8; 4]) -> Self {
        Self {
            b: bytes[0],
            g: bytes[1],
            r: bytes[2],
            a: bytes[3],
        }
    }
}

/// A rectangle in pixel coordinates. A non-positive size covers nothing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Tightly packed BGRA frame. The caller owns the bytes.
pub struct Bitmap<'a> {
    bytes: &'a mut [u8],
    width: u32,
    height: u32,
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

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }

    /// Write `color` at one pixel. A coordinate outside the frame is ignored.
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        let Ok(x) = u32::try_from(x) else {
            return;
        };
        let Ok(y) = u32::try_from(y) else {
            return;
        };
        if x >= self.width || y >= self.height {
            return;
        }
        let off = self.offset(x, y);
        self.bytes[off..off + 4].copy_from_slice(&color.to_bytes());
    }

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

    /// First pixel that is not [`Color::WHITE`], in row-major order.
    pub fn first_non_white(&self) -> Option<[u8; 4]> {
        let white = Color::WHITE.to_bytes();
        for chunk in self.bytes.chunks_exact(4) {
            if chunk == white {
                continue;
            }
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(chunk);
            return Some(bytes);
        }
        None
    }

    /// Serial line for [`Self::first_non_white`]. `None` when every pixel is
    /// opaque white.
    pub fn pixel_line(&self) -> Option<PixelLine> {
        self.first_non_white().map(PixelLine::from_bgra)
    }

    fn offset(&self, x: u32, y: u32) -> usize {
        (y as usize * self.width as usize + x as usize) * 4
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PixelLine {
    bytes: [u8; PIXEL_LINE_LEN],
}

impl PixelLine {
    fn from_bgra(pixel: [u8; 4]) -> Self {
        let mut bytes = [0u8; PIXEL_LINE_LEN];
        bytes[..6].copy_from_slice(b"pixel ");
        write_hex(pixel[0], &mut bytes[6..8]);
        write_hex(pixel[1], &mut bytes[8..10]);
        write_hex(pixel[2], &mut bytes[10..12]);
        write_hex(pixel[3], &mut bytes[12..14]);
        bytes[14] = b'\n';
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

fn write_hex(byte: u8, dst: &mut [u8]) {
    const HEX: [u8; 16] = *b"0123456789abcdef";
    dst[0] = HEX[(byte >> 4) as usize];
    dst[1] = HEX[(byte & 0x0f) as usize];
}
