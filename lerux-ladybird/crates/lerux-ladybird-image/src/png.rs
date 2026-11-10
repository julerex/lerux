//! PNG with 8-bit RGB or RGBA samples and no interlacing.

use alloc::vec::Vec;

use crate::inflate::inflate_zlib;

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
const CRC_TABLE: [u32; 256] = crc_table();

/// Why [`decode_png`] rejected the bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PngError {
    /// The first eight bytes are not the PNG signature.
    Signature,
    /// A chunk runs past the end, `IHDR` is not first or is malformed, or
    /// `IDAT` or `IEND` is missing.
    Chunk,
    /// A chunk CRC does not match its type and data.
    Crc,
    /// The header is not 8-bit RGB or RGBA, is interlaced, or describes an
    /// image too large to allocate.
    Unsupported,
    /// The zlib stream is malformed.
    Zlib,
    /// The Adler-32 of the inflated bytes does not match the trailer.
    Adler,
    /// The inflated scanlines are the wrong length or name an unknown filter.
    Scanline,
}

/// Decoded pixels, top row first.
pub struct Image {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Image {
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// `width * height` pixels, tightly packed, each stored B, G, R, A.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

/// Decode a PNG. An RGB pixel gets alpha `0xff`.
///
/// # Errors
///
/// Returns the first [`PngError`] the bytes hit.
pub fn decode_png(bytes: &[u8]) -> Result<Image, PngError> {
    let mut rest = bytes
        .strip_prefix(SIGNATURE.as_slice())
        .ok_or(PngError::Signature)?;
    let header = Header::parse(&next_chunk(&mut rest)?)?;
    let zlib = image_data(&mut rest)?;
    let mut raw = inflate_zlib(&zlib, header.raw_len)?;
    drop(zlib);
    unfilter(&mut raw, &header)?;
    header.image(&raw)
}

struct Chunk<'a> {
    kind: [u8; 4],
    data: &'a [u8],
}

fn next_chunk<'a>(rest: &mut &'a [u8]) -> Result<Chunk<'a>, PngError> {
    let (length, tail) = rest.split_first_chunk::<4>().ok_or(PngError::Chunk)?;
    let len = usize::try_from(u32::from_be_bytes(*length)).map_err(|_| PngError::Chunk)?;
    let body_len = len.checked_add(4).ok_or(PngError::Chunk)?;
    let (body, tail) = tail.split_at_checked(body_len).ok_or(PngError::Chunk)?;
    let (crc, tail) = tail.split_first_chunk::<4>().ok_or(PngError::Chunk)?;
    if crc32(body) != u32::from_be_bytes(*crc) {
        return Err(PngError::Crc);
    }
    let (kind, data) = body.split_first_chunk::<4>().ok_or(PngError::Chunk)?;
    *rest = tail;
    Ok(Chunk { kind: *kind, data })
}

/// Concatenated `IDAT` data up to `IEND`. Ancillary chunks are skipped.
fn image_data(rest: &mut &[u8]) -> Result<Vec<u8>, PngError> {
    let mut zlib = Vec::new();
    loop {
        let chunk = next_chunk(rest)?;
        match &chunk.kind {
            b"IDAT" => {
                zlib.try_reserve(chunk.data.len())
                    .map_err(|_| PngError::Unsupported)?;
                zlib.extend_from_slice(chunk.data);
            }
            b"IEND" if !zlib.is_empty() => return Ok(zlib),
            b"IEND" | b"IHDR" => return Err(PngError::Chunk),
            _ => {}
        }
    }
}

#[derive(Clone, Copy)]
enum ColorType {
    Rgb,
    Rgba,
}

impl ColorType {
    fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgb => 3,
            Self::Rgba => 4,
        }
    }
}

/// `IHDR` with the derived sizes, each checked against `usize` overflow.
struct Header {
    width: u32,
    height: u32,
    color: ColorType,
    stride: usize,
    raw_len: usize,
    pixel_len: usize,
}

