//! Inlined vendored crate (`lerux-serde_spanned`): **build-time only** spanned values.
//!
//! Used only by `build.rs` (via the TOML crates) at compile time; never runs on
//! the target. Inlined for a zero-dependency build (see `docs/vendored.md`).
//! Upstream docs follow.
//!
//! ---
//!
//! A [serde]-compatible spanned Value
//!
//! This allows capturing the location, in bytes, for a value in the original parsed document for
//! compatible deserializers.
//!
//! [serde]: https://serde.rs/

// Makes rustc abort compilation if there are any unsafe blocks in the crate.
// Presence of this annotation is picked up by tools such as cargo-geiger
// and lets them ensure that there is indeed no unsafe code as opposed to
// something they couldn't detect (e.g. unsafe added via macro expansion, etc).

mod spanned;
pub use crate::spanned::Spanned;

#[doc(hidden)]
#[cfg(feature = "serde")]
pub mod __unstable {
    pub use crate::spanned::is_spanned;
    pub use crate::spanned::END_FIELD;
    pub use crate::spanned::NAME;
    pub use crate::spanned::START_FIELD;
    pub use crate::spanned::VALUE_FIELD;
}

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
