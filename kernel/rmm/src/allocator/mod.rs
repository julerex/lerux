//! Physical frame (page) allocators.
//!
//! Re-exports the public frame allocator types and the [`FrameAllocator`] trait.
//! The actual implementations live in the `frame` submodule (bump + buddy).

pub use self::frame::*;

mod frame;
