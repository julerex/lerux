//! Paintable tree for one layout.
//!
//! A layout box whose border box has no positive area has no paintable. Its
//! visible descendants are kept by the parent. This subset has no
//! `visibility` property, and `display: none` never produces a layout box.
//!
//! The tree has one stacking context. Its root is the initial containing
//! block, and that block's background is white. Paint walks every background
//! and border, then every line of text. Computed style has no border width,
//! so the border phase adds no pixels. Text uses the frame font. Hit testing
//! walks from front to back and returns no target on a miss.

use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};

use lerux_ladybird_gfx::{glyph, Bitmap, Color as BitmapColor, Rect, GLYPH_WIDTH};
use lerux_ladybird_html::Element;

use crate::{
    layout::{AnonymousBox, BlockBox, LayoutBox, LayoutTree, LineBox},
    style::Color,
};

/// The paintable tree for one layout. It holds the only stacking context.
pub struct PaintableTree<'s, 'd> {
    context: StackingContext<'s, 'd>,
}

/// The stacking context rooted at the initial containing block.
pub struct StackingContext<'s, 'd> {
    root: PaintableBox<'s, 'd>,
}

/// A box that can be seen, or one line of glyphs.
#[derive(Debug)]
pub enum Paintable<'s, 'd> {
    Box(PaintableBox<'s, 'd>),
    Text(PaintableText<'d>),
}

/// A paintable box. The initial containing block has no element.
pub struct PaintableBox<'s, 'd> {
    element: Option<&'s Element<'d>>,
    rect: Rect,
    background: BitmapColor,
    children: Vec<Paintable<'s, 'd>>,
}

/// One line of glyphs in the frame font.
#[derive(Debug)]
pub struct PaintableText<'d> {
    text: &'d str,
    rect: Rect,
    color: BitmapColor,
}

/// The front-most paintable that contains a point.
#[derive(Debug)]
pub enum Hit<'a, 's, 'd> {
    Box(&'a PaintableBox<'s, 'd>),
    Text(&'a PaintableText<'d>),
}

impl<'s, 'd> LayoutTree<'s, 'd> {
    /// Build the paintable tree. Boxes with no area are omitted.
    #[must_use]
    pub fn paintables(&self) -> PaintableTree<'s, 'd> {
        PaintableTree {
            context: StackingContext {
                root: PaintableBox {
                    element: None,
                    rect: self.frame(),
                    background: BitmapColor::WHITE,
                    children: paintables_for(self.children()),
                },
            },
        }
    }
}

impl<'s, 'd> PaintableTree<'s, 'd> {
    #[must_use]
    pub fn stacking_context(&self) -> &StackingContext<'s, 'd> {
        &self.context
    }

    /// Fill backgrounds and borders, then text. Alpha is forced opaque so a
    /// sample can read the first pixel that is not white.
    pub fn paint(&self, bitmap: &mut Bitmap<'_>) {
        paint_background_and_border(&self.context.root, bitmap);
        paint_text(&self.context.root, bitmap);
        bitmap.set_opaque();
    }

    /// The front-most paintable containing `(x, y)`. A miss is [`None`].
    #[must_use]
    pub fn hit_test(&self, x: i32, y: i32) -> Option<Hit<'_, 's, 'd>> {
        hit_box(&self.context.root, x, y)
    }
}

impl<'s, 'd> StackingContext<'s, 'd> {
    #[must_use]
    pub fn root(&self) -> &PaintableBox<'s, 'd> {
        &self.root
    }
}

impl<'s, 'd> PaintableBox<'s, 'd> {
    #[must_use]
    pub fn element(&self) -> Option<&'s Element<'d>> {
        self.element
    }

    #[must_use]
    pub fn rect(&self) -> Rect {
        self.rect
    }

    #[must_use]
    pub fn children(&self) -> &[Paintable<'s, 'd>] {
        &self.children
    }
}

impl Debug for PaintableBox<'_, '_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PaintableBox")
            .field("element", &self.element.map(Element::name))
            .field("rect", &self.rect)
            .field("background", &self.background)
            .field("children", &self.children)
            .finish()
    }
}

impl<'d> PaintableText<'d> {
    #[must_use]
    pub fn text(&self) -> &'d str {
        self.text
    }

    #[must_use]
    pub fn rect(&self) -> Rect {
        self.rect
    }
}

