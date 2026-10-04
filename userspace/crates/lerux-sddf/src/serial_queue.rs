//! Serial queue operations from `include/sddf/serial/queue.h`.
//!
//! The producer is the only writer of `tail`. The consumer is the only writer of `head`.
//! Loads of the other side's index use acquire. Stores of this side's index use release.
//! That is the `CONFIG_ENABLE_SMP_SUPPORT` pairing in the header.

use core::sync::atomic::{fence, AtomicU32, Ordering};

use crate::serial::{serial_connection_resource_t, serial_queue_handle_t, serial_queue_t};

fn load_acquire(word: &AtomicU32) -> u32 {
    word.load(Ordering::Acquire)
}

fn store_release(word: &AtomicU32, value: u32) {
    word.store(value, Ordering::Release);
}

/// # Safety
///
/// `queue_handle.queue` and `queue_handle.data_region` must point at shared
/// memory valid for `capacity` bytes. `capacity` must be non-zero. The caller
/// is the producer.
pub unsafe fn serial_queue_init(
    queue_handle: &mut serial_queue_handle_t,
    queue: *mut serial_queue_t,
    capacity: u32,
    data_region: *mut u8,
) {
    queue_handle.queue = queue;
    queue_handle.capacity = capacity;
    queue_handle.data_region = data_region;
}

/// # Safety
///
/// Same pointer rules as [`serial_queue_init`]. The caller is the producer.
pub unsafe fn serial_queue_length_producer(queue_handle: &serial_queue_handle_t) -> u32 {
    unsafe {
        let tail = (*queue_handle.queue).tail.load(Ordering::Relaxed);
        let head = load_acquire(&(*queue_handle.queue).head);
        tail.wrapping_sub(head)
    }
}

/// # Safety
///
/// Same pointer rules as [`serial_queue_init`]. The caller is the producer.
pub unsafe fn serial_queue_free(queue_handle: &serial_queue_handle_t) -> u32 {
    unsafe { queue_handle.capacity - serial_queue_length_producer(queue_handle) }
}

/// # Safety
///
/// Same pointer rules as [`serial_queue_init`]. The caller is the consumer.
pub unsafe fn serial_queue_length_consumer(queue_handle: &serial_queue_handle_t) -> u32 {
    unsafe {
        let tail = load_acquire(&(*queue_handle.queue).tail);
        let head = (*queue_handle.queue).head.load(Ordering::Relaxed);
        tail.wrapping_sub(head)
    }
}

/// # Safety
///
/// Same pointer rules as [`serial_queue_init`]. The caller is the producer.
pub unsafe fn serial_queue_empty(queue_handle: &serial_queue_handle_t, local_head: u32) -> bool {
    unsafe {
        let tail = load_acquire(&(*queue_handle.queue).tail);
        local_head == tail
    }
}

/// # Safety
///
/// Same pointer rules as [`serial_queue_init`]. The caller is the producer.
pub unsafe fn serial_queue_full(queue_handle: &serial_queue_handle_t, local_tail: u32) -> bool {
    unsafe {
        let head = load_acquire(&(*queue_handle.queue).head);
        local_tail.wrapping_sub(head) == queue_handle.capacity
    }
}

/// # Safety
///
/// Same pointer rules as [`serial_queue_init`]. The caller is the producer.
pub unsafe fn serial_enqueue(queue_handle: &serial_queue_handle_t, character: u8) -> i32 {
    unsafe {
        let tail = (*queue_handle.queue).tail.load(Ordering::Relaxed);
        if serial_queue_full(queue_handle, tail) {
            return -1;
        }
        let index = (tail % queue_handle.capacity) as usize;
        *queue_handle.data_region.add(index) = character;
        store_release(&(*queue_handle.queue).tail, tail.wrapping_add(1));
        0
    }
}

