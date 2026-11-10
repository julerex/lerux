use core::fmt::{self, Display, Formatter};

/// The bytes are not an 8-bit, non-interlaced RGB or RGBA PNG.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeError {
    _private: (),
}

impl DecodeError {
    pub(crate) const fn new() -> Self {
        Self { _private: () }
    }
}

impl Display for DecodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("unsupported image")
    }
}

impl core::error::Error for DecodeError {}
