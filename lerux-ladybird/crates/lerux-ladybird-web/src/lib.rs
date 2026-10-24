//! Stand-in for Ladybird `Libraries/LibWeb`.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod page;

pub use page::Page;
