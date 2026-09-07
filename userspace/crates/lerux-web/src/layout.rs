//! Block-flow layout (no flex, grid, floats, or positioning).

use alloc::{string::String, vec::Vec};

use lerux_html::{Document, NodeId, NodeKind, TagName};

use crate::{
    css::DisplayKind,
    style::{Cascade, ComputedStyle},
};

const IMG_PLACEHOLDER: i32 = 16;

#[derive(Debug, Clone)]
pub(crate) struct GlyphRun {
    pub x: i32,
    pub y: i32,
    pub text: String,
    pub color: u32,
    pub font_size: i32,
}

#[derive(Debug, Clone)]
pub(crate) struct Replaced {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub color: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct LayoutBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub background: Option<u32>,
    pub runs: Vec<GlyphRun>,
    pub replaced: Vec<Replaced>,
    pub children: Vec<LayoutBox>,
}

pub(crate) fn layout_document(doc: &Document, viewport_w: i32, viewport_h: i32) -> LayoutBox {
    let cascade = Cascade::new(doc);
    let initial = ComputedStyle::initial();
    let root = if let Some(html) = doc.first_element(TagName::Html) {
        layout_block(doc, html, &cascade, &initial, 0, 0, viewport_w)
    } else {
        let mut anon = empty_box(0, 0, viewport_w, 0, initial.background);
        let mut y = 0;
        for child in &doc.get(doc.root()).children {
            if let Some(b) = layout_node(doc, *child, &cascade, &initial, 0, y, viewport_w) {
                y = b.y + b.height + margin_bottom_of(doc, *child, &cascade, &initial);
                anon.children.push(b);
            }
        }
        anon.height = y.max(viewport_h);
        anon
    };
    let mut root = root;
    if root.height < viewport_h {
        root.height = viewport_h;
    }
    if root.width < viewport_w {
        root.width = viewport_w;
    }
    root
}

fn margin_bottom_of(doc: &Document, id: NodeId, cascade: &Cascade, parent: &ComputedStyle) -> i32 {
    cascade.compute(doc, id, parent).margin.bottom
}

fn empty_box(x: i32, y: i32, w: i32, h: i32, background: Option<u32>) -> LayoutBox {
    LayoutBox {
        x,
        y,
        width: w,
        height: h,
        background,
        runs: Vec::new(),
        replaced: Vec::new(),
        children: Vec::new(),
    }
}

fn layout_node(
    doc: &Document,
    id: NodeId,
    cascade: &Cascade,
    parent: &ComputedStyle,
    x: i32,
    y: i32,
    avail_w: i32,
) -> Option<LayoutBox> {
    match &doc.get(id).kind {
        NodeKind::Element { .. } => Some(layout_block(doc, id, cascade, parent, x, y, avail_w)),
        NodeKind::Text(_) | NodeKind::Document => None,
    }
}

fn layout_block(
    doc: &Document,
    id: NodeId,
    cascade: &Cascade,
    parent: &ComputedStyle,
    x: i32,
    y: i32,
    avail_w: i32,
) -> LayoutBox {
    let style = cascade.compute(doc, id, parent);
    if style.display == DisplayKind::None {
        return empty_box(x, y, 0, 0, None);
    }
    if style.display == DisplayKind::Inline {
        // Inline elements are handled by the parent's inline formatting context.
        return empty_box(x, y, 0, 0, None);
    }

    let margin = style.margin;
    let pad = style.padding;
    let border_x = x + margin.left;
    let border_y = y + margin.top;
    let width_for_content = style
        .width
        .unwrap_or_else(|| (avail_w - margin.left - margin.right - pad.left - pad.right).max(0));
    let border_w = width_for_content + pad.left + pad.right;
    let content_x = border_x + pad.left;
    let content_w = width_for_content;

    let mut box_ = empty_box(
        border_x,
        border_y,
        border_w,
        pad.top + pad.bottom,
        style.background,
    );
    let mut cy = border_y + pad.top;
    let children = doc.get(id).children.clone();
    let mut i = 0;
    while i < children.len() {
        if is_block_child(doc, children[i], cascade, &style) {
            if let Some(child) =
                layout_node(doc, children[i], cascade, &style, content_x, cy, content_w)
            {
                let bottom = child.y
                    + child.height
                    + cascade.compute(doc, children[i], &style).margin.bottom;
                cy = bottom;
                box_.children.push(child);
            }
            i += 1;
        } else {
            let mut run_ids = Vec::new();
            while i < children.len() && !is_block_child(doc, children[i], cascade, &style) {
                run_ids.push(children[i]);
                i += 1;
            }
            let (runs, replaced, h) =
                layout_inline_run(doc, &run_ids, cascade, &style, content_x, cy, content_w);
            box_.runs.extend(runs);
            box_.replaced.extend(replaced);
            cy += h;
        }
    }
    box_.height = (cy - border_y) + pad.bottom;
    if box_.height < pad.top + pad.bottom {
        box_.height = pad.top + pad.bottom;
    }
    box_
}

