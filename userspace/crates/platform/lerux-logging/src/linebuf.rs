//! Coalesce sel4-logging write chunks into complete lines.
//!
//! The upstream logger may invoke the write callback with short fragments of
//! one logical line. Concurrent PDs that print those fragments immediately
//! interleave on the UART, so smoke substring checks miss tokens such as
//! `virtio-net: MAC`.

use core::cell::UnsafeCell;

/// Byte capacity of one coalesced line (truncated by flushing when full).
pub const DEFAULT_CAP: usize = 256;

/// Accumulates bytes until a newline (or [`CAP`]) and then emits one line.
pub struct LineCoalescer<const CAP: usize = DEFAULT_CAP> {
    buf: [u8; CAP],
    len: usize,
}

impl<const CAP: usize> LineCoalescer<CAP> {
    pub const fn new() -> Self {
        Self {
            buf: [0; CAP],
            len: 0,
        }
    }

    /// Feed a logger chunk. `emit` receives each complete line without `\n`.
    pub fn push(&mut self, s: &str, mut emit: impl FnMut(&str)) {
        for &b in s.as_bytes() {
            if b == b'\n' || b == b'\r' {
                self.flush(&mut emit);
                continue;
            }
            if self.len == CAP {
                self.flush(&mut emit);
            }
            self.buf[self.len] = b;
            self.len += 1;
        }
    }

    fn flush(&mut self, emit: &mut impl FnMut(&str)) {
        if self.len == 0 {
            return;
        }
        if let Ok(line) = core::str::from_utf8(&self.buf[..self.len]) {
            emit(line);
        }
        self.len = 0;
    }
}

/// Process-wide line buffer for a single-threaded Microkit PD.
pub struct StaticLineBuf<const CAP: usize = DEFAULT_CAP>(UnsafeCell<LineCoalescer<CAP>>);

// SAFETY: Microkit PDs are single-threaded; the buffer is installed once in
// `init` and only touched from that PD's log write callback.
unsafe impl<const CAP: usize> Sync for StaticLineBuf<CAP> {}

impl<const CAP: usize> StaticLineBuf<CAP> {
    pub const fn new() -> Self {
        Self(UnsafeCell::new(LineCoalescer::new()))
    }

    /// # Safety
    /// Caller must be the only thread that touches this buffer (PD init / log
    /// callbacks on a single-threaded Microkit PD).
    pub unsafe fn push(&self, s: &str, emit: impl FnMut(&str)) {
        // SAFETY: caller guarantees exclusive access; see function contract.
        unsafe {
            (*self.0.get()).push(s, emit);
        }
    }
}
