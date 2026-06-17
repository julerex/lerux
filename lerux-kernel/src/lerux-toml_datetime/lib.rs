//! Inlined vendored crate (`lerux-toml_datetime`): **build-time only** TOML datetime type.
//!
//! Used only by `build.rs` (via `lerux-toml`) at compile time; never runs on the
//! target. Inlined for a zero-dependency build (see `docs/vendored.md`). Upstream
//! docs follow.
//!
//! ---
//!
//! A [TOML]-compatible datetime type
//!
//! [TOML]: https://github.com/toml-lang/toml

// Makes rustc abort compilation if there are any unsafe blocks in the crate.
// Presence of this annotation is picked up by tools such as cargo-geiger
// and lets them ensure that there is indeed no unsafe code as opposed to
// something they couldn't detect (e.g. unsafe added via macro expansion, etc).

mod datetime;

pub use crate::datetime::Date;
pub use crate::datetime::Datetime;
pub use crate::datetime::DatetimeParseError;
pub use crate::datetime::Offset;
pub use crate::datetime::Time;

#[doc(hidden)]
#[cfg(feature = "serde")]
pub mod __unstable {
    pub use crate::datetime::DatetimeFromString;
    pub use crate::datetime::FIELD;
    pub use crate::datetime::NAME;
}

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