fn is_block_child(doc: &Document, id: NodeId, cascade: &Cascade, parent: &ComputedStyle) -> bool {
    match &doc.get(id).kind {
        NodeKind::Element { .. } => {
            let d = cascade.compute(doc, id, parent).display;
            d == DisplayKind::Block || d == DisplayKind::None
        }
        NodeKind::Text(_) => false,
        NodeKind::Document => false,
    }
}

fn layout_inline_run(
    doc: &Document,
    ids: &[NodeId],
    cascade: &Cascade,
    parent: &ComputedStyle,
    content_x: i32,
    content_y: i32,
    content_w: i32,
) -> (Vec<GlyphRun>, Vec<Replaced>, i32) {
    let mut cur = InlineCursor {
        x: content_x,
        y: content_y,
        line_h: 0,
        content_x,
        max_x: content_x + content_w,
        start_y: content_y,
        runs: Vec::new(),
        replaced: Vec::new(),
    };
    for id in ids {
        emit_inline(doc, *id, cascade, parent, &mut cur);
    }
    let h = (cur.y - cur.start_y) + cur.line_h.max(0);
    (cur.runs, cur.replaced, h.max(0))
}

struct InlineCursor {
    x: i32,
    y: i32,
    line_h: i32,
    content_x: i32,
    max_x: i32,
    start_y: i32,
    runs: Vec<GlyphRun>,
    replaced: Vec<Replaced>,
}

fn emit_inline(
    doc: &Document,
    id: NodeId,
    cascade: &Cascade,
    parent: &ComputedStyle,
    cur: &mut InlineCursor,
) {
    match &doc.get(id).kind {
        NodeKind::Text(text) => {
            let preserve = doc
                .get(id)
                .parent
                .is_some_and(|p| doc.get(p).tag() == Some(TagName::Pre));
            push_text(cur, text, parent.color, parent.font_size, preserve);
        }
        NodeKind::Element { name, .. } => {
            let style = cascade.compute(doc, id, parent);
            if style.display == DisplayKind::None {
                return;
            }
            if *name == TagName::Img {
                push_replaced(cur, IMG_PLACEHOLDER, IMG_PLACEHOLDER, 0x0080_8080);
                return;
            }
            for child in &doc.get(id).children.clone() {
                emit_inline(doc, *child, cascade, &style, cur);
            }
        }
        NodeKind::Document => {}
    }
}

fn push_replaced(cur: &mut InlineCursor, w: i32, h: i32, color: u32) {
    wrap_if_needed(cur, w, h);
    cur.replaced.push(Replaced {
        x: cur.x,
        y: cur.y,
        width: w,
        height: h,
        color,
    });
    cur.x += w;
    cur.line_h = cur.line_h.max(h);
}

fn push_text(cur: &mut InlineCursor, text: &str, color: u32, font_size: i32, preserve: bool) {
    if preserve {
        for (i, line) in text.split('\n').enumerate() {
            if i > 0 {
                new_line(cur);
            }
            push_chars(cur, line, color, font_size, true);
        }
        return;
    }
    let collapsed = collapse_ws(text);
    push_chars(cur, &collapsed, color, font_size, false);
}

fn collapse_ws(text: &str) -> String {
    let mut out = String::new();
    let mut prev_ws = false;
    for b in text.bytes() {
        if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
            if !prev_ws {
                out.push(' ');
                prev_ws = true;
            }
        } else {
            out.push(b as char);
            prev_ws = false;
        }
    }
    out
}

fn push_chars(cur: &mut InlineCursor, text: &str, color: u32, font_size: i32, preserve: bool) {
    let mut buf = String::new();
    let mut run_x = cur.x;
    let mut run_y = cur.y;
    for ch in text.chars() {
        if ch == ' ' && !preserve && cur.x == cur.content_x && buf.is_empty() {
            continue;
        }
        let w = font_size;
        let h = font_size;
        if cur.x + w > cur.max_x && cur.x > cur.content_x {
            flush_run(cur, &mut buf, run_x, run_y, color, font_size);
            new_line(cur);
            run_x = cur.x;
            run_y = cur.y;
        }
        if buf.is_empty() {
            run_x = cur.x;
            run_y = cur.y;
        }
        buf.push(ch);
        cur.x += w;
        cur.line_h = cur.line_h.max(h);
    }
    flush_run(cur, &mut buf, run_x, run_y, color, font_size);
}

fn flush_run(cur: &mut InlineCursor, buf: &mut String, x: i32, y: i32, color: u32, font_size: i32) {
    if buf.is_empty() {
        return;
    }
    cur.runs.push(GlyphRun {
        x,
        y,
        text: core::mem::take(buf),
        color,
        font_size,
    });
}

fn wrap_if_needed(cur: &mut InlineCursor, w: i32, h: i32) {
    if cur.x + w > cur.max_x && cur.x > cur.content_x {
        new_line(cur);
    }
    cur.line_h = cur.line_h.max(h);
}

fn new_line(cur: &mut InlineCursor) {
    cur.y += cur.line_h.max(1);
    cur.x = cur.content_x;
    cur.line_h = 0;
}