fn paintables_for<'s, 'd>(boxes: &[LayoutBox<'s, 'd>]) -> Vec<Paintable<'s, 'd>> {
    let mut paintables = Vec::new();
    for layout_box in boxes {
        match layout_box {
            LayoutBox::Block(block) => push_block(&mut paintables, block),
            LayoutBox::Anonymous(anonymous) => push_anonymous(&mut paintables, anonymous),
            LayoutBox::Line(line) => push_line(&mut paintables, line),
        }
    }
    paintables
}

fn push_block<'s, 'd>(paintables: &mut Vec<Paintable<'s, 'd>>, block: &BlockBox<'s, 'd>) {
    let children = paintables_for(block.children());
    push_box(
        paintables,
        block.rect(),
        Some(block.element()),
        bitmap_color(block.style().background_color()),
        children,
    );
}

fn push_anonymous<'s, 'd>(
    paintables: &mut Vec<Paintable<'s, 'd>>,
    anonymous: &AnonymousBox<'s, 'd>,
) {
    let children = paintables_for(anonymous.children());
    push_box(
        paintables,
        anonymous.rect(),
        None,
        bitmap_color(Color::TRANSPARENT),
        children,
    );
}

fn push_line<'d>(paintables: &mut Vec<Paintable<'_, 'd>>, line: &LineBox<'d>) {
    if !can_be_visible(line.rect()) {
        return;
    }
    paintables.push(Paintable::Text(PaintableText {
        text: line.text(),
        rect: line.rect(),
        color: bitmap_color(line.color()),
    }));
}

fn push_box<'s, 'd>(
    paintables: &mut Vec<Paintable<'s, 'd>>,
    rect: Rect,
    element: Option<&'s Element<'d>>,
    background: BitmapColor,
    children: Vec<Paintable<'s, 'd>>,
) {
    if !can_be_visible(rect) {
        paintables.extend(children);
        return;
    }
    paintables.push(Paintable::Box(PaintableBox {
        element,
        rect,
        background,
        children,
    }));
}

const fn can_be_visible(rect: Rect) -> bool {
    rect.width > 0 && rect.height > 0
}

const fn bitmap_color(color: Color) -> BitmapColor {
    BitmapColor {
        b: color.blue,
        g: color.green,
        r: color.red,
        a: color.alpha,
    }
}

fn paint_background_and_border(block: &PaintableBox<'_, '_>, bitmap: &mut Bitmap<'_>) {
    fill_background(block, bitmap);
    for child in &block.children {
        match child {
            Paintable::Box(child) => paint_background_and_border(child, bitmap),
            Paintable::Text(_) => {}
        }
    }
}

fn fill_background(block: &PaintableBox<'_, '_>, bitmap: &mut Bitmap<'_>) {
    if block.background.a == 0 {
        return;
    }
    bitmap.fill_rect(block.rect, block.background);
}

fn paint_text(block: &PaintableBox<'_, '_>, bitmap: &mut Bitmap<'_>) {
    for child in &block.children {
        match child {
            Paintable::Text(text) => paint_glyphs(text, bitmap),
            Paintable::Box(child) => paint_text(child, bitmap),
        }
    }
}

fn paint_glyphs(text: &PaintableText<'_>, bitmap: &mut Bitmap<'_>) {
    let mut x = text.rect.x;
    for ch in text.text.chars() {
        if let Some(glyph) = glyph(ch) {
            glyph.paint(bitmap, x, text.rect.y, text.color);
        }
        x = x.saturating_add(GLYPH_WIDTH);
    }
}

fn hit_box<'a, 's, 'd>(block: &'a PaintableBox<'s, 'd>, x: i32, y: i32) -> Option<Hit<'a, 's, 'd>> {
    for child in block.children.iter().rev() {
        if let Some(hit) = hit_paintable(child, x, y) {
            return Some(hit);
        }
    }
    contains(block.rect, x, y).then_some(Hit::Box(block))
}

fn hit_paintable<'a, 's, 'd>(
    paintable: &'a Paintable<'s, 'd>,
    x: i32,
    y: i32,
) -> Option<Hit<'a, 's, 'd>> {
    match paintable {
        Paintable::Box(block) => hit_box(block, x, y),
        Paintable::Text(text) => contains(text.rect, x, y).then_some(Hit::Text(text)),
    }
}

const fn contains(rect: Rect, x: i32, y: i32) -> bool {
    if rect.width <= 0 || rect.height <= 0 {
        return false;
    }
    let Some(right) = rect.x.checked_add(rect.width) else {
        return false;
    };
    let Some(bottom) = rect.y.checked_add(rect.height) else {
        return false;
    };
    x >= rect.x && x < right && y >= rect.y && y < bottom
}
