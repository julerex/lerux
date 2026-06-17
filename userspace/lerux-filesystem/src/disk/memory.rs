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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BLOCK_SIZE;

    #[test]
    fn read_write_roundtrip() {
        let mut disk = DiskMemory::new(BLOCK_SIZE * 4);
        let block_data = [0xABu8; BLOCK_SIZE as usize];
        unsafe {
            disk.write_at(1, &block_data).unwrap();
        }
        let mut read_buf = [0u8; BLOCK_SIZE as usize];
        unsafe {
            disk.read_at(1, &mut read_buf).unwrap();
        }
        assert_eq!(read_buf, block_data);
    }

    #[test]
    fn read_past_end_fails() {
        let mut disk = DiskMemory::new(BLOCK_SIZE);
        let mut buf = [0u8; BLOCK_SIZE as usize];
        assert!(unsafe { disk.read_at(1, &mut buf) }.is_err());
    }

    #[test]
    fn write_past_end_fails() {
        let mut disk = DiskMemory::new(BLOCK_SIZE);
        let buf = [0u8; BLOCK_SIZE as usize];
        assert!(unsafe { disk.write_at(1, &buf) }.is_err());
    }

    #[test]
    fn size_matches_allocation() {
        let mut disk = DiskMemory::new(BLOCK_SIZE * 8);
        assert_eq!(disk.size().unwrap(), BLOCK_SIZE * 8);
    }
}
