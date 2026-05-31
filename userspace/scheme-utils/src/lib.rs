#![no_std]
#![feature(never_type)]

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use core::fmt;
use core::num::Wrapping;

use syscall::{EBADF, Error, Result};

mod blocking;
mod readiness_based;

pub use blocking::Blocking;
pub use readiness_based::ReadinessBased;

pub struct HandleMap<T> {
    handles: BTreeMap<usize, T>,
    next_id: Wrapping<usize>,
}

impl<T> HandleMap<T> {
    pub const fn new() -> Self {
        HandleMap {
            handles: BTreeMap::new(),
            next_id: Wrapping(1),
        }
    }

    pub fn insert(&mut self, handle: T) -> usize {
        let id = self.next_id;

        self.next_id += Wrapping(1);
        loop {
            if !self.handles.contains_key(&self.next_id.0) {
                break;
            } else {
                self.next_id += Wrapping(1);
            }
        }

        self.handles.insert(id.0, handle);
        id.0
    }

    pub fn remove(&mut self, id: usize) -> Option<T> {
        self.handles.remove(&id)
    }

    pub fn get(&self, id: usize) -> Result<&T> {
        self.handles.get(&id).ok_or(Error::new(EBADF))
    }

    pub fn get_mut(&mut self, id: usize) -> Result<&mut T> {
        self.handles.get_mut(&id).ok_or(Error::new(EBADF))
    }

    pub fn iter(&self) -> btree_map::Iter<'_, usize, T> {
        self.handles.iter()
    }

    pub fn iter_mut(&mut self) -> btree_map::IterMut<'_, usize, T> {
        self.handles.iter_mut()
    }

    pub fn keys(&self) -> btree_map::Keys<'_, usize, T> {
        self.handles.keys()
    }

    pub fn values(&self) -> btree_map::Values<'_, usize, T> {
        self.handles.values()
    }

    pub fn values_mut(&mut self) -> btree_map::ValuesMut<'_, usize, T> {
        self.handles.values_mut()
    }
}

use alloc::collections::btree_map;

pub struct FpathWriter<'a> {
    buf: &'a mut [u8],
    written: usize,
}

impl<'a> FpathWriter<'a> {
    pub fn with(buf: &'a mut [u8], scheme: &str, f: impl FnOnce(&mut Self) -> Result<()>) -> Result<usize> {
        let mut w = FpathWriter { buf, written: 0 };
        w.push_str(scheme)?;
        w.push_str(":")?;
        f(&mut w)?;
        Ok(w.written)
    }

    pub fn push_str(&mut self, s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        if self.written + bytes.len() > self.buf.len() {
            return Err(Error::new(syscall::ENAMETOOLONG));
        }
        self.buf[self.written..self.written + bytes.len()].copy_from_slice(bytes);
        self.written += bytes.len();
        Ok(())
    }
}
