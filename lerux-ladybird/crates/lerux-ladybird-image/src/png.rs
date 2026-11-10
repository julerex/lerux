//! 8-bit, non-interlaced PNG with one `IHDR` and one `IDAT`.

use alloc::vec::Vec;

use crate::{
    checksum::crc32_pair,
    error::DecodeError,
    inflate::{self, MAX_INFLATED_BYTES},
};

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const IHDR: [u8; 4] = *b"IHDR";
const IDAT: [u8; 4] = *b"IDAT";
const IEND: [u8; 4] = *b"IEND";

/// One pixel. An RGB image is stored with alpha `0xff`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Pixel {
    /// Red, green, blue, and alpha.
    #[must_use]
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            r: red,
            g: green,
            b: blue,
            a: alpha,
        }
    }

    /// Opaque red, green, and blue. Alpha is `0xff`.
    #[must_use]
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, 0xff)
    }

    /// Packed bytes in frame order: B, G, R, A.
    #[must_use]
    pub const fn bgra(self) -> [u8; 4] {
        [self.b, self.g, self.r, self.a]
    }
}

/// Decoded pixels in row-major order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Image {
    width: u32,
    height: u32,
    pixels: Vec<Pixel>,
}

impl Image {
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub const fn pixels(&self) -> &[Pixel] {
        self.pixels.as_slice()
    }

    /// The pixel at `x`, `y`, or `None` when the point is outside the image.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<Pixel> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let width = usize::try_from(self.width).ok()?;
        let x = usize::try_from(x).ok()?;
        let y = usize::try_from(y).ok()?;
        let index = y.checked_mul(width)?.checked_add(x)?;
        self.pixels.get(index).copied()
    }
}

/// Decode one 8-bit, non-interlaced PNG.
///
/// Accepted files contain one `IHDR`, one `IDAT`, and one `IEND`. The color
/// type is RGB or RGBA. An interlaced image, a JPEG, a missing header, or a
/// bad checksum returns [`DecodeError`] instead of panicking.
///
/// # Errors
///
/// Returns [`DecodeError`] when `bytes` are not that PNG subset.
pub fn decode(bytes: &[u8]) -> Result<Image, DecodeError> {
    let mut bytes = bytes;
    read_signature(&mut bytes)?;
    let header = read_ihdr(&mut bytes)?;
    let compressed = read_idat(&mut bytes)?;
    read_iend(&mut bytes)?;
    if !bytes.is_empty() {
        return Err(DecodeError::new());
    }
    let layout = layout(header)?;
    let inflated = inflate::inflate(compressed, layout.total)?;
    let samples = unfilter(&inflated, layout)?;
    image_from(header, &samples)
}

#[derive(Clone, Copy)]
struct Header {
    width: u32,
    height: u32,
    color: ColorKind,
}

#[derive(Clone, Copy)]
enum ColorKind {
    Rgb,
    Rgba,
}

impl ColorKind {
    const fn channels(self) -> usize {
        match self {
            Self::Rgb => 3,
            Self::Rgba => 4,
        }
    }
}

#[derive(Clone, Copy)]
struct Layout {
    height: usize,
    row_bytes: usize,
    stride: usize,
    bpp: usize,
    total: usize,
}

fn read_signature(input: &mut &[u8]) -> Result<(), DecodeError> {
    let got = read_array::<8>(input)?;
    if got != SIGNATURE {
        return Err(DecodeError::new());
    }
    Ok(())
}

fn read_ihdr(input: &mut &[u8]) -> Result<Header, DecodeError> {
    let chunk = read_chunk(input)?;
    if chunk.kind != IHDR {
        return Err(DecodeError::new());
    }
    parse_ihdr(chunk.data)
}

fn parse_ihdr(data: &[u8]) -> Result<Header, DecodeError> {
    if data.len() != 13 {
        return Err(DecodeError::new());
    }
    let mut data = data;
    let width = read_be_u32(&mut data)?;
    let height = read_be_u32(&mut data)?;
    let depth = read_one(&mut data)?;
    let color = read_one(&mut data)?;
    let compression = read_one(&mut data)?;
    let filter = read_one(&mut data)?;
    let interlace = read_one(&mut data)?;
    if depth != 8 || compression != 0 || filter != 0 || interlace != 0 {
        return Err(DecodeError::new());
    }
    if width == 0 || height == 0 {
        return Err(DecodeError::new());
    }
    Ok(Header {
        width,
        height,
        color: color_kind(color)?,
    })
}

fn color_kind(byte: u8) -> Result<ColorKind, DecodeError> {
    match byte {
        2 => Ok(ColorKind::Rgb),
        6 => Ok(ColorKind::Rgba),
        _ => Err(DecodeError::new()),
    }
}