impl Header {
    fn parse(chunk: &Chunk<'_>) -> Result<Self, PngError> {
        let (b"IHDR", Ok(data)) = (&chunk.kind, <[u8; 13]>::try_from(chunk.data)) else {
            return Err(PngError::Chunk);
        };
        let width = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let height = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let [depth, color, compression, filter, interlace] =
            [data[8], data[9], data[10], data[11], data[12]];
        if width == 0 || height == 0 {
            return Err(PngError::Chunk);
        }
        let color = match color {
            2 => ColorType::Rgb,
            6 => ColorType::Rgba,
            _ => return Err(PngError::Unsupported),
        };
        if depth != 8 || compression != 0 || filter != 0 || interlace != 0 {
            return Err(PngError::Unsupported);
        }
        Self::sized(width, height, color).ok_or(PngError::Unsupported)
    }

    fn sized(width: u32, height: u32, color: ColorType) -> Option<Self> {
        let columns = usize::try_from(width).ok()?;
        let rows = usize::try_from(height).ok()?;
        let stride = columns.checked_mul(color.bytes_per_pixel())?;
        Some(Self {
            width,
            height,
            color,
            stride,
            raw_len: stride.checked_add(1)?.checked_mul(rows)?,
            pixel_len: columns.checked_mul(4)?.checked_mul(rows)?,
        })
    }

    fn image(&self, raw: &[u8]) -> Result<Image, PngError> {
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(self.pixel_len)
            .map_err(|_| PngError::Unsupported)?;
        for line in raw.chunks_exact(self.stride + 1) {
            for sample in line[1..].chunks_exact(self.color.bytes_per_pixel()) {
                let alpha = match self.color {
                    ColorType::Rgb => 0xff,
                    ColorType::Rgba => sample[3],
                };
                pixels.extend_from_slice(&[sample[2], sample[1], sample[0], alpha]);
            }
        }
        Ok(Image {
            width: self.width,
            height: self.height,
            pixels,
        })
    }
}

#[derive(Clone, Copy)]
enum Filter {
    None,
    Sub,
    Up,
    Average,
    Paeth,
}

impl TryFrom<u8> for Filter {
    type Error = PngError;

    fn try_from(byte: u8) -> Result<Self, PngError> {
        Ok(match byte {
            0 => Self::None,
            1 => Self::Sub,
            2 => Self::Up,
            3 => Self::Average,
            4 => Self::Paeth,
            _ => return Err(PngError::Scanline),
        })
    }
}

impl Filter {
    /// The byte this filter subtracted, from the reconstructed neighbours.
    fn predict(self, left: u8, above: u8, corner: u8) -> u8 {
        match self {
            Self::None => 0,
            Self::Sub => left,
            Self::Up => above,
            Self::Average => u8::midpoint(left, above),
            Self::Paeth => paeth(left, above, corner),
        }
    }
}

fn paeth(left: u8, above: u8, corner: u8) -> u8 {
    let estimate = i16::from(left) + i16::from(above) - i16::from(corner);
    let to_left = (estimate - i16::from(left)).abs();
    let to_above = (estimate - i16::from(above)).abs();
    let to_corner = (estimate - i16::from(corner)).abs();
    if to_left <= to_above && to_left <= to_corner {
        left
    } else if to_above <= to_corner {
        above
    } else {
        corner
    }
}

/// Undo each scanline's filter in place. The row above the first is zero.
fn unfilter(raw: &mut [u8], header: &Header) -> Result<(), PngError> {
    let line = header.stride + 1;
    let bpp = header.color.bytes_per_pixel();
    for start in (0..raw.len()).step_by(line) {
        let (done, rest) = raw.split_at_mut(start);
        let above = &done[done.len().saturating_sub(header.stride)..];
        let (filter, row) = rest[..line].split_first_mut().ok_or(PngError::Scanline)?;
        let filter = Filter::try_from(*filter)?;
        for i in 0..row.len() {
            let left = i.checked_sub(bpp).map_or(0, |j| row[j]);
            let up = above.get(i).copied().unwrap_or(0);
            let corner = i
                .checked_sub(bpp)
                .and_then(|j| above.get(j))
                .copied()
                .unwrap_or(0);
            row[i] = row[i].wrapping_add(filter.predict(left, up, corner));
        }
    }
    Ok(())
}

const fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut n = 0u32;
    while n < 256 {
        let mut c = n;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 == 1 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[n as usize] = c;
        n += 1;
    }
    table
}

fn crc32(bytes: &[u8]) -> u32 {
    !bytes.iter().fold(u32::MAX, |crc, &byte| {
        CRC_TABLE[usize::from(crc.to_le_bytes()[0] ^ byte)] ^ (crc >> 8)
    })
}
