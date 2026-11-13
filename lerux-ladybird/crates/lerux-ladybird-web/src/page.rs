//! One top-level navigable.
//!
//! Navigating replaces the document. There is no back stack. A reference that
//! keeps the scheme, host, port, path, and query updates the fragment and the
//! scroll offset, and does not ask for a new body. One `iframe` is a child
//! navigable. Its document is fetched after the parent and painted inside the
//! parent's content box. A `src` that does not parse, or a child fetch that
//! returns no body, leaves that child unloaded. A nested frame is left unloaded.

use alloc::string::String;

use lerux_ladybird_gfx::{Bitmap, Color as BitmapColor, Rect, FRAME_BYTES};
use lerux_ladybird_html::{Children, Document, Node};
use lerux_ladybird_url::Url;

use crate::{
    layout::{LayoutBox, LayoutTree},
    StyleSheet, StyledDocument,
};

/// Hosts one top-level navigable.
pub struct Page {
    url: Option<Url>,
    document: Document,
    scroll: i32,
    child: Option<Document>,
}

impl Page {
    /// A page with no URL and an empty document.
    #[expect(
        clippy::new_without_default,
        reason = "Default would be a second constructor for the same page"
    )]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            url: None,
            document: Document::empty(),
            scroll: 0,
            child: None,
        }
    }

    #[must_use]
    pub const fn document(&self) -> &Document {
        &self.document
    }

    #[must_use]
    pub const fn url(&self) -> Option<&Url> {
        self.url.as_ref()
    }

    /// Vertical scroll of the top-level navigable, in CSS pixels.
    #[must_use]
    pub const fn scroll(&self) -> i32 {
        self.scroll
    }

    /// Replace this navigable with `url`.
    ///
    /// A fragment-only change keeps the document and does not call `load`.
    /// Any other URL replaces the document when `load` returns a body, drops
    /// the previous child, and fetches the first iframe. A `src` that does
    /// not parse, or a child `load` that returns no body, leaves the child
    /// absent and still commits the parent. `load` returns the response body.
    #[must_use]
    pub fn navigate(&mut self, url: Url, mut load: impl FnMut(&Url) -> Option<String>) -> bool {
        if self
            .url
            .as_ref()
            .is_some_and(|current| same_document(current, &url))
        {
            self.url = Some(url);
            self.scroll = self.fragment_scroll();
            return true;
        }
        let Some(html) = load(&url) else {
            return false;
        };
        let document = Document::parse(&html);
        let child = load_child(&url, &document, &mut load);
        self.url = Some(url);
        self.document = document;
        self.scroll = 0;
        self.child = match child {
            ChildLoad::Absent => None,
            ChildLoad::Present(child) => Some(child),
        };
        true
    }

    /// Resolve `href` against the current URL and [`Self::navigate`].
    #[must_use]
    pub fn follow(&mut self, href: &str, load: impl FnMut(&Url) -> Option<String>) -> bool {
        let Some(base) = self.url.clone() else {
            return false;
        };
        let Ok(next) = Url::parse(href, Some(&base)) else {
            return false;
        };
        self.navigate(next, load)
    }

    /// Paint the top-level document, then the child inside the iframe box.
    pub fn paint(&self, bitmap: &mut Bitmap<'_>) {
        let Ok(sheet) = StyleSheet::parse("") else {
            return;
        };
        let styled = StyledDocument::new(&self.document, &sheet);
        let layout = styled.layout();
        let iframe = iframe_rect(layout.children());
        layout.paintables().paint(bitmap);
        if let (Some(child), Some(rect)) = (&self.child, iframe) {
            paint_child(child, &sheet, bitmap, rect);
        }
        scroll_up(bitmap, self.scroll);
    }

    fn fragment_scroll(&self) -> i32 {
        let Some(id) = self.url.as_ref().and_then(Url::fragment) else {
            return 0;
        };
        if id.is_empty() {
            return 0;
        }
        let Ok(sheet) = StyleSheet::parse("") else {
            return 0;
        };
        let styled = StyledDocument::new(&self.document, &sheet);
        element_top(&styled.layout(), id).unwrap_or(0)
    }
}

