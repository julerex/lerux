//! Stand-in for Ladybird `Libraries/LibWeb`.
//!
//! [`StyleSheet`] is the CSS object model: rules, selectors, and declarations.
//! [`StyledDocument`] is cold style: the computed style of every element.

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
    Color, ComputedStyle, Display, Edges, StyledDocument, StyledElement, StyledNode, Width,
};
