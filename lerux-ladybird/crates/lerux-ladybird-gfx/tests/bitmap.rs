use core::mem::{align_of, offset_of, size_of};

use lerux_ladybird_gfx::{
    control_page_zeroed, glyph, Bitmap, Color, ControlPage, Rect, CONTROL_REGION_SIZE,
    CONTROL_STATE_IDLE, CONTROL_STATE_PAINTED, FRAME_BYTES, FRAME_HEIGHT, FRAME_WIDTH,
    GLYPH_HEIGHT, GLYPH_WIDTH, TITLE_CAP,
};

fn frame() -> Vec<u8> {
    vec![0u8; FRAME_BYTES]
}

fn white(bitmap: &mut Bitmap<'_>) {
    bitmap.fill_rect(
        Rect::new(0, 0, FRAME_WIDTH as i32, FRAME_HEIGHT as i32),
        Color::WHITE,
    );
}

#[test]
fn fill_rect_and_one_glyph_sample_black() {
    let mut storage = frame();
    let mut bitmap = Bitmap::frame(&mut storage).expect("frame");
    white(&mut bitmap);
    let swatch = Color {
        b: 0x10,
        g: 0x20,
        r: 0x30,
        a: 0xff,
    };
    bitmap.fill_rect(Rect::new(0, 16, 3, 2), swatch);
    assert_eq!(bitmap.pixel(0, 16), Some(swatch));
    assert_eq!(bitmap.pixel(2, 17), Some(swatch));
    assert_eq!(bitmap.pixel(3, 16), Some(Color::WHITE));
    assert_eq!(bitmap.pixel(0, 18), Some(Color::WHITE));

    glyph('l')
        .expect("l")
        .paint(&mut bitmap, 0, 0, Color::BLACK);
    bitmap.set_opaque();
    assert_eq!(bitmap.pixel(0, 0), Some(Color::WHITE));
    assert_eq!(bitmap.pixel(1, 0), Some(Color::WHITE));
    assert_eq!(bitmap.pixel(2, 0), Some(Color::BLACK));
    assert_eq!(bitmap.pixel(3, 0), Some(Color::BLACK));
    let line = bitmap.pixel_line().expect("ink");
    assert_eq!(line.as_bytes(), b"pixel 000000ff\n");
}

#[test]
fn fill_rect_clips_and_set_pixel_ignores_the_outside() {
    let mut storage = frame();
    let mut bitmap = Bitmap::frame(&mut storage).expect("frame");
    white(&mut bitmap);
    bitmap.fill_rect(Rect::new(-2, -3, 4, 5), Color::BLACK);
    assert_eq!(bitmap.pixel(0, 0), Some(Color::BLACK));
    assert_eq!(bitmap.pixel(1, 1), Some(Color::BLACK));
    assert_eq!(bitmap.pixel(2, 0), Some(Color::WHITE));
    bitmap.fill_rect(Rect::new(0, 0, 0, 8), Color::BLACK);
    bitmap.set_pixel(-1, 0, Color::BLACK);
    bitmap.set_pixel(FRAME_WIDTH as i32, 0, Color::BLACK);
    assert_eq!(bitmap.pixel(2, 0), Some(Color::WHITE));
}

#[test]
fn set_pixel_is_the_sample_when_it_is_not_white() {
    let mut storage = frame();
    let mut bitmap = Bitmap::frame(&mut storage).expect("frame");
    white(&mut bitmap);
    assert!(bitmap.pixel_line().is_none());
    let ink = Color {
        b: 0xaa,
        g: 0xbb,
        r: 0xcc,
        a: 0x00,
    };
    bitmap.set_pixel(4, 5, ink);
    assert_eq!(bitmap.first_non_white(), Some([0xaa, 0xbb, 0xcc, 0x00]));
    bitmap.set_opaque();
    assert_eq!(bitmap.pixel(4, 5), Some(Color { a: 0xff, ..ink }));
    assert_eq!(
        bitmap.pixel_line().expect("ink").as_bytes(),
        b"pixel aabbccff\n"
    );
}

