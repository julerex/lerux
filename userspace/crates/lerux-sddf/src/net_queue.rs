//! Network queue operations from `include/sddf/network/queue.h`.
//!
//! The producer is the only writer of `tail`. The consumer is the only writer of `head`.
//! Loads of the other side's index use acquire. Stores of this side's index use release.
//! That is the symmetric-multiprocessing pairing, including on one processor.
//! The header indexes with `tail % capacity`, so capacity need not be a power of two.
//! It must fit in `u16`: the full check compares a wrapping 16-bit difference with it.

use core::sync::atomic::{fence, Ordering};

use crate::net::{net_buff_desc_t, net_queue_handle_t, net_queue_t, NET_BUFFER_SIZE};

fn load_acquire(word: &core::sync::atomic::AtomicU16) -> u16 {
    word.load(Ordering::Acquire)
}

fn store_release(word: &core::sync::atomic::AtomicU16, value: u16) {
    word.store(value, Ordering::Release);
}

fn descriptors(queue: *mut net_queue_t) -> *mut net_buff_desc_t {
    queue
        .cast::<u8>()
        .wrapping_add(core::mem::size_of::<net_queue_t>())
        .cast()
}

fn length(tail: u16, head: u16) -> u16 {
    tail.wrapping_sub(head)
}

/// # Safety
///
/// `free` and `active` must point at shared queues whose descriptor arrays hold
/// `capacity` entries. `capacity` must be non-zero and fit in `u16`.
pub unsafe fn net_queue_init(
    queue: &mut net_queue_handle_t,
    free: *mut net_queue_t,
    active: *mut net_queue_t,
    capacity: u32,
) {
    debug_assert_ne!(capacity, 0);
    debug_assert!(capacity <= u32::from(u16::MAX));
    queue.free = free;
    queue.active = active;
    queue.capacity = capacity;
}