enum ChildLoad {
    Absent,
    Present(Document),
}

fn load_child(
    base: &Url,
    document: &Document,
    load: &mut impl FnMut(&Url) -> Option<String>,
) -> ChildLoad {
    let Some(src) = first_iframe_src(document) else {
        return ChildLoad::Absent;
    };
    let Ok(url) = Url::parse(src, Some(base)) else {
        return ChildLoad::Absent;
    };
    if same_document(base, &url) {
        return ChildLoad::Absent;
    }
    let Some(html) = load(&url) else {
        return ChildLoad::Absent;
    };
    ChildLoad::Present(Document::parse(&html))
}

fn same_document(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host() == right.host()
        && left.port() == right.port()
        && left.path() == right.path()
        && left.query() == right.query()
}

fn first_iframe_src(document: &Document) -> Option<&str> {
    fn walk(children: Children<'_>) -> Option<&str> {
        for node in children {
            let Node::Element(element) = node else {
                continue;
            };
            if element.name() == "iframe" {
                return element.src();
            }
            if let Some(src) = walk(element.children()) {
                return Some(src);
            }
        }
        None
    }
    walk(document.children())
}

fn iframe_rect(boxes: &[LayoutBox<'_, '_>]) -> Option<Rect> {
    for layout_box in boxes {
        match layout_box {
            LayoutBox::Block(block) => {
                if block.element().name() == "iframe" {
                    return Some(block.rect());
                }
                if let Some(rect) = iframe_rect(block.children()) {
                    return Some(rect);
                }
            }
            LayoutBox::Anonymous(anonymous) => {
                if let Some(rect) = iframe_rect(anonymous.children()) {
                    return Some(rect);
                }
            }
            LayoutBox::Line(_) => {}
        }
    }
    None
}

fn element_top(layout: &LayoutTree<'_, '_>, id: &str) -> Option<i32> {
    fn walk(boxes: &[LayoutBox<'_, '_>], id: &str) -> Option<i32> {
        for layout_box in boxes {
            match layout_box {
                LayoutBox::Block(block) => {
                    if block.element().id() == Some(id) {
                        return Some(block.rect().y);
                    }
                    if let Some(y) = walk(block.children(), id) {
                        return Some(y);
                    }
                }
                LayoutBox::Anonymous(anonymous) => {
                    if let Some(y) = walk(anonymous.children(), id) {
                        return Some(y);
                    }
                }
                LayoutBox::Line(_) => {}
            }
        }
        None
    }
    walk(layout.children(), id)
}

fn paint_child(document: &Document, sheet: &StyleSheet, bitmap: &mut Bitmap<'_>, iframe: Rect) {
    let mut bytes = alloc::vec![0u8; FRAME_BYTES];
    let Some(mut child) = Bitmap::frame(&mut bytes) else {
        return;
    };
    let styled = StyledDocument::new(document, sheet);
    styled.layout().paintables().paint(&mut child);
    bitmap.blit_rect(iframe, &child, Rect::new(0, 0, iframe.width, iframe.height));
}

fn scroll_up(bitmap: &mut Bitmap<'_>, dy: i32) {
    if dy <= 0 {
        return;
    }
    let width = i32::try_from(bitmap.width()).unwrap_or(0);
    let height = i32::try_from(bitmap.height()).unwrap_or(0);
    for y in 0..height {
        let src = y.saturating_add(dy);
        for x in 0..width {
            let color = if src < height {
                bitmap.pixel(x, src).unwrap_or(BitmapColor::WHITE)
            } else {
                BitmapColor::WHITE
            };
            bitmap.set_pixel(x, y, color);
        }
    }
}