#[test]
fn blit_rect_copies_the_glyph() {
    let mut src_bytes = frame();
    let mut dst_bytes = frame();
    let mut src = Bitmap::frame(&mut src_bytes).expect("src");
    let mut dst = Bitmap::frame(&mut dst_bytes).expect("dst");
    white(&mut src);
    white(&mut dst);
    glyph('l').expect("l").paint(&mut src, 0, 0, Color::BLACK);
    dst.blit_rect(
        Rect::new(4, 1, GLYPH_WIDTH, GLYPH_HEIGHT),
        &src,
        Rect::new(0, 0, GLYPH_WIDTH, GLYPH_HEIGHT),
    );
    assert_eq!(dst.pixel(4, 1), Some(Color::WHITE));
    assert_eq!(dst.pixel(6, 1), Some(Color::BLACK));
    assert_eq!(
        dst.pixel_line().expect("ink").as_bytes(),
        b"pixel 000000ff\n"
    );
    // Column 4 of `l` is ink. An origin of x=-4 places that column at x=0.
    dst.blit_rect(
        Rect::new(-4, 0, GLYPH_WIDTH, GLYPH_HEIGHT),
        &src,
        Rect::new(0, 0, GLYPH_WIDTH, GLYPH_HEIGHT),
    );
    assert_eq!(dst.pixel(0, 6), Some(Color::BLACK));
}

#[test]
fn frame_rejects_an_empty_side_or_a_short_buffer() {
    let mut short = vec![0u8; FRAME_BYTES - 1];
    assert!(Bitmap::frame(&mut short).is_none());
    let mut storage = frame();
    assert!(Bitmap::new(&mut storage, 0, 1).is_none());
}

#[test]
fn fixture_text_has_glyphs_and_other_characters_do_not() {
    for ch in "lerux-http-fixture".chars() {
        let mut storage = frame();
        let mut bitmap = Bitmap::frame(&mut storage).expect("frame");
        white(&mut bitmap);
        glyph(ch)
            .expect("glyph")
            .paint(&mut bitmap, 0, 0, Color::BLACK);
        assert!(bitmap.pixel_line().is_some(), "{ch}");
    }
    for ch in ['a', ' ', '0', 'L'] {
        assert!(glyph(ch).is_none(), "{ch}");
    }
}

#[test]
fn control_page_is_zeroed_then_assigned() {
    assert_eq!(offset_of!(ControlPage, state), 0);
    assert_eq!(offset_of!(ControlPage, title_len), 4);
    assert_eq!(offset_of!(ControlPage, title), 8);
    assert!(size_of::<ControlPage>() <= CONTROL_REGION_SIZE);
    assert!(align_of::<ControlPage>() <= 8);

    let mut page = control_page_zeroed();
    assert_eq!(page.state, CONTROL_STATE_IDLE);
    assert_eq!(page.title_len, 0);
    assert_eq!(page.title(), b"");
    assert!(page.title.iter().all(|byte| *byte == 0));

    page.set_title(b"lerux");
    page.state = CONTROL_STATE_PAINTED;
    assert_eq!(page.title(), b"lerux");
    assert_eq!(page.title_len, 5);
    assert_eq!(page.state, CONTROL_STATE_PAINTED);
    assert!(page.title[5..].iter().all(|byte| *byte == 0));

    page.set_title(b"lerux-http-fixture");
    assert_eq!(page.title(), b"lerux-http-fixture");

    let long = vec![b'a'; TITLE_CAP + 8];
    page.set_title(&long);
    assert_eq!(page.title().len(), TITLE_CAP);
    assert!(page.title().iter().all(|byte| *byte == b'a'));
    page.set_title(b"lerux");
    assert_eq!(page.title(), b"lerux");
    assert!(page.title[5..].iter().all(|byte| *byte == 0));

    page.title_len = u32::MAX;
    assert_eq!(page.title().len(), TITLE_CAP);
}
