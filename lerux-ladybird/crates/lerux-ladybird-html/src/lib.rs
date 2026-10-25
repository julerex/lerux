//! Modes are doctype, `html`, `head`, `title`, `body`, and `p`.
//! `title` is raw text. Any other tag is an element with no mode of its own.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod document;

pub use document::{Children, Document, Element, Node, Parent, Text};
