//! Stand-in for Ladybird `libweb_html_tokenizer` and the DOM.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

/// This value is the root.
pub struct Document {
    _private: (),
}

impl Document {
    pub const fn empty() -> Self {
        Self { _private: () }
    }
}
