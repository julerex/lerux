//! Host checks for the network single-producer single-consumer rings.
//!
//! Capacity 3 is intentional. An index mask that assumes a power of two would
//! pass a capacity of 4 and still be the wrong rule.

use core::sync::atomic::Ordering;

use lerux_sddf::{
    net_buff_desc_t, net_buffers_init, net_cancel_signal_free, net_dequeue_active,
    net_dequeue_free, net_enqueue_active, net_enqueue_free, net_queue_empty_active,
    net_queue_full_free, net_queue_handle_t, net_queue_init, net_queue_length, net_queue_t,
    net_request_signal_active, net_request_signal_free, net_require_signal_active,
    net_require_signal_free, NET_BUFFER_SIZE,
};

#[repr(C, align(8))]
struct Region([u8; 4096]);

struct Harness {
    free: Box<Region>,
    active: Box<Region>,
    handle: net_queue_handle_t,
}

fn harness(capacity: u32) -> Harness {
    let mut harness = Harness {
        free: Box::new(Region([0; 4096])),
        active: Box::new(Region([0; 4096])),
        handle: net_queue_handle_t {
            free: core::ptr::null_mut(),
            active: core::ptr::null_mut(),
            capacity: 0,
        },
    };
    let free = harness.free.0.as_mut_ptr().cast();
    let active = harness.active.0.as_mut_ptr().cast();
    // SAFETY: the boxed regions stay allocated for the life of this harness.
    // `capacity` is non-zero and fits in `u16`.
    unsafe { net_queue_init(&mut harness.handle, free, active, capacity) };
    // A zeroed flag means the consumer wants a signal. Start from "not requesting".
    unsafe {
        (*harness.handle.free)
            .consumer_signalled
            .store(1, Ordering::Relaxed);
        (*harness.handle.active)
            .consumer_signalled
            .store(1, Ordering::Relaxed);
    }
    harness
}

fn descriptor(offset: u64, len: u16, oid: u8) -> net_buff_desc_t {
    net_buff_desc_t::new(offset, len, oid)
}

fn enqueue_free(harness: &Harness, buffer: net_buff_desc_t) -> i32 {
    // SAFETY: the harness owns both queues.
    unsafe { net_enqueue_free(&harness.handle, buffer) }
}

fn dequeue_free(harness: &Harness) -> Result<net_buff_desc_t, i32> {
    let mut buffer = descriptor(0, 0, 0);
    // SAFETY: the harness owns both queues.
    let status = unsafe { net_dequeue_free(&harness.handle, &mut buffer) };
    if status == 0 {
        Ok(buffer)
    } else {
        Err(status)
    }
}

#[test]
fn capacity_three_fills_and_then_rejects() {
    let harness = harness(3);
    assert_eq!(enqueue_free(&harness, descriptor(1, 4, 0x7f)), 0);
    assert_eq!(enqueue_free(&harness, descriptor(2, 5, 1)), 0);
    assert_eq!(enqueue_free(&harness, descriptor(3, 6, 2)), 0);
    assert_eq!(enqueue_free(&harness, descriptor(4, 7, 3)), -1);
    // SAFETY: the harness owns both queues.
    assert!(unsafe { net_queue_full_free(&harness.handle) });

    let first = dequeue_free(&harness).unwrap();
    assert_eq!(first.io_or_offset, 1);
    assert_eq!(first.len, 4);
    assert_eq!(first.oid(), 0x3f);
    assert_eq!(dequeue_free(&harness).unwrap().io_or_offset, 2);
    assert_eq!(enqueue_free(&harness, descriptor(9, 8, 0)), 0);
    assert_eq!(dequeue_free(&harness).unwrap().io_or_offset, 3);
    assert_eq!(dequeue_free(&harness).unwrap().io_or_offset, 9);
    assert_eq!(dequeue_free(&harness).unwrap_err(), -1);
}

#[test]
fn indexes_wrap_through_u16_max() {
    let harness = harness(3);
    unsafe {
        (*harness.handle.free)
            .tail
            .store(u16::MAX, Ordering::Relaxed);
        (*harness.handle.free)
            .head
            .store(u16::MAX, Ordering::Relaxed);
    }
    assert_eq!(enqueue_free(&harness, descriptor(0x11, 1, 0)), 0);
    // SAFETY: the harness owns the free queue.
    assert_eq!(unsafe { net_queue_length(harness.handle.free) }, 1);
    assert_eq!(
        unsafe { (*harness.handle.free).tail.load(Ordering::Relaxed) },
        0
    );
    assert_eq!(dequeue_free(&harness).unwrap().io_or_offset, 0x11);
    assert_eq!(
        unsafe { (*harness.handle.free).head.load(Ordering::Relaxed) },
        0
    );
}

#[test]
fn free_and_active_rings_are_independent() {
    let harness = harness(2);
    assert_eq!(enqueue_free(&harness, descriptor(8, 2, 0)), 0);
    // SAFETY: the harness owns both queues.
    assert!(unsafe { net_queue_empty_active(&harness.handle) });
    let mut active = descriptor(0, 0, 0);
    assert_eq!(
        unsafe { net_dequeue_active(&harness.handle, &mut active) },
        -1
    );
    assert_eq!(
        unsafe { net_enqueue_active(&harness.handle, descriptor(9, 3, 4)) },
        0
    );
    assert_eq!(
        unsafe { net_dequeue_active(&harness.handle, &mut active) },
        0
    );
    assert_eq!(active.io_or_offset, 9);
    assert_eq!(active.oid(), 4);
    assert_eq!(dequeue_free(&harness).unwrap().io_or_offset, 8);
}

#[test]
fn buffers_init_enqueues_one_offset_per_slot() {
    let harness = harness(4);
    // SAFETY: the free queue is empty and the harness owns it.
    unsafe { net_buffers_init(&harness.handle, 0x20_0000) };
    for index in 0..4u64 {
        let buffer = dequeue_free(&harness).unwrap();
        assert_eq!(
            buffer.io_or_offset,
            0x20_0000 + index * u64::from(NET_BUFFER_SIZE)
        );
        assert_eq!(buffer.len, 0);
        assert_eq!(buffer.oid(), 0);
    }
    assert_eq!(dequeue_free(&harness).unwrap_err(), -1);
    // SAFETY: the harness owns the active queue.
    assert!(unsafe { net_queue_empty_active(&harness.handle) });
}

#[test]
fn zeroed_queue_already_requests_a_signal() {
    let mut region = Box::new(Region([0; 4096]));
    let queue = region.0.as_mut_ptr().cast::<net_queue_t>();
    let mut handle = net_queue_handle_t {
        free: core::ptr::null_mut(),
        active: core::ptr::null_mut(),
        capacity: 0,
    };
    // SAFETY: `region` stays allocated and the queue prefix is inside it.
    unsafe { net_queue_init(&mut handle, queue, queue, 2) };
    assert!(unsafe { net_require_signal_free(&handle) });
}

#[test]
fn signal_request_is_visible_on_each_ring() {
    let harness = harness(2);
    // SAFETY: the harness owns both queues.
    unsafe { net_request_signal_free(&harness.handle) };
    assert!(unsafe { net_require_signal_free(&harness.handle) });
    unsafe { net_cancel_signal_free(&harness.handle) };
    assert!(!unsafe { net_require_signal_free(&harness.handle) });

    unsafe { net_request_signal_active(&harness.handle) };
    assert!(unsafe { net_require_signal_active(&harness.handle) });
    assert!(!unsafe { net_require_signal_free(&harness.handle) });
}
