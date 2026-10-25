//! Stand-in for Ladybird `Libraries/LibURL`.
//!
//! `Url::parse` is the only constructor. A base resolves a relative reference.
//! `Url::request_target` is the path and the query.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod url;

pub use url::{ParseError, RequestTarget, Url};
