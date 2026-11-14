use lerux_ladybird_image::{decode_png, Image, PngError};

const LENNA: &[u8] = include_bytes!("../../../../support/browser/lenna.png");

/// Four by five RGB, row `y` filtered with PNG filter `y` (None, Sub, Up,
/// Average, Paeth). The pixels are [`grid`]. Python `zlib.compress(raw, 0)`
/// wrote this one stored block.
const GRID_STORED: &[u8] = &[
    0x78, 0x01, 0x01, 0x41, 0x00, 0xbe, 0xff, 0x00, 0x09, 0xc8, 0x05, 0x31, 0xaa, 0x05, 0x59, 0x8c,
    0x05, 0x81, 0x6e, 0x05, 0x01, 0x16, 0xc1, 0x05, 0x28, 0xe2, 0x25, 0x28, 0xe2, 0x25, 0x28, 0xe2,
    0x25, 0x02, 0x0d, 0xf9, 0x00, 0x0d, 0xf9, 0x25, 0x0d, 0xf9, 0x4a, 0x0d, 0xf9, 0x6f, 0x03, 0x1f,
    0x56, 0x03, 0x1b, 0xee, 0x4a, 0x1b, 0xee, 0x5d, 0x1b, 0xee, 0x6f, 0x04, 0x0d, 0xf9, 0x00, 0x0d,
    0xf9, 0x25, 0x0d, 0xf9, 0x4a, 0x0d, 0xf9, 0x94, 0xab, 0xfc, 0x16, 0xc2,
];

/// The same scanlines from `zlib.compressobj(9, DEFLATED, 15, 9, Z_FIXED)`,
/// one fixed Huffman block.
const GRID_FIXED: &[u8] = &[
    0x78, 0x01, 0x63, 0xe0, 0x3c, 0xc1, 0x6a, 0xb8, 0x8a, 0x35, 0xb2, 0x87, 0xb5, 0x31, 0x8f, 0x95,
    0x51, 0xec, 0x20, 0xab, 0xc6, 0x23, 0x55, 0x08, 0x62, 0xe2, 0xfd, 0xc9, 0xc0, 0xfb, 0x53, 0x95,
    0xf7, 0xa7, 0x17, 0xef, 0xcf, 0x7c, 0x66, 0xf9, 0x30, 0x66, 0xe9, 0x77, 0x5e, 0xd2, 0xef, 0x62,
    0xa5, 0xdf, 0xe5, 0xb3, 0x20, 0xc9, 0x4c, 0x01, 0x00, 0xab, 0xfc, 0x16, 0xc2,
];

/// One row of eight `c01020` pixels, filter None, fixed Huffman. 25 bytes
/// inflate from 10 bytes of codes, so a back reference overlaps its own output.
const RUN_FIXED: &[u8] = &[
    0x78, 0x01, 0x63, 0x38, 0x20, 0xa0, 0x80, 0x15, 0x01, 0x00, 0x62, 0xd9, 0x07, 0x81,
];

/// One RGBA row, `11223380` then `44556600`, filter None, fixed Huffman.
const RGBA_FIXED: &[u8] = &[
    0x78, 0x01, 0x63, 0x10, 0x54, 0x32, 0x6e, 0x70, 0x09, 0x4d, 0x63, 0x00, 0x00, 0x08, 0x0c, 0x01,
    0xe6,
];

const RGB: u8 = 2;
const RGBA: u8 = 6;
/// Signature, then the `IHDR` length, type, and 13 data bytes.
const IHDR_CRC: usize = 8 + 4 + 4 + 13;

fn grid(x: u8, y: u8) -> u32 {
    let red = x
        .wrapping_mul(40)
        .wrapping_add(y.wrapping_mul(13))
        .wrapping_add(9);
    let green = 200u8
        .wrapping_sub(x.wrapping_mul(30))
        .wrapping_sub(y.wrapping_mul(7));
    let blue = x.wrapping_mul(y).wrapping_mul(37).wrapping_add(5);
    u32::from_be_bytes([red, green, blue, 0xff])
}