/// Number of buffers between `head` and `tail`.
///
/// Both indexes are loaded relaxed, matching `net_queue_length` in the header.
/// [`net_queue_empty_free`] and [`net_queue_full_free`] are the ordered checks.
///
/// # Safety
///
/// `queue` must point at a shared queue prefix.
pub unsafe fn net_queue_length(queue: *const net_queue_t) -> u16 {
    unsafe {
        let tail = (*queue).tail.load(Ordering::Relaxed);
        let head = (*queue).head.load(Ordering::Relaxed);
        length(tail, head)
    }
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller is the consumer of the free queue.
pub unsafe fn net_queue_empty_free(queue: &net_queue_handle_t) -> bool {
    unsafe {
        let tail = load_acquire(&(*queue.free).tail);
        let head = (*queue.free).head.load(Ordering::Relaxed);
        length(tail, head) == 0
    }
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller is the consumer of the active queue.
pub unsafe fn net_queue_empty_active(queue: &net_queue_handle_t) -> bool {
    unsafe {
        let tail = load_acquire(&(*queue.active).tail);
        let head = (*queue.active).head.load(Ordering::Relaxed);
        length(tail, head) == 0
    }
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller is the producer of the free queue.
pub unsafe fn net_queue_full_free(queue: &net_queue_handle_t) -> bool {
    unsafe {
        let tail = (*queue.free).tail.load(Ordering::Relaxed);
        let head = load_acquire(&(*queue.free).head);
        u32::from(length(tail, head)) == queue.capacity
    }
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller is the producer of the active queue.
pub unsafe fn net_queue_full_active(queue: &net_queue_handle_t) -> bool {
    unsafe {
        let tail = (*queue.active).tail.load(Ordering::Relaxed);
        let head = load_acquire(&(*queue.active).head);
        u32::from(length(tail, head)) == queue.capacity
    }
}

unsafe fn enqueue(
    queue: *mut net_queue_t,
    capacity: u32,
    buffer: net_buff_desc_t,
    full: bool,
) -> i32 {
    if full {
        return -1;
    }
    unsafe {
        let tail = (*queue).tail.load(Ordering::Relaxed);
        let index = u32::from(tail) % capacity;
        *descriptors(queue).wrapping_add(index as usize) = buffer;
        store_release(&(*queue).tail, tail.wrapping_add(1));
    }
    0
}

unsafe fn dequeue(
    queue: *mut net_queue_t,
    capacity: u32,
    buffer: &mut net_buff_desc_t,
    empty: bool,
) -> i32 {
    if empty {
        return -1;
    }
    unsafe {
        let head = (*queue).head.load(Ordering::Relaxed);
        let index = u32::from(head) % capacity;
        *buffer = *descriptors(queue).wrapping_add(index as usize);
        store_release(&(*queue).head, head.wrapping_add(1));
    }
    0
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller is the producer of the free queue.
pub unsafe fn net_enqueue_free(queue: &net_queue_handle_t, buffer: net_buff_desc_t) -> i32 {
    unsafe {
        enqueue(
            queue.free,
            queue.capacity,
            buffer,
            net_queue_full_free(queue),
        )
    }
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller is the producer of the active queue.
pub unsafe fn net_enqueue_active(queue: &net_queue_handle_t, buffer: net_buff_desc_t) -> i32 {
    unsafe {
        enqueue(
            queue.active,
            queue.capacity,
            buffer,
            net_queue_full_active(queue),
        )
    }
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller is the consumer of the free queue.
pub unsafe fn net_dequeue_free(queue: &net_queue_handle_t, buffer: &mut net_buff_desc_t) -> i32 {
    unsafe {
        dequeue(
            queue.free,
            queue.capacity,
            buffer,
            net_queue_empty_free(queue),
        )
    }
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller is the consumer of the active queue.
pub unsafe fn net_dequeue_active(queue: &net_queue_handle_t, buffer: &mut net_buff_desc_t) -> i32 {
    unsafe {
        dequeue(
            queue.active,
            queue.capacity,
            buffer,
            net_queue_empty_active(queue),
        )
    }
}

/// Fill a free queue with one descriptor per buffer.
///
/// `base_addr` is added to each offset. The receive virtualiser passes the
/// region input/output address. A client passes 0 and stores offsets.
///
/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller is the producer of the free queue.
/// The free queue holds no descriptors yet.
pub unsafe fn net_buffers_init(queue: &net_queue_handle_t, base_addr: u64) {
    for index in 0..queue.capacity {
        let buffer = net_buff_desc_t::new(
            u64::from(NET_BUFFER_SIZE) * u64::from(index) + base_addr,
            0,
            0,
        );
        let status = unsafe { net_enqueue_free(queue, buffer) };
        debug_assert_eq!(status, 0);
    }
}

unsafe fn request_signal(queue: *mut net_queue_t) {
    unsafe {
        // The network header uses a release fence. The serial header's
        // sequentially consistent pair is what keeps the producer re-check and
        // the consumer flag load from both missing the update.
        (*queue).consumer_signalled.store(0, Ordering::Relaxed);
        fence(Ordering::SeqCst);
    }
}

unsafe fn cancel_signal(queue: *mut net_queue_t) {
    unsafe {
        // A cancellation is followed by a request, so this store has no fence.
        (*queue).consumer_signalled.store(1, Ordering::Relaxed);
    }
}

unsafe fn require_signal(queue: *mut net_queue_t) -> bool {
    unsafe {
        fence(Ordering::SeqCst);
        (*queue).consumer_signalled.load(Ordering::Relaxed) == 0
    }
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller consumes the free queue.
pub unsafe fn net_request_signal_free(queue: &net_queue_handle_t) {
    unsafe { request_signal(queue.free) }
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller consumes the active queue.
pub unsafe fn net_request_signal_active(queue: &net_queue_handle_t) {
    unsafe { request_signal(queue.active) }
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller produces the free queue.
pub unsafe fn net_cancel_signal_free(queue: &net_queue_handle_t) {
    unsafe { cancel_signal(queue.free) }
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller produces the active queue.
pub unsafe fn net_cancel_signal_active(queue: &net_queue_handle_t) {
    unsafe { cancel_signal(queue.active) }
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller produces the free queue.
pub unsafe fn net_require_signal_free(queue: &net_queue_handle_t) -> bool {
    unsafe { require_signal(queue.free) }
}

/// # Safety
///
/// Same pointer rules as [`net_queue_init`]. The caller produces the active queue.
pub unsafe fn net_require_signal_active(queue: &net_queue_handle_t) -> bool {
    unsafe { require_signal(queue.active) }
}