fn read_idat<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], DecodeError> {
    let chunk = read_chunk(input)?;
    if chunk.kind != IDAT {
        return Err(DecodeError::new());
    }
    Ok(chunk.data)
}

fn read_iend(input: &mut &[u8]) -> Result<(), DecodeError> {
    let chunk = read_chunk(input)?;
    if chunk.kind != IEND || !chunk.data.is_empty() {
        return Err(DecodeError::new());
    }
    Ok(())
}

struct Chunk<'a> {
    kind: [u8; 4],
    data: &'a [u8],
}

fn read_chunk<'a>(input: &mut &'a [u8]) -> Result<Chunk<'a>, DecodeError> {
    let len = usize_from(read_be_u32(input)?)?;
    let kind = read_array::<4>(input)?;
    let data = take_slice(input, len)?;
    let crc = read_be_u32(input)?;
    if crc32_pair(&kind, data) != crc {
        return Err(DecodeError::new());
    }
    Ok(Chunk { kind, data })
}

fn layout(header: Header) -> Result<Layout, DecodeError> {
    let width = usize::try_from(header.width).map_err(|_| DecodeError::new())?;
    let height = usize::try_from(header.height).map_err(|_| DecodeError::new())?;
    let bpp = header.color.channels();
    let row_bytes = width.checked_mul(bpp).ok_or(DecodeError::new())?;
    let stride = row_bytes.checked_add(1).ok_or(DecodeError::new())?;
    let total = stride.checked_mul(height).ok_or(DecodeError::new())?;
    if total > MAX_INFLATED_BYTES {
        return Err(DecodeError::new());
    }
    Ok(Layout {
        height,
        row_bytes,
        stride,
        bpp,
        total,
    })
}

fn unfilter(raw: &[u8], layout: Layout) -> Result<Vec<u8>, DecodeError> {
    if raw.len() != layout.total {
        return Err(DecodeError::new());
    }
    let sample_len = layout
        .row_bytes
        .checked_mul(layout.height)
        .ok_or(DecodeError::new())?;
    let mut samples = alloc::vec![0; sample_len];
    let mut previous = alloc::vec![0; layout.row_bytes];
    for row in 0..layout.height {
        unfilter_row(raw, &mut samples, &previous, layout, row)?;
        remember_row(&mut previous, &samples, layout, row)?;
    }
    Ok(samples)
}

fn unfilter_row(
    raw: &[u8],
    samples: &mut [u8],
    previous: &[u8],
    layout: Layout,
    row: usize,
) -> Result<(), DecodeError> {
    let start = row.checked_mul(layout.stride).ok_or(DecodeError::new())?;
    let filter = filter_kind(*raw.get(start).ok_or(DecodeError::new())?)?;
    let src_at = start.checked_add(1).ok_or(DecodeError::new())?;
    let src_end = start.checked_add(layout.stride).ok_or(DecodeError::new())?;
    let src = raw.get(src_at..src_end).ok_or(DecodeError::new())?;
    let dst_at = row
        .checked_mul(layout.row_bytes)
        .ok_or(DecodeError::new())?;
    let dst_end = dst_at
        .checked_add(layout.row_bytes)
        .ok_or(DecodeError::new())?;
    let dst = samples.get_mut(dst_at..dst_end).ok_or(DecodeError::new())?;
    reconstruct(filter, src, previous, layout.bpp, dst)
}

fn remember_row(
    previous: &mut [u8],
    samples: &[u8],
    layout: Layout,
    row: usize,
) -> Result<(), DecodeError> {
    let start = row
        .checked_mul(layout.row_bytes)
        .ok_or(DecodeError::new())?;
    let end = start
        .checked_add(layout.row_bytes)
        .ok_or(DecodeError::new())?;
    let src = samples.get(start..end).ok_or(DecodeError::new())?;
    let dst = previous
        .get_mut(..layout.row_bytes)
        .ok_or(DecodeError::new())?;
    dst.copy_from_slice(src);
    Ok(())
}

#[derive(Clone, Copy)]
enum Filter {
    None,
    Sub,
    Up,
    Average,
    Paeth,
}

fn filter_kind(byte: u8) -> Result<Filter, DecodeError> {
    match byte {
        0 => Ok(Filter::None),
        1 => Ok(Filter::Sub),
        2 => Ok(Filter::Up),
        3 => Ok(Filter::Average),
        4 => Ok(Filter::Paeth),
        _ => Err(DecodeError::new()),
    }
}

