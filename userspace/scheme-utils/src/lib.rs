#![feature(never_type)]

//! Utilities for scheme (device/driver) daemons.
//!
//! Provides common building blocks used by early userspace daemons
//! (logd, ramfs, zerod, etc.):
//!
//! * [`HandleMap`] — simple ID allocator + map for open file handles / resources.
//! * [`Blocking`] — readiness-based blocking read/write helper.
//! * [`ReadinessBased`] — event readiness tracking for schemes.
//!
//! These reduce boilerplate for implementing `redox_scheme::Scheme` (or `SchemeSync`).

use std::collections::{BTreeMap, btree_map};
use std::fmt;
use std::num::Wrapping;

use syscall::{EBADF, Error, Result};

mod blocking;
mod readiness_based;

pub use blocking::Blocking;
pub use readiness_based::ReadinessBased;

/// Simple growing ID allocator + BTreeMap for scheme handles.
///
/// IDs start at 1 and wrap; `insert` finds the next free slot (skipping in-use IDs).
/// Used by daemons to assign unique handles returned to userspace on open.
pub struct HandleMap<T> {
    handles: BTreeMap<usize, T>,
    next_id: Wrapping<usize>,
}

impl<T> HandleMap<T> {
    /// Create an empty handle map (next ID starts at 1).
    pub const fn new() -> Self {
        HandleMap {
            handles: BTreeMap::new(),
            next_id: Wrapping(1),
        }
    }

    /// Insert a value and return a fresh ID for it.
    ///
    /// Loops to find an unused ID (handles wrap-around collisions).
    pub fn insert(&mut self, handle: T) -> usize {
        let id = self.next_id;

        // If we've looped round there's a small chance that the file descriptor still exists, so loop till we get one that doesn't
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

    /// Remove and return the value for an ID, if present.
    pub fn remove(&mut self, id: usize) -> Option<T> {
        self.handles.remove(&id)
    }

    /// Get a reference to the value for an ID.
    ///
    /// Returns `EBADF` error if not present (standard for bad fd/handle).
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

pub struct FpathWriter<'a> {
    buf: &'a mut [u8],
    written: usize,
}

impl<'a> FpathWriter<'a> {
    pub fn with(
        buf: &'a mut [u8],
        scheme_name: &str,
        f: impl FnOnce(&mut Self) -> Result<()>,
    ) -> Result<usize> {
        let mut w = FpathWriter { buf, written: 0 };
        write!(w, "/scheme/{scheme_name}/").unwrap();
        f(&mut w)?;
        Ok(w.written)
    }

    pub fn with_legacy(
        buf: &'a mut [u8],
        scheme_name: &str,
        f: impl FnOnce(&mut Self) -> Result<()>,
    ) -> Result<usize> {
        let mut w = FpathWriter { buf, written: 0 };
        write!(w, "{scheme_name}:").unwrap();
        f(&mut w)?;
        Ok(w.written)
    }

    pub fn push_str(&mut self, s: &str) {
        let count = core::cmp::min(s.len(), self.buf.len() - self.written);
        self.buf[self.written..self.written + count].copy_from_slice(&s.as_bytes()[..count]);
        self.written += count;
    }

    pub fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        std::fmt::write(self, args)
    }
}

impl fmt::Write for FpathWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}
