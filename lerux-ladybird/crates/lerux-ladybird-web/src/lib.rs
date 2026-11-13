//! Stand-in for Ladybird `Libraries/LibWeb`.
//!
//! [`StyleSheet`] is the CSS object model: rules, selectors, and declarations.
//! [`StyledDocument`] is cold style: the computed style of every element.
//! [`LayoutTree`] is the box tree built from that style.
//! [`PaintableTree`] paints that box tree. One stacking context is rooted at
//! the initial containing block. [`Page`] is one top-level navigable.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod css;
mod layout;
mod page;
mod painting;
mod style;

pub use css::{
    Combinator, CompoundSelector, Declaration, ParseError, Property, Selector, SimpleSelector,
    StyleRule, StyleSheet, Value,
};
pub use layout::{AnonymousBox, BlockBox, LayoutBox, LayoutTree, LineBox};
pub use page::Page;
pub use painting::{Hit, Paintable, PaintableBox, PaintableText, PaintableTree, StackingContext};
pub use style::{
    Color, ComputedStyle, Display, Edges, StyledDocument, StyledElement, StyledNode, Width,
};
