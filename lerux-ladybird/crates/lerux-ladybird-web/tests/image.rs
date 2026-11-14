use lerux_ladybird_gfx::{Bitmap, FRAME_BYTES};
use lerux_ladybird_image::decode_png;
use lerux_ladybird_url::Url;
use lerux_ladybird_web::{Page, Raster};

const PAGE: &str = include_str!("../../../../support/browser/image.html");
const GRID: &[u8] = include_bytes!("../../../../support/browser/grid.png");

fn url(text: &str) -> Url {
    Url::parse(text, None).expect("url")
}

fn grid() -> Raster {
    let image = decode_png(GRID).expect("grid");
    Raster {
        width: i32::try_from(image.width()).expect("width"),
        height: i32::try_from(image.height()).expect("height"),
        pixels: image.pixels().to_vec(),
    }
}

#[test]
fn the_img_blits_the_named_top_left() {
    let mut page = Page::new();
    let loaded = page.navigate_with(
        url("https://host:8443/image.html"),
        |next| {
            assert_eq!(next.path(), "/image.html");
            Some(PAGE.to_string())
        },
        |next| {
            assert_eq!(next.path(), "/grid.png");
            Some(grid())
        },
    );
    assert!(loaded);
    assert_eq!(page.document().title_text(), "image");

    let mut bytes = vec![0u8; FRAME_BYTES];
    let mut bitmap = Bitmap::frame(&mut bytes).expect("frame");
    page.paint(&mut bitmap);
    let line = bitmap.pixel_line().expect("sample");
    assert_eq!(line.as_bytes(), b"pixel 05c809ff\n");
}

#[test]
fn a_rejected_image_leaves_the_rest_of_the_page() {
    let mut page = Page::new();
    let html = "<!DOCTYPE html><html><head><title>kept</title></head><body><img src=\"/nope.png\"><p>x</p></body></html>";
    let mut asked = 0u32;
    let loaded = page.navigate_with(
        url("https://host:8443/kept.html"),
        |_| Some(html.to_string()),
        |_| {
            asked += 1;
            None
        },
    );
    assert!(loaded);
    assert_eq!(asked, 1);
    assert_eq!(page.document().title_text(), "kept");

    let mut bytes = vec![0u8; FRAME_BYTES];
    let mut bitmap = Bitmap::frame(&mut bytes).expect("frame");
    page.paint(&mut bitmap);
    let line = bitmap.pixel_line().expect("sample");
    assert_ne!(line.as_bytes(), b"pixel 05c809ff\n");
}

#[test]
fn a_fragment_below_an_image_includes_the_image_height() {
    let empty = fragment_scroll(None);
    let tall = fragment_scroll(Some(40));
    assert_eq!(tall.saturating_sub(empty), 40);
}

fn fragment_scroll(height: Option<i32>) -> i32 {
    let mut page = Page::new();
    let html =
        "<!DOCTYPE html><html><body><img src=\"/grid.png\"><p id=\"mark\">x</p></body></html>";
    let loaded = page.navigate_with(
        url("https://host:8443/page.html"),
        |_| Some(html.to_string()),
        |_| height.map(raster),
    );
    assert!(loaded);
    let loaded = page.navigate_with(url("https://host:8443/page.html#mark"), |_| None, |_| None);
    assert!(loaded);
    page.scroll()
}

fn raster(height: i32) -> Raster {
    let pixels = usize::try_from(height).expect("height") * 16;
    Raster {
        width: 4,
        height,
        pixels: vec![0; pixels],
    }
}
