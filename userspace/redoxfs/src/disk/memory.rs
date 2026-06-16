use syscall::error::{Error, Result, EIO};

use alloc::{vec, vec::Vec};

use crate::disk::Disk;
use crate::BLOCK_SIZE;

pub struct DiskMemory {
    data: Vec<u8>,
}

impl DiskMemory {
    pub fn new(size: u64) -> DiskMemory {
        DiskMemory {
            data: vec![0; size as usize],
        }
    }
}

impl Disk for DiskMemory {
    /// # Safety
    /// - `block` must correspond to a valid on-disk block index for this in-memory disk.
    /// - `buffer.len()` must be a multiple of BLOCK_SIZE or the caller must handle partial
    ///   block semantics (current RedoxFS always uses full blocks for this trait).
    /// - The buffer must not overlap with any other concurrent access to the same blocks.
    /// - The implementation assumes the disk size is a multiple of BLOCK_SIZE and never
    ///   grows after construction.
    unsafe fn read_at(&mut self, block: u64, buffer: &mut [u8]) -> Result<usize> {
        let offset = (block * BLOCK_SIZE) as usize;
        let end = offset + buffer.len();
        if end > self.data.len() {
            return Err(Error::new(EIO));
        }
        buffer.copy_from_slice(&self.data[offset..end]);
        Ok(buffer.len())
    }

    /// # Safety
    /// Same invariants as `read_at`. Writes must be durable from the caller's perspective
    /// (for DiskMemory this is immediate since it is RAM; real disks require flushes higher up).
    unsafe fn write_at(&mut self, block: u64, buffer: &[u8]) -> Result<usize> {
        let offset = (block * BLOCK_SIZE) as usize;
        let end = offset + buffer.len();
        if end > self.data.len() {
            return Err(Error::new(EIO));
        }
        self.data[offset..end].copy_from_slice(buffer);
        Ok(buffer.len())
    }

    fn size(&mut self) -> Result<u64> {
        Ok(self.data.len() as u64)
    }
}
