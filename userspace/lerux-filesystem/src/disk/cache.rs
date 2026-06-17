use std::collections::{HashMap, VecDeque};
use std::{cmp, ptr};
use syscall::error::Result;

use crate::disk::Disk;
use crate::BLOCK_SIZE;

fn copy_memory(src: &[u8], dest: &mut [u8]) -> usize {
    let len = cmp::min(src.len(), dest.len());
    unsafe { ptr::copy(src.as_ptr(), dest.as_mut_ptr(), len) };
    len
}

pub struct DiskCache<T> {
    inner: T,
    cache: HashMap<u64, [u8; BLOCK_SIZE as usize]>,
    order: VecDeque<u64>,
    size: usize,
}

impl<T: Disk> DiskCache<T> {
    pub fn new(inner: T) -> Self {
        // 16 MB cache
        let size = 16 * 1024 * 1024 / BLOCK_SIZE as usize;
        DiskCache {
            inner,
            cache: HashMap::with_capacity(size),
            order: VecDeque::with_capacity(size),
            size,
        }
    }

    fn insert(&mut self, i: u64, data: [u8; BLOCK_SIZE as usize]) {
        while self.order.len() >= self.size {
            let removed = self.order.pop_front().unwrap();
            self.cache.remove(&removed);
        }

        self.cache.insert(i, data);
        self.order.push_back(i);
    }
}

impl<T: Disk> Disk for DiskCache<T> {
    unsafe fn read_at(&mut self, block: u64, buffer: &mut [u8]) -> Result<usize> {
        // println!("Cache read at {}", block);

        let mut read = 0;
        let mut failed = false;
        for i in 0..buffer.len().div_ceil(BLOCK_SIZE as usize) {
            let block_i = block + i as u64;

            let buffer_i = i * BLOCK_SIZE as usize;
            let buffer_j = cmp::min(buffer_i + BLOCK_SIZE as usize, buffer.len());
            let buffer_slice = &mut buffer[buffer_i..buffer_j];

            if let Some(cache_buf) = self.cache.get_mut(&block_i) {
                read += copy_memory(cache_buf, buffer_slice);
            } else {
                failed = true;
                break;
            }
        }

        if failed {
            self.inner.read_at(block, buffer)?;

            read = 0;
            for i in 0..buffer.len().div_ceil(BLOCK_SIZE as usize) {
                let block_i = block + i as u64;

                let buffer_i = i * BLOCK_SIZE as usize;
                let buffer_j = cmp::min(buffer_i + BLOCK_SIZE as usize, buffer.len());
                let buffer_slice = &buffer[buffer_i..buffer_j];

                let mut cache_buf = [0; BLOCK_SIZE as usize];
                read += copy_memory(buffer_slice, &mut cache_buf);
                self.insert(block_i, cache_buf);
            }
        }

        Ok(read)
    }

    unsafe fn write_at(&mut self, block: u64, buffer: &[u8]) -> Result<usize> {
        //TODO: Write only blocks that have changed
        // println!("Cache write at {}", block);

        self.inner.write_at(block, buffer)?;

        let mut written = 0;
        for i in 0..buffer.len().div_ceil(BLOCK_SIZE as usize) {
            let block_i = block + i as u64;

            let buffer_i = i * BLOCK_SIZE as usize;
            let buffer_j = cmp::min(buffer_i + BLOCK_SIZE as usize, buffer.len());
            let buffer_slice = &buffer[buffer_i..buffer_j];

            let mut cache_buf = [0; BLOCK_SIZE as usize];
            written += copy_memory(buffer_slice, &mut cache_buf);
            self.insert(block_i, cache_buf);
        }

        Ok(written)
    }

    fn size(&mut self) -> Result<u64> {
        self.inner.size()
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::disk::DiskMemory;
    use crate::BLOCK_SIZE;

    #[test]
    fn cache_read_through_from_inner() {
        let inner = DiskMemory::new(BLOCK_SIZE * 4);
        let mut cache = DiskCache::new(inner);
        let block_data = [0xCDu8; BLOCK_SIZE as usize];
        unsafe {
            cache.write_at(2, &block_data).unwrap();
        }
        let mut read_buf = [0u8; BLOCK_SIZE as usize];
        unsafe {
            cache.read_at(2, &mut read_buf).unwrap();
        }
        assert_eq!(read_buf, block_data);
    }

    #[test]
    fn cache_hit_after_write() {
        let inner = DiskMemory::new(BLOCK_SIZE * 4);
        let mut cache = DiskCache::new(inner);
        let a = [1u8; BLOCK_SIZE as usize];
        let b = [2u8; BLOCK_SIZE as usize];
        unsafe {
            cache.write_at(0, &a).unwrap();
            cache.write_at(1, &b).unwrap();
        }
        let mut buf = [0u8; BLOCK_SIZE as usize];
        unsafe {
            cache.read_at(1, &mut buf).unwrap();
        }
        assert_eq!(buf, b);
    }
}
