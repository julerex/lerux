//! Cascade: UA sheet, author `<style>`, then inline `style=""`.

use lerux_html::{Document, NodeId, TagName};

use crate::css::{
    parse_inline_decls, parse_stylesheet, Decl, DisplayKind, Length, Property, Selector,
    Stylesheet, Value,
};

pub(crate) const BLACK: u32 = 0x0000_0000;
pub(crate) const WHITE: u32 = 0x00FF_FFFF;

const UA_SHEET: &str = r"
html { display: block; color: #000000; background-color: #ffffff; }
head, title, style { display: none; }
body { display: block; margin: 8px; color: #000000; background-color: #ffffff; }
div, p, h1, h2, h3, ul, ol, li, pre { display: block; }
span, a, strong, em, code, img { display: inline; }
p { margin: 16px 0; }
h1 { font-size: 32px; margin: 16px 0; }
h2 { font-size: 24px; margin: 16px 0; }
h3 { font-size: 18px; margin: 16px 0; }
ul, ol { margin: 16px 0; padding: 0 0 0 40px; }
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Edges {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl Edges {
    pub const ZERO: Self = Self {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };

    fn from_px(vals: &[i32]) -> Self {
        match vals {
            [a] => Self {
                top: *a,
                right: *a,
                bottom: *a,
                left: *a,
            },
            [a, b] => Self {
                top: *a,
                right: *b,
                bottom: *a,
                left: *b,
            },
            [a, b, c] => Self {
                top: *a,
                right: *b,
                bottom: *c,
                left: *b,
            },
            [a, b, c, d] => Self {
                top: *a,
                right: *b,
                bottom: *c,
                left: *d,
            },
            _ => Self::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ComputedStyle {
    pub display: DisplayKind,
    pub color: u32,
    pub background: Option<u32>,
    pub font_size: i32,
    pub margin: Edges,
    pub padding: Edges,
    pub width: Option<i32>,
}

impl ComputedStyle {
    pub fn initial() -> Self {
        Self {
            display: DisplayKind::Block,
            color: BLACK,
            background: None,
            font_size: 16,
            margin: Edges::ZERO,
            padding: Edges::ZERO,
            width: None,
        }
    }
}

pub(crate) struct Cascade {
    ua: Stylesheet,
    author: Stylesheet,
}

impl Cascade {
    pub fn new(doc: &Document) -> Self {
        Self {
            ua: parse_stylesheet(UA_SHEET),
            author: parse_stylesheet(&author_css(doc)),
        }
    }

    pub fn compute(&self, doc: &Document, id: NodeId, parent: &ComputedStyle) -> ComputedStyle {
        let node = doc.get(id);
        let Some(tag) = node.tag() else {
            return *parent;
        };
        let mut style = ComputedStyle {
            display: DisplayKind::Block,
            color: parent.color,
            background: None,
            font_size: parent.font_size,
            margin: Edges::ZERO,
            padding: Edges::ZERO,
            width: None,
        };
        apply_matching(&mut style, tag, &self.ua, parent.font_size);
        apply_matching(&mut style, tag, &self.author, parent.font_size);
        if let Some(inline) = node.attr("style") {
            apply_decls(&mut style, &parse_inline_decls(inline), parent.font_size);
        }
        style
    }
}

fn author_css(doc: &Document) -> alloc::string::String {
    let mut out = alloc::string::String::new();
    for i in 0..doc.node_count() {
        let id = NodeId(i);
        if doc.get(id).tag() == Some(TagName::Style) {
            out.push_str(&doc.text_content(id));
        }
    }
    out
}

fn apply_matching(style: &mut ComputedStyle, tag: TagName, sheet: &Stylesheet, parent_font: i32) {
    for rule in &sheet.rules {
        if rule
            .selectors
            .iter()
            .any(|s| matches!(s, Selector::Star) || matches!(s, Selector::Tag(t) if *t == tag))
        {
            apply_decls(style, &rule.decls, parent_font);
        }
    }
}

fn apply_decls(style: &mut ComputedStyle, decls: &[Decl], parent_font: i32) {
    for decl in decls {
        if decl.prop == Property::FontSize
            && let Value::Lengths(v) = &decl.value
            && let Some(len) = v.first()
        {
            style.font_size = len.to_px(parent_font).max(1);
        }
    }
    let em = style.font_size;
    for decl in decls {
        match (&decl.prop, &decl.value) {
            (Property::Color, Value::Color(c)) => style.color = *c,
            (Property::BackgroundColor, Value::Color(c)) => style.background = Some(*c),
            (Property::Display, Value::Display(d)) => style.display = *d,
            (Property::Width, Value::WidthAuto) => style.width = None,
            (Property::Width, Value::Lengths(v)) => {
                if let Some(len) = v.first() {
                    style.width = Some(len.to_px(em).max(0));
                }
            }
            (Property::Margin, Value::Lengths(v)) => {
                style.margin = Edges::from_px(&lengths_px(v, em));
            }
            (Property::Padding, Value::Lengths(v)) => {
                style.padding = Edges::from_px(&lengths_px(v, em));
            }
            (Property::FontSize, _) => {}
            _ => {}
        }
    }
}

fn lengths_px(v: &[Length], em: i32) -> alloc::vec::Vec<i32> {
    v.iter().map(|l| l.to_px(em)).collect()
}