fn reconstruct(
    filter: Filter,
    src: &[u8],
    previous: &[u8],
    bpp: usize,
    dst: &mut [u8],
) -> Result<(), DecodeError> {
    if src.len() != dst.len() || previous.len() != dst.len() {
        return Err(DecodeError::new());
    }
    for index in 0..src.len() {
        write_sample(filter, src, previous, bpp, dst, index)?;
    }
    Ok(())
}

fn write_sample(
    filter: Filter,
    src: &[u8],
    previous: &[u8],
    bpp: usize,
    dst: &mut [u8],
    index: usize,
) -> Result<(), DecodeError> {
    let filtered = src.get(index).copied().ok_or(DecodeError::new())?;
    let predicted = predictor(filter, index, bpp, dst, previous);
    let Some(slot) = dst.get_mut(index) else {
        return Err(DecodeError::new());
    };
    *slot = filtered.wrapping_add(predicted);
    Ok(())
}

fn predictor(filter: Filter, index: usize, bpp: usize, dst: &[u8], previous: &[u8]) -> u8 {
    let west = earlier(dst, index, bpp);
    let north = byte_at(previous, index);
    let northwest = earlier(previous, index, bpp);
    match filter {
        Filter::None => 0,
        Filter::Sub => west,
        Filter::Up => north,
        Filter::Average => average(west, north),
        Filter::Paeth => paeth(west, north, northwest),
    }
}

fn earlier(bytes: &[u8], index: usize, bpp: usize) -> u8 {
    let Some(at) = index.checked_sub(bpp) else {
        return 0;
    };
    byte_at(bytes, at)
}

fn byte_at(bytes: &[u8], index: usize) -> u8 {
    bytes.get(index).copied().unwrap_or(0)
}

fn average(west: u8, north: u8) -> u8 {
    let sum = u16::from(west) + u16::from(north);
    u8::try_from(sum / 2).unwrap_or(0)
}

fn paeth(west: u8, north: u8, northwest: u8) -> u8 {
    let origin = i16::from(west) + i16::from(north) - i16::from(northwest);
    let west_distance = (origin - i16::from(west)).abs();
    let north_distance = (origin - i16::from(north)).abs();
    let northwest_distance = (origin - i16::from(northwest)).abs();
    if west_distance <= north_distance && west_distance <= northwest_distance {
        west
    } else if north_distance <= northwest_distance {
        north
    } else {
        northwest
    }
}

fn image_from(header: Header, samples: &[u8]) -> Result<Image, DecodeError> {
    let mut pixels = Vec::new();
    let mut rest = samples;
    while !rest.is_empty() {
        pixels.push(read_pixel(&mut rest, header.color)?);
    }
    if pixels.len() != expected_pixels(header)? {
        return Err(DecodeError::new());
    }
    Ok(Image {
        width: header.width,
        height: header.height,
        pixels,
    })
}

fn expected_pixels(header: Header) -> Result<usize, DecodeError> {
    let width = usize::try_from(header.width).map_err(|_| DecodeError::new())?;
    let height = usize::try_from(header.height).map_err(|_| DecodeError::new())?;
    width.checked_mul(height).ok_or(DecodeError::new())
}

fn read_pixel(rest: &mut &[u8], color: ColorKind) -> Result<Pixel, DecodeError> {
    let red = take_u8(rest)?;
    let green = take_u8(rest)?;
    let blue = take_u8(rest)?;
    let alpha = match color {
        ColorKind::Rgb => 0xff,
        ColorKind::Rgba => take_u8(rest)?,
    };
    Ok(Pixel::rgba(red, green, blue, alpha))
}

fn take_u8(rest: &mut &[u8]) -> Result<u8, DecodeError> {
    let Some((byte, tail)) = rest.split_first() else {
        return Err(DecodeError::new());
    };
    *rest = tail;
    Ok(*byte)
}

fn read_be_u32(input: &mut &[u8]) -> Result<u32, DecodeError> {
    Ok(u32::from_be_bytes(read_array::<4>(input)?))
}

fn read_one(input: &mut &[u8]) -> Result<u8, DecodeError> {
    let bytes = read_array::<1>(input)?;
    Ok(bytes[0])
}

fn read_array<const N: usize>(input: &mut &[u8]) -> Result<[u8; N], DecodeError> {
    let slice = take_slice(input, N)?;
    let mut array = [0; N];
    array.copy_from_slice(slice);
    Ok(array)
}

fn take_slice<'a>(input: &mut &'a [u8], len: usize) -> Result<&'a [u8], DecodeError> {
    let Some((head, tail)) = input.split_at_checked(len) else {
        return Err(DecodeError::new());
    };
    *input = tail;
    Ok(head)
}

fn usize_from(value: u32) -> Result<usize, DecodeError> {
    usize::try_from(value).map_err(|_| DecodeError::new())
}
