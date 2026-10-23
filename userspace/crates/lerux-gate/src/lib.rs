//! First-party pointer, queue, and device operations.
//!
//! This file is the home of those `unsafe` tokens. A protection domain names a
//! linker symbol with [`region!`] or [`mmio!`] and then calls safe methods.
//! The macros expand to an `unsafe` block whose text lives here. A caller crate
//! that forbids `unsafe_code` still compiles.
//!
//! Field order of [`SerialQueue`] matches `serial_queue_t` in `lerux-sddf`.
//! The producer is the only writer of `tail`. The consumer is the only writer
//! of `head`.

#![cfg_attr(not(test), no_std)]

use core::{
    cell::UnsafeCell,
    ptr::{self, read_volatile, write_volatile},
    sync::atomic::{AtomicU32, AtomicUsize, Ordering},
};

const MAX_CLAIMS: usize = 64;

struct ClaimTable {
    n: usize,
    ranges: [(usize, usize); MAX_CLAIMS],
}

struct Claims(UnsafeCell<ClaimTable>);

// SAFETY: `claim` runs on one kernel thread, or under the host test lock.
unsafe impl Sync for Claims {}

static CLAIMS: Claims = Claims(UnsafeCell::new(ClaimTable {
    n: 0,
    ranges: [(0, 0); MAX_CLAIMS],
}));

static BORROWS: AtomicUsize = AtomicUsize::new(0);

struct BorrowGuard;

impl Drop for BorrowGuard {
    fn drop(&mut self) {
        BORROWS.fetch_sub(1, Ordering::Release);
    }
}

fn claim(start: usize, len: usize) {
    if len == 0 {
        return;
    }
    let end = start.saturating_add(len);
    // SAFETY: one kernel thread, or a host test, calls `claim`. The table is
    // not shared with another thread of execution.
    let table = unsafe { &mut *CLAIMS.0.get() };
    for i in 0..table.n {
        let (claimed_start, claimed_end) = table.ranges[i];
        if start < claimed_end && claimed_start < end {
            panic!("region claim overlaps");
        }
    }
    assert!(table.n < MAX_CLAIMS, "region claim table is full");
    table.ranges[table.n] = (start, end);
    table.n += 1;
}

/// A shared byte window. The pointer stays private.
pub struct Region {
    ptr: *mut u8,
    len: usize,
}

/// Names a Microkit memory region. The `unsafe` text is in this file.
#[macro_export]
macro_rules! region {
    ($name:ident, $len:expr) => {{
        unsafe extern "C" {
            static $name: [::core::cell::UnsafeCell<u8>; $len];
        }
        $crate::Region::from_ref(unsafe { &$name })
    }};
}

/// Memory-mapped registers. Register policy stays in the driver.
#[macro_export]
macro_rules! mmio {
    ($name:ident, $len:expr) => {{
        unsafe extern "C" {
            static $name: [::core::cell::UnsafeCell<u8>; $len];
        }
        $crate::Mmio::from_ref(unsafe { &$name })
    }};
}

impl Region {
    /// Claims `bytes`. A second claim of an overlapping range panics.
    pub fn from_ref(bytes: &'static [UnsafeCell<u8>]) -> Self {
        let len = bytes.len();
        let ptr = bytes.as_ptr().cast::<u8>().cast_mut();
        claim(ptr as usize, len);
        Self { ptr, len }
    }

    /// Claims an exclusive leaked buffer. Safe Rust cannot alias `bytes`.
    pub fn from_mut(bytes: &'static mut [u8]) -> Self {
        let len = bytes.len();
        let ptr = bytes.as_mut_ptr();
        claim(ptr as usize, len);
        Self { ptr, len }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn aligned_to(&self, align: usize) -> bool {
        (self.ptr as usize).is_multiple_of(align)
    }
}

/// Device registers. Every access is volatile and stays inside this window.
pub struct Mmio {
    ptr: *mut u8,
    len: usize,
}

impl Mmio {
    pub fn from_ref(bytes: &'static [UnsafeCell<u8>]) -> Self {
        let len = bytes.len();
        let ptr = bytes.as_ptr().cast::<u8>().cast_mut();
        claim(ptr as usize, len);
        Self { ptr, len }
    }

    pub fn from_mut(bytes: &'static mut [u8]) -> Self {
        let len = bytes.len();
        let ptr = bytes.as_mut_ptr();
        claim(ptr as usize, len);
        Self { ptr, len }
    }

