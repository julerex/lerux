//! Stand-in for Ladybird `Libraries/LibWeb`.
//!
//! [`StyleSheet`] is the CSS object model: rules, selectors, and declarations.
//! [`StyledDocument`] is cold style: the computed style of every element.
//! [`LayoutTree`] is the box tree built from that style.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod css;
mod layout;
mod page;
mod style;

pub use css::{
    Combinator, CompoundSelector, Declaration, ParseError, Property, Selector, SimpleSelector,
    StyleRule, StyleSheet, Value,
};
pub use layout::{AnonymousBox, BlockBox, LayoutBox, LayoutTree, LineBox};
pub use page::Page;
pub use style::{
    Color, ComputedStyle, Display, Edges, StyledDocument, StyledElement, StyledNode, Width,
};
