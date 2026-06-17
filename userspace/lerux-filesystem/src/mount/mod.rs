#[cfg(all(not(target_os = "redox"), not(fuzzing), feature = "fuse"))]
mod fuse;

#[cfg(all(not(target_os = "redox"), fuzzing, feature = "fuse"))]
pub mod fuse;

#[cfg(all(not(target_os = "redox"), feature = "fuse"))]
pub use self::fuse::mount;

#[cfg(all(not(target_os = "redox"), not(fuzzing), not(feature = "fuse"), feature = "std"))]
mod stub;

#[cfg(all(not(target_os = "redox"), fuzzing, not(feature = "fuse"), feature = "std"))]
pub mod stub;

#[cfg(all(not(target_os = "redox"), not(feature = "fuse"), feature = "std"))]
pub use self::stub::mount;

#[cfg(target_os = "redox")]
mod redox;

#[cfg(target_os = "redox")]
pub use self::redox::{mount, mount_via_init};