    pub fn read_u8(&self, offset: usize) -> u8 {
        assert!(offset < self.len, "mmio offset fits");
        // SAFETY: `offset` is inside the claimed window. The load is volatile
        // because the window may be a device register.
        unsafe { read_volatile(self.ptr.add(offset)) }
    }

    pub fn write_u8(&self, offset: usize, value: u8) {
        assert!(offset < self.len, "mmio offset fits");
        // SAFETY: `offset` is inside the claimed window.
        unsafe { write_volatile(self.ptr.add(offset), value) }
    }
}

/// The only interior-mutability cell a protection domain uses.
///
/// `with` panics if it is already on the stack. The context switch will use
/// that same count so a borrow cannot cross a switch.
pub struct Shared<T> {
    value: UnsafeCell<T>,
}

// SAFETY: `T: Send` and this protection domain has one kernel thread. Host
// tests take their process lock before they touch the runtime.
unsafe impl<T: Send> Sync for Shared<T> {}

impl<T> Shared<T> {
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let previous = BORROWS.fetch_add(1, Ordering::Acquire);
        assert!(previous == 0, "Shared::with re-entered");
        let _guard = BorrowGuard;
        // SAFETY: the borrow count is one, so this closure is the only reader.
        unsafe { f(&mut *self.value.get()) }
    }
}

/// Shared serial queue indexes. The producer owns `tail`. The consumer owns `head`.
#[repr(C)]
struct SerialQueue {
    tail: AtomicU32,
    head: AtomicU32,
    producer_signalled: AtomicU32,
}

/// Transmit side of one serial byte ring.
pub struct SerialProducer {
    queue: *mut SerialQueue,
    data: *mut u8,
    capacity: u32,
}

/// Receive side of one serial byte ring.
pub struct SerialConsumer {
    queue: *mut SerialQueue,
    data: *mut u8,
    capacity: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PairError {
    EmptyCapacity,
    ShortData,
    ShortQueue,
    UnalignedQueue,
}

#[derive(Debug, PartialEq, Eq)]
pub enum QueueError {
    Full,
    Empty,
}

/// Builds both ends of one ring. `queue` must be zeroed and large enough for
/// [`SerialQueue`]. `data.len()` must be at least `capacity`.
pub fn serial_pair(
    queue: Region,
    data: Region,
    capacity: u32,
) -> Result<(SerialProducer, SerialConsumer), PairError> {
    if capacity == 0 {
        return Err(PairError::EmptyCapacity);
    }
    let Ok(data_len) = u32::try_from(data.len()) else {
        return Err(PairError::ShortData);
    };
    if data_len < capacity {
        return Err(PairError::ShortData);
    }
    if queue.len() < size_of::<SerialQueue>() {
        return Err(PairError::ShortQueue);
    }
    if !queue.aligned_to(align_of::<SerialQueue>()) {
        return Err(PairError::UnalignedQueue);
    }
    let producer = SerialProducer {
        queue: queue.ptr.cast(),
        data: data.ptr,
        capacity,
    };
    let consumer = SerialConsumer {
        queue: queue.ptr.cast(),
        data: data.ptr,
        capacity,
    };
    Ok((producer, consumer))
}

impl SerialProducer {
    pub fn enqueue(&self, byte: u8) -> Result<(), QueueError> {
        // SAFETY: `serial_pair` checked the queue alignment and the data length.
        // This side is the only writer of `tail`.
        unsafe {
            let tail = (*self.queue).tail.load(Ordering::Relaxed);
            let head = (*self.queue).head.load(Ordering::Acquire);
            if tail.wrapping_sub(head) == self.capacity {
                return Err(QueueError::Full);
            }
            let index = (tail % self.capacity) as usize;
            ptr::write(self.data.add(index), byte);
            (*self.queue)
                .tail
                .store(tail.wrapping_add(1), Ordering::Release);
        }
        Ok(())
    }
}

impl SerialConsumer {
    pub fn dequeue(&self) -> Result<u8, QueueError> {
        // SAFETY: `serial_pair` checked the queue alignment and the data length.
        // This side is the only writer of `head`.
        unsafe {
            let head = (*self.queue).head.load(Ordering::Relaxed);
            let tail = (*self.queue).tail.load(Ordering::Acquire);
            if head == tail {
                return Err(QueueError::Empty);
            }
            let index = (head % self.capacity) as usize;
            let byte = ptr::read(self.data.add(index));
            (*self.queue)
                .head
                .store(head.wrapping_add(1), Ordering::Release);
            Ok(byte)
        }
    }
}
