//! sDDF-shaped single-producer single-consumer serial byte queues.
//!
//! Layout (shared between driver and virtualiser):
//! - **Queue region**: [`SerialQueue`] (head / tail / `producer_signalled`)
//! - **Data region**: byte ring of power-of-two capacity
//!
//! Local [`SerialQueueHandle`] holds capacity and pointers; capacity is never
//! trusted from shared memory (untrusted peer cannot enlarge the ring).
//!
//! # Safety
//!
//! Cross-PD use requires Microkit maps of the same physical pages into both
//! address spaces and correct producer/consumer roles on each queue.

#![cfg_attr(not(test), no_std)]

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Shared queue metadata (one page is enough).
///
/// Head is next **read** index (consumer-owned). Tail is next **write** index
/// (producer-owned). Indices are free-running and masked with `capacity - 1`.
#[repr(C, align(64))]
pub struct SerialQueue {
    pub head: AtomicUsize,
    pub tail: AtomicUsize,
    /// Producer sets false when waiting for free space; consumer notifies if false.
    pub producer_signalled: AtomicBool,
}

impl SerialQueue {
    pub const fn new() -> Self {
        Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            producer_signalled: AtomicBool::new(true),
        }
    }
}

impl Default for SerialQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Local handle: queue + data pointers and power-of-two capacity.
#[derive(Clone, Copy)]
pub struct SerialQueueHandle {
    queue: *mut SerialQueue,
    data: *mut u8,
    capacity: usize,
}

// Handles are moved across single-threaded PD init; shared memory is Sync by map.
unsafe impl Send for SerialQueueHandle {}
unsafe impl Sync for SerialQueueHandle {}

impl SerialQueueHandle {
    /// # Safety
    /// `queue` and `data` must be valid for the lifetime of the handle, shared
    /// with exactly one peer PD, and `capacity` must be a power of two ≥ 2.
    pub const unsafe fn new(queue: *mut SerialQueue, data: *mut u8, capacity: usize) -> Self {
        Self {
            queue,
            data,
            capacity,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    fn mask(&self) -> usize {
        self.capacity - 1
    }

    fn q(&self) -> &SerialQueue {
        // SAFETY: constructed with a valid shared pointer.
        unsafe { &*self.queue }
    }

    pub fn len(&self) -> usize {
        let head = self.q().head.load(Ordering::Acquire);
        let tail = self.q().tail.load(Ordering::Acquire);
        tail.wrapping_sub(head)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }

    pub fn free_space(&self) -> usize {
        self.capacity.saturating_sub(self.len())
    }

    /// Enqueue one byte (producer). Returns `false` if full.
    pub fn enqueue(&self, byte: u8) -> bool {
        let q = self.q();
        let head = q.head.load(Ordering::Acquire);
        let tail = q.tail.load(Ordering::Relaxed);
        if tail.wrapping_sub(head) >= self.capacity {
            return false;
        }
        let idx = tail & self.mask();
        // SAFETY: idx in [0, capacity); data region sized to capacity.
        unsafe {
            self.data.add(idx).write(byte);
        }
        q.tail.store(tail.wrapping_add(1), Ordering::Release);
        true
    }

    /// Dequeue one byte (consumer). Returns `None` if empty.
    pub fn dequeue(&self) -> Option<u8> {
        let q = self.q();
        let head = q.head.load(Ordering::Relaxed);
        let tail = q.tail.load(Ordering::Acquire);
        if head == tail {
            return None;
        }
        let idx = head & self.mask();
        // SAFETY: idx in [0, capacity).
        let byte = unsafe { self.data.add(idx).read() };
        q.head.store(head.wrapping_add(1), Ordering::Release);
        Some(byte)
    }

    /// Request free-space notification (producer side of sDDF protocol).
    pub fn request_signal(&self) {
        self.q().producer_signalled.store(false, Ordering::Release);
    }

    /// After consumer progress: return true if producer should be notified.
    pub fn consumer_should_signal_producer(&self) -> bool {
        // Swap to true; if it was false, producer is waiting.
        !self.q().producer_signalled.swap(true, Ordering::AcqRel)
    }

    /// Initialize shared queue header (call once from one side before traffic).
    pub fn init_shared(&self) {
        let q = self.q();
        q.head.store(0, Ordering::Relaxed);
        q.tail.store(0, Ordering::Relaxed);
        q.producer_signalled.store(true, Ordering::Relaxed);
    }
}

/// Default data-region capacity used by workstation serial virt (power of two).
pub const DEFAULT_CAPACITY: usize = 2048;

/// Size of queue metadata region (one page).
pub const QUEUE_META_SIZE: usize = 0x1000;

/// Size of data region for [`DEFAULT_CAPACITY`].
pub const DEFAULT_DATA_SIZE: usize = 0x1000;

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::MaybeUninit;

    fn pair(cap: usize) -> (SerialQueueHandle, Box<SerialQueue>, Vec<u8>) {
        assert!(cap.is_power_of_two());
        let mut q = Box::new(SerialQueue::new());
        let mut data = vec![0u8; cap];
        let h = unsafe { SerialQueueHandle::new(&mut *q as *mut _, data.as_mut_ptr(), cap) };
        h.init_shared();
        (h, q, data)
    }

    #[test]
    fn enqueue_dequeue_roundtrip() {
        let (h, _q, _d) = pair(8);
        assert!(h.enqueue(b'a'));
        assert!(h.enqueue(b'b'));
        assert_eq!(h.dequeue(), Some(b'a'));
        assert_eq!(h.dequeue(), Some(b'b'));
        assert_eq!(h.dequeue(), None);
    }

    #[test]
    fn full_queue_rejects() {
        let (h, _q, _d) = pair(4);
        assert!(h.enqueue(1));
        assert!(h.enqueue(2));
        assert!(h.enqueue(3));
        assert!(h.enqueue(4));
        assert!(!h.enqueue(5));
        assert_eq!(h.dequeue(), Some(1));
        assert!(h.enqueue(5));
    }

    #[test]
    fn wrap_around() {
        let (h, _q, _d) = pair(4);
        for i in 0..10u8 {
            assert!(h.enqueue(i), "enqueue {i}");
            assert_eq!(h.dequeue(), Some(i));
        }
    }

    #[test]
    fn producer_signalled_protocol() {
        let (h, _q, _d) = pair(4);
        h.request_signal();
        assert!(h.consumer_should_signal_producer());
        assert!(!h.consumer_should_signal_producer());
    }

    #[test]
    fn queue_layout_fits_page() {
        assert!(core::mem::size_of::<SerialQueue>() <= QUEUE_META_SIZE);
        let _ = MaybeUninit::<SerialQueue>::uninit();
    }
}
