use lerux_ladybird_gfx::{Bitmap, Color as GfxColor, FRAME_BYTES, FRAME_HEIGHT, FRAME_WIDTH};
use lerux_ladybird_html::Document;
use lerux_ladybird_web::{Hit, Paintable, PaintableTree, StyleSheet, StyledDocument};

const FIXTURE: &str = include_str!("../../../../support/browser/fixture.html");
const L_FIRST_ROW: u8 = 0x30;

fn with_paint(html: &str, author: &str, check: impl FnOnce(&Bitmap<'_>, &PaintableTree<'_, '_>)) {
    let document = Document::parse(html);
    let sheet = StyleSheet::parse(author).expect("author sheet");
    let styled = StyledDocument::new(&document, &sheet);
    let layout = styled.layout();
    let tree = layout.paintables();
    let mut bytes = vec![0u8; FRAME_BYTES];
    let mut bitmap = Bitmap::frame(&mut bytes).expect("frame");
    tree.paint(&mut bitmap);
    check(&bitmap, &tree);
}

fn names<'d>(paintables: &[Paintable<'_, 'd>], found: &mut Vec<&'d str>) {
    for paintable in paintables {
        let Paintable::Box(block) = paintable else {
            continue;
        };
        if let Some(element) = block.element() {
            found.push(element.name());
        }
        names(block.children(), found);
    }
}

fn first_ink_column(row: u8) -> i32 {
    let column = (0u32..8)
        .find(|bit| row & (0x80u8 >> bit) != 0)
        .expect("set bit");
    i32::try_from(column).expect("column fits")
}

fn assert_white_until(bitmap: &Bitmap<'_>, ink_x: i32, ink_y: i32) {
    let width = i32::try_from(bitmap.width()).expect("width");
    for y in 0..ink_y {
        for x in 0..width {
            assert_eq!(bitmap.pixel(x, y), Some(GfxColor::WHITE), "{x},{y}");
        }
    }
    for x in 0..ink_x {
        assert_eq!(bitmap.pixel(x, ink_y), Some(GfxColor::WHITE), "{x}");
    }
}

fn line_for(color: GfxColor) -> [u8; 15] {
    let mut bytes = [0u8; 15];
    bytes[..6].copy_from_slice(b"pixel ");
    write_hex(color.b, &mut bytes[6..8]);
    write_hex(color.g, &mut bytes[8..10]);
    write_hex(color.r, &mut bytes[10..12]);
    write_hex(color.a, &mut bytes[12..14]);
    bytes[14] = b'\n';
    bytes
}

fn write_hex(byte: u8, dst: &mut [u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    dst[0] = HEX[usize::from(byte >> 4)];
    dst[1] = HEX[usize::from(byte & 0x0f)];
}

#[test]
fn fixture_samples_the_paragraph_ink() {
    with_paint(FIXTURE, "", |bitmap, tree| {
        let root = tree.stacking_context().root();
        assert!(root.element().is_none());
        assert_eq!(
            root.rect().width,
            i32::try_from(FRAME_WIDTH).expect("width")
        );
        assert_eq!(
            root.rect().height,
            i32::try_from(FRAME_HEIGHT).expect("height")
        );
        let x = 8 + first_ink_column(L_FIRST_ROW);
        let y = 24;
        assert_white_until(bitmap, x, y);
        assert_eq!(bitmap.pixel(x, y), Some(GfxColor::BLACK));
        let right = i32::try_from(FRAME_WIDTH).expect("width") - 1;
        let bottom = i32::try_from(FRAME_HEIGHT).expect("height") - 1;
        assert_eq!(bitmap.pixel(right, bottom), Some(GfxColor::WHITE));
        let line = bitmap.pixel_line().expect("sample");
        assert_eq!(line.as_bytes(), line_for(GfxColor::BLACK).as_slice());
    });
}

#[test]
fn head_and_title_have_no_paintable() {
    with_paint(FIXTURE, "", |_bitmap, tree| {
        let mut found = Vec::new();
        names(tree.stacking_context().root().children(), &mut found);
        assert_eq!(found, ["html", "body", "p"]);
    });
}

#[test]
fn zero_area_box_has_no_paintable() {
    with_paint(
        "<div class=empty></div>",
        ".empty { display: block; width: 10px }",
        |bitmap, tree| {
            assert!(tree.stacking_context().root().children().is_empty());
            assert!(bitmap.pixel_line().is_none());
        },
    );
}

#[test]
fn padding_makes_an_empty_box_paintable() {
    with_paint(
        "<div class=swatch></div>",
        ".swatch { display: block; width: 4px; padding: 2px; background-color: #00ff00 }",
        |bitmap, tree| {
            let children = tree.stacking_context().root().children();
            let [Paintable::Box(swatch)] = children else {
                panic!("swatch, got {children:?}");
            };
            assert_eq!(swatch.element().expect("div").name(), "div");
            let green = GfxColor {
                b: 0,
                g: 0xff,
                r: 0,
                a: 0xff,
            };
            assert_eq!(bitmap.pixel(0, 0), Some(green));
        },
    );
}

#[test]
fn background_paints_before_text() {
    with_paint(
        FIXTURE,
        "p { background-color: #ff0000 }",
        |bitmap, _tree| {
            let red = GfxColor {
                b: 0,
                g: 0,
                r: 0xff,
                a: 0xff,
            };
            assert_eq!(bitmap.pixel(8, 24), Some(red));
            assert_eq!(bitmap.pixel(10, 24), Some(GfxColor::BLACK));
            assert_eq!(bitmap.pixel(0, 0), Some(GfxColor::WHITE));
        },
    );
}

#[test]
fn inline_color_paints_the_glyph() {
    let html = "<html><body><p><span class=ink>lerux-http-fixture</span></p></body></html>";
    with_paint(html, ".ink { color: #0000ff }", |bitmap, _tree| {
        let blue = GfxColor {
            b: 0xff,
            g: 0,
            r: 0,
            a: 0xff,
        };
        assert_eq!(bitmap.pixel(8, 24), Some(GfxColor::WHITE));
        assert_eq!(bitmap.pixel(10, 24), Some(blue));
    });
}

#[test]
fn hit_test_misses_outside_the_frame_and_hits_the_line() {
    with_paint(FIXTURE, "", |_bitmap, tree| {
        assert!(tree.hit_test(-1, 24).is_none());
        match tree.hit_test(8, 24) {
            Some(Hit::Text(text)) => assert_eq!(text.text(), "lerux-http-fixture"),
            other => panic!("line, got {other:?}"),
        }
        match tree.hit_test(0, 0) {
            Some(Hit::Box(block)) => assert_eq!(block.element().expect("html").name(), "html"),
            other => panic!("html, got {other:?}"),
        }
    });
}
