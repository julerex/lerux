//! Stand-in for Ladybird `Libraries/LibWeb`.
//!
//! [`StyleSheet`] is the CSS object model: rules, selectors, and declarations.
//! It does not compute style.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod css;
mod page;

pub use css::{
    Combinator, CompoundSelector, Declaration, ParseError, Property, Selector, SimpleSelector,
    StyleRule, StyleSheet, Value,
};
pub use page::Page;
