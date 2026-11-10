//! Stand-in for Ladybird `Libraries/LibWeb`.
//!
//! [`StyleSheet`] is the CSS object model: rules, selectors, and declarations.
//! [`ComputedStyle`] is the cold cascade over that model.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod css;
mod page;
mod style;

pub use css::{
    Combinator, CompoundSelector, Declaration, ParseError, Property, Selector, SimpleSelector,
    StyleRule, StyleSheet, Value,
};
pub use page::Page;
pub use style::{
    compute_style, style_document, BoxEdges, Color, ComputedStyle, CustomProperty, Display, Edges,
    FontSize, Length, Rgba, StyleElement, StyledElement, Width,
};
