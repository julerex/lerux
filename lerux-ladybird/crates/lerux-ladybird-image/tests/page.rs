use lerux_ladybird_image::{paint_page, PageError, Shifts};

const WIDTH: usize = 512;
const HEIGHT: usize = 520;
const UNTOUCHED: u8 = 0x5a;
const PANEL: Shifts = Shifts {
    red: 16,
    green: 8,
    blue: 0,
};

fn at(frame: &[u8], pitch: usize, x: usize, y: usize) -> u32 {
    let start = y * pitch + x * 4;
    u32::from_le_bytes(frame[start..start + 4].try_into().expect("pixel"))
}

#[test]
fn the_heading_sits_above_the_picture() {
    let pitch = WIDTH * 4;
    let mut frame = vec![UNTOUCHED; pitch * HEIGHT];
    paint_page(&mut frame, pitch, PANEL).expect("page");
    assert_eq!(at(&frame, pitch, 0, 0), 0x00ff_ffff, "strip left of the L");
    assert_eq!(at(&frame, pitch, 1, 0), 0, "column 1 of the capital L");
    assert_eq!(at(&frame, pitch, 0, 8), 0x00e2_897d, "png (0, 0)");
    assert_eq!(at(&frame, pitch, 256, 264), 0x00b4_4148, "png (256, 256)");
    assert_eq!(at(&frame, pitch, 511, 519), 0x00b9_4a51, "png (511, 511)");
}

#[test]
fn a_wider_frame_keeps_the_pixels_beside_and_below_the_page() {
    let pitch = (WIDTH + 8) * 4;
    let mut frame = vec![UNTOUCHED; pitch * (HEIGHT + 1)];
    let swapped = Shifts {
        red: 0,
        green: 8,
        blue: 16,
    };
    paint_page(&mut frame, pitch, swapped).expect("page");
    let untouched = u32::from_le_bytes([UNTOUCHED; 4]);
    assert_eq!(
        at(&frame, pitch, 0, 8),
        0x007d_89e2,
        "png (0, 0) in B, G, R order"
    );
    assert_eq!(at(&frame, pitch, WIDTH, 0), untouched, "right of the page");
    assert_eq!(at(&frame, pitch, 0, HEIGHT), untouched, "below the page");
}

#[test]
fn a_short_frame_is_rejected_untouched() {
    let pitch = WIDTH * 4;
    let mut frame = vec![UNTOUCHED; pitch * (HEIGHT - 1)];
    assert_eq!(paint_page(&mut frame, pitch, PANEL), Err(PageError::Frame));
    assert!(frame.iter().all(|&byte| byte == UNTOUCHED));
}
