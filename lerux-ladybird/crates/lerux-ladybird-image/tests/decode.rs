use lerux_ladybird_image::{decode, Image, Pixel};

/// Pixel (3, 2) is opaque blue: R=0, G=0, B=255, A=255.
/// Packed BGRA, the frame byte order, is `ff0000ff`.
const SWATCH: [Pixel; 40] = [
    Pixel::rgb(0xff, 0x00, 0x00),
    Pixel::rgb(0x00, 0xff, 0x00),
    Pixel::rgb(0x00, 0x00, 0xff),
    Pixel::rgb(0xff, 0xff, 0xff),
    Pixel::rgb(0x00, 0x00, 0x00),
    Pixel::rgb(0xff, 0xff, 0x00),
    Pixel::rgb(0x00, 0xff, 0xff),
    Pixel::rgb(0xff, 0x00, 0xff),
    Pixel::rgb(0x11, 0x22, 0x33),
    Pixel::rgb(0x44, 0x55, 0x66),
    Pixel::rgb(0x77, 0x88, 0x99),
    Pixel::rgb(0xaa, 0xbb, 0xcc),
    Pixel::rgb(0xde, 0xad, 0xbe),
    Pixel::rgb(0x01, 0x23, 0x45),
    Pixel::rgb(0x67, 0x89, 0xab),
    Pixel::rgb(0xcd, 0xef, 0x10),
    Pixel::rgb(0x20, 0x20, 0x20),
    Pixel::rgb(0x40, 0x40, 0x40),
    Pixel::rgb(0x60, 0x60, 0x60),
    Pixel::rgb(0x00, 0x00, 0xff),
    Pixel::rgb(0x80, 0x00, 0x00),
    Pixel::rgb(0x00, 0x80, 0x00),
    Pixel::rgb(0x00, 0x00, 0x80),
    Pixel::rgb(0xf0, 0xf0, 0xf0),
    Pixel::rgb(0x01, 0x02, 0x03),
    Pixel::rgb(0x04, 0x05, 0x06),
    Pixel::rgb(0x07, 0x08, 0x09),
    Pixel::rgb(0x0a, 0x0b, 0x0c),
    Pixel::rgb(0x0d, 0x0e, 0x0f),
    Pixel::rgb(0x10, 0x11, 0x12),
    Pixel::rgb(0x13, 0x14, 0x15),
    Pixel::rgb(0x16, 0x17, 0x18),
    Pixel::rgb(0xff, 0x80, 0x00),
    Pixel::rgb(0x80, 0xff, 0x00),
    Pixel::rgb(0x00, 0xff, 0x80),
    Pixel::rgb(0x00, 0x80, 0xff),
    Pixel::rgb(0x80, 0x00, 0xff),
    Pixel::rgb(0xff, 0x00, 0x80),
    Pixel::rgb(0x10, 0x20, 0x30),
    Pixel::rgb(0x40, 0x50, 0x60),
];

#[test]
fn swatch_names_pixel_3_2_as_opaque_blue() {
    let image = decode_swatch();
    let pixel = image.pixel(3, 2).expect("pixel 3,2");
    assert_eq!(pixel, Pixel::rgba(0x00, 0x00, 0xff, 0xff));
    assert_eq!(pixel.bgra(), [0xff, 0x00, 0x00, 0xff]);
    assert_eq!(image.pixels(), &SWATCH);
    assert_eq!(image.pixel(8, 0), None);
    assert_eq!(image.pixel(0, 5), None);
}

#[test]
fn rgba_keeps_the_alpha_byte() {
    let image = decode(include_bytes!("fixtures/alpha.png")).expect("alpha");
    assert_eq!(image.width(), 2);
    assert_eq!(image.height(), 2);
    assert_eq!(image.pixel(0, 0), Some(Pixel::rgba(255, 0, 0, 128)));
    assert_eq!(image.pixel(1, 0), Some(Pixel::rgba(0, 255, 0, 64)));
    assert_eq!(image.pixel(0, 1), Some(Pixel::rgba(0, 0, 255, 0)));
    assert_eq!(image.pixel(1, 1), Some(Pixel::rgba(255, 255, 255, 255)));
}

#[test]
fn interlaced_png_is_an_error() {
    assert!(decode(include_bytes!("fixtures/interlace.png")).is_err());
}

#[test]
fn jpeg_is_an_error() {
    assert!(decode(&[0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10]).is_err());
}

#[test]
fn missing_header_is_an_error() {
    assert!(decode(&SIGNATURE).is_err());
    assert!(decode(&[]).is_err());
}

#[test]
fn bad_chunk_checksum_is_an_error() {
    let mut bytes = include_bytes!("fixtures/swatch.png").to_vec();
    let Some(last) = bytes.last_mut() else {
        panic!("png");
    };
    *last ^= 0xff;
    assert!(decode(&bytes).is_err());
}

#[test]
fn bad_adler_checksum_is_an_error() {
    assert!(decode(include_bytes!("fixtures/bad-adler.png")).is_err());
}

fn decode_swatch() -> Image {
    let image = decode(include_bytes!("fixtures/swatch.png")).expect("swatch");
    assert_eq!(image.width(), 8);
    assert_eq!(image.height(), 5);
    image
}

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
