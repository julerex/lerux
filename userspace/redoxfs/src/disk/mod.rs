use syscall::error::Result;

#[cfg(feature = "std")]
pub use self::cache::DiskCache;
#[cfg(feature = "std")]
pub use self::file::DiskFile;
#[cfg(feature = "std")]
pub use self::io::DiskIo;
pub use self::memory::DiskMemory;   // available without std (pure alloc + Disk impl for in-RAM smoke / tests)
#[cfg(feature = "std")]
pub use self::sparse::DiskSparse;

#[cfg(feature = "std")]
mod cache;
#[cfg(feature = "std")]
mod file;
#[cfg(feature = "std")]
mod io;
mod memory;   // no_std capable (Vec + syscall only)
#[cfg(feature = "std")]
mod sparse;

/// A disk
pub trait Disk {
    /// Read blocks from disk
    ///
    /// # Safety
    /// Unsafe to discourage use, use filesystem wrappers instead
    unsafe fn read_at(&mut self, block: u64, buffer: &mut [u8]) -> Result<usize>;

    /// Write blocks from disk
    ///
    /// # Safety
    /// Unsafe to discourage use, use filesystem wrappers instead
    unsafe fn write_at(&mut self, block: u64, buffer: &[u8]) -> Result<usize>;

    /// Get size of disk in bytes
    fn size(&mut self) -> Result<u64>;
}