/// The pixel at `(x, y)` as `0xRRGGBBAA`.
fn rgba(image: &Image, x: u32, y: u32) -> u32 {
    let at = usize::try_from((y * image.width() + x) * 4).expect("offset");
    let [blue, green, red, alpha] = image.pixels()[at..at + 4] else {
        panic!("four bytes");
    };
    u32::from_be_bytes([red, green, blue, alpha])
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                0xedb8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    out.extend_from_slice(&u32::try_from(data.len()).expect("length").to_be_bytes());
    let body = [kind.as_slice(), data].concat();
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

fn png(width: u32, height: u32, depth: u8, color: u8, interlace: u8, zlib: &[u8]) -> Vec<u8> {
    let mut out = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[depth, color, 0, 0, interlace]);
    chunk(&mut out, *b"IHDR", &header);
    chunk(&mut out, *b"IDAT", zlib);
    chunk(&mut out, *b"IEND", &[]);
    out
}

fn assert_grid(image: &Image) {
    assert_eq!((image.width(), image.height()), (4, 5));
    for y in 0..5u8 {
        for x in 0..4u8 {
            assert_eq!(
                rgba(image, u32::from(x), u32::from(y)),
                grid(x, y),
                "({x}, {y})"
            );
        }
    }
}

#[test]
fn the_guest_grid_names_its_top_left() {
    let image = decode_png(include_bytes!("../../../../support/browser/grid.png")).expect("grid");
    assert_eq!((image.width(), image.height()), (4, 5));
    assert_eq!(rgba(&image, 0, 0), grid(0, 0));
    assert_eq!(image.pixels()[..4], [0x05, 0xc8, 0x09, 0xff]);
}

#[test]
fn lenna_matches_the_python_decode() {
    let image = decode_png(LENNA).expect("lenna");
    assert_eq!((image.width(), image.height()), (512, 512));
    assert_eq!(image.pixels().len(), 512 * 512 * 4);
    assert_eq!(rgba(&image, 0, 0), 0xe289_7dff);
    assert_eq!(rgba(&image, 256, 256), 0xb441_48ff);
    assert_eq!(rgba(&image, 511, 511), 0xb94a_51ff);
}

#[test]
fn a_stored_block_undoes_every_filter() {
    assert_grid(&decode_png(&png(4, 5, 8, RGB, 0, GRID_STORED)).expect("stored"));
}

#[test]
fn a_fixed_huffman_block_undoes_every_filter() {
    assert_grid(&decode_png(&png(4, 5, 8, RGB, 0, GRID_FIXED)).expect("fixed"));
}

#[test]
fn a_back_reference_repeats_its_own_output() {
    let image = decode_png(&png(8, 1, 8, RGB, 0, RUN_FIXED)).expect("run");
    for x in 0..8 {
        assert_eq!(rgba(&image, x, 0), 0xc010_20ff, "{x}");
    }
}

#[test]
fn rgba_keeps_its_alpha() {
    let image = decode_png(&png(2, 1, 8, RGBA, 0, RGBA_FIXED)).expect("rgba");
    assert_eq!(rgba(&image, 0, 0), 0x1122_3380);
    assert_eq!(rgba(&image, 1, 0), 0x4455_6600);
}

#[test]
fn a_bad_signature_is_rejected() {
    let mut bytes = LENNA.to_vec();
    bytes[1] = b'Q';
    assert_eq!(decode_png(&bytes).err(), Some(PngError::Signature));
}

#[test]
fn a_bad_chunk_crc_is_rejected() {
    let mut bytes = png(4, 5, 8, RGB, 0, GRID_STORED);
    bytes[IHDR_CRC] ^= 1;
    assert_eq!(decode_png(&bytes).err(), Some(PngError::Crc));
}

#[test]
fn sixteen_bit_samples_are_rejected() {
    let bytes = png(4, 5, 16, RGB, 0, GRID_STORED);
    assert_eq!(decode_png(&bytes).err(), Some(PngError::Unsupported));
}

#[test]
fn interlacing_is_rejected() {
    let bytes = png(4, 5, 8, RGB, 1, GRID_STORED);
    assert_eq!(decode_png(&bytes).err(), Some(PngError::Unsupported));
}

#[test]
fn a_bad_adler_is_rejected() {
    let mut zlib = GRID_STORED.to_vec();
    *zlib.last_mut().expect("trailer") ^= 1;
    let bytes = png(4, 5, 8, RGB, 0, &zlib);
    assert_eq!(decode_png(&bytes).err(), Some(PngError::Adler));
}