/// # Safety
///
/// Same pointer rules as [`serial_queue_init`]. The caller is the producer.
/// `local_tail` is the caller's copy and is not the shared tail.
pub unsafe fn serial_enqueue_local(
    queue_handle: &serial_queue_handle_t,
    local_tail: &mut u32,
    character: u8,
) -> i32 {
    unsafe {
        if serial_queue_full(queue_handle, *local_tail) {
            return -1;
        }
        let index = (*local_tail % queue_handle.capacity) as usize;
        *queue_handle.data_region.add(index) = character;
        *local_tail = local_tail.wrapping_add(1);
        0
    }
}

/// # Safety
///
/// Same pointer rules as [`serial_queue_init`]. The caller is the consumer.
pub unsafe fn serial_dequeue(queue_handle: &serial_queue_handle_t, character: &mut u8) -> i32 {
    unsafe {
        let head = (*queue_handle.queue).head.load(Ordering::Relaxed);
        if serial_queue_empty(queue_handle, head) {
            return -1;
        }
        let index = (head % queue_handle.capacity) as usize;
        *character = *queue_handle.data_region.add(index);
        store_release(&(*queue_handle.queue).head, head.wrapping_add(1));
        0
    }
}

/// # Safety
///
/// Same pointer rules as [`serial_queue_init`]. The caller is the producer.
pub unsafe fn serial_update_shared_tail(queue_handle: &serial_queue_handle_t, local_tail: u32) {
    unsafe { store_release(&(*queue_handle.queue).tail, local_tail) }
}

/// # Safety
///
/// Same pointer rules as [`serial_queue_init`]. The caller is the consumer.
pub unsafe fn serial_update_shared_head(queue_handle: &serial_queue_handle_t, local_head: u32) {
    unsafe { store_release(&(*queue_handle.queue).head, local_head) }
}

/// # Safety
///
/// Same pointer rules as [`serial_queue_init`]. The caller is the producer.
pub unsafe fn serial_request_consumer_signal(queue_handle: &serial_queue_handle_t) {
    unsafe {
        (*queue_handle.queue)
            .producer_signalled
            .store(0, Ordering::Relaxed);
        fence(Ordering::SeqCst);
    }
}

/// # Safety
///
/// Same pointer rules as [`serial_queue_init`]. The caller is the producer.
pub unsafe fn serial_cancel_consumer_signal(queue_handle: &serial_queue_handle_t) {
    unsafe {
        (*queue_handle.queue)
            .producer_signalled
            .store(1, Ordering::Relaxed);
    }
}

/// Build a handle for one connection in a serial configuration page.
///
/// `connection.data.size` is the queue capacity, matching `serial_queue_init`'s
/// capacity argument in the transmit and receive virtualisers.
///
/// # Safety
///
/// `connection.queue.vaddr` and `connection.data.vaddr` must point at shared
/// memory mapped in this protection domain. `connection.data.size` must be
/// non-zero and fit in `u32`.
pub unsafe fn serial_handle_from_connection(
    connection: &serial_connection_resource_t,
) -> serial_queue_handle_t {
    let capacity = connection.data.size as u32;
    debug_assert_eq!(u64::from(capacity), connection.data.size);
    debug_assert_ne!(capacity, 0);
    let mut handle = serial_queue_handle_t {
        queue: core::ptr::null_mut(),
        capacity: 0,
        data_region: core::ptr::null_mut(),
    };
    unsafe {
        serial_queue_init(
            &mut handle,
            connection.queue.vaddr.cast(),
            capacity,
            connection.data.vaddr,
        );
    }
    handle
}

/// # Safety
///
/// Same pointer rules as [`serial_queue_init`]. The caller is the consumer.
pub unsafe fn serial_require_consumer_signal(queue_handle: &serial_queue_handle_t) -> bool {
    unsafe {
        fence(Ordering::SeqCst);
        (*queue_handle.queue)
            .producer_signalled
            .load(Ordering::Relaxed)
            == 0
    }
}
