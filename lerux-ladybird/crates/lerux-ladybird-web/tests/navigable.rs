use lerux_ladybird_gfx::{Bitmap, Color, FRAME_BYTES};
use lerux_ladybird_html::{Children, Document, Node};
use lerux_ladybird_url::Url;
use lerux_ladybird_web::Page;

const FIRST: &str = "<!DOCTYPE html><html><head><title>one</title></head><body><p><a href=\"second.html\">go</a></p></body></html>";
const SECOND: &str = "<!DOCTYPE html><html><head><title>two</title></head><body><p>top</p><p id=mark>later</p></body></html>";
const FRAMES: &str = include_str!("../../../../support/browser/frames.html");
const CHILD: &str = include_str!("../../../../support/browser/child.html");

fn url(text: &str) -> Url {
    Url::parse(text, None).expect("url")
}

fn href_of(document: &Document) -> &str {
    fn walk(children: Children<'_>) -> Option<&str> {
        for node in children {
            let Node::Element(element) = node else {
                continue;
            };
            if element.name() == "a" {
                return element.href();
            }
            if let Some(href) = walk(element.children()) {
                return Some(href);
            }
        }
        None
    }
    walk(document.children()).expect("link")
}

fn body_for(path: &str) -> Option<String> {
    match path {
        "/first.html" => Some(FIRST.to_string()),
        "/second.html" => Some(SECOND.to_string()),
        "/frames.html" => Some(FRAMES.to_string()),
        "/child.html" => Some(CHILD.to_string()),
        _ => None,
    }
}

fn row_has_ink(bitmap: &Bitmap<'_>, y: i32) -> bool {
    let width = i32::try_from(bitmap.width()).expect("width");
    (0..width).any(|x| {
        bitmap
            .pixel(x, y)
            .is_some_and(|color| color != Color::WHITE)
    })
}

#[test]
fn follow_a_link_then_a_fragment_without_a_second_load() {
    let mut loads = 0u32;
    let mut page = Page::new();
    let loaded = page.navigate(url("https://host:8443/first.html"), |next| {
        loads += 1;
        body_for(next.path())
    });
    assert!(loaded);
    assert_eq!(loads, 1);
    assert_eq!(page.document().title_text(), "one");
    assert_eq!(page.scroll(), 0);

    let href = href_of(page.document()).to_string();
    let loaded = page.follow(&href, |next| {
        loads += 1;
        assert_eq!(next.path(), "/second.html");
        body_for(next.path())
    });
    assert!(loaded);
    assert_eq!(loads, 2);
    assert_eq!(page.document().title_text(), "two");
    assert_eq!(page.url().expect("url").path(), "/second.html");

    let loaded = page.follow("#mark", |_next| {
        loads += 1;
        None
    });
    assert!(loaded);
    assert_eq!(loads, 2);
    assert_eq!(page.url().expect("url").fragment(), Some("mark"));
    assert_eq!(page.document().title_text(), "two");
    assert!(page.scroll() > 0);

    let mut bytes = vec![0u8; FRAME_BYTES];
    let mut bitmap = Bitmap::frame(&mut bytes).expect("frame");
    page.paint(&mut bitmap);
    assert!(row_has_ink(&bitmap, 0));
}

#[test]
fn iframe_is_fetched_after_the_parent_and_paints_its_pixels() {
    let mut paths = Vec::new();
    let mut page = Page::new();
    let loaded = page.navigate(url("https://host:8443/frames.html"), |next| {
        paths.push(next.path().to_string());
        body_for(next.path())
    });
    assert!(loaded);
    assert_eq!(paths, ["/frames.html", "/child.html"]);

    let mut bytes = vec![0u8; FRAME_BYTES];
    let mut bitmap = Bitmap::frame(&mut bytes).expect("frame");
    page.paint(&mut bitmap);
    let line = bitmap.pixel_line().expect("sample");
    assert_eq!(line.as_bytes(), b"pixel ff0000ff\n");
}
