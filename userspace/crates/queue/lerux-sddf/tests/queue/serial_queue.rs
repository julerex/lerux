//! Host checks for the serial single-producer single-consumer ring.
//!
//! Capacity 3 is intentional. An index mask that assumes a power of two would
//! pass a capacity of 4 and still be the wrong rule.

use core::sync::atomic::{AtomicU32, Ordering};

use lerux_sddf::{
    serial_cancel_consumer_signal, serial_dequeue, serial_enqueue, serial_enqueue_local,
    serial_queue_free, serial_queue_handle_t, serial_queue_init, serial_queue_t,
    serial_request_consumer_signal, serial_require_consumer_signal, serial_update_shared_head,
    serial_update_shared_tail,
};

struct Harness {
    queue: Box<serial_queue_t>,
    data: Vec<u8>,
    handle: serial_queue_handle_t,
}

fn harness(capacity: u32) -> Harness {
    let mut harness = Harness {
        queue: Box::new(serial_queue_t {
            tail: AtomicU32::new(0),
            head: AtomicU32::new(0),
            producer_signalled: AtomicU32::new(1),
        }),
        data: vec![0; capacity as usize],
        handle: serial_queue_handle_t {
            queue: core::ptr::null_mut(),
            capacity: 0,
            data_region: core::ptr::null_mut(),
        },
    };
    let queue = &raw mut *harness.queue;
    let data_region = harness.data.as_mut_ptr();
    // SAFETY: `queue` and `data` stay allocated for the life of this harness.
    // `capacity` is non-zero.
    unsafe { serial_queue_init(&mut harness.handle, queue, capacity, data_region) };
    harness
}

fn enqueue(harness: &Harness, byte: u8) -> i32 {
    // SAFETY: the harness owns the queue and the data region.
    unsafe { serial_enqueue(&harness.handle, byte) }
}

fn dequeue(harness: &Harness) -> Result<u8, i32> {
    let mut byte = 0;
    // SAFETY: the harness owns the queue and the data region.
    let status = unsafe { serial_dequeue(&harness.handle, &mut byte) };
    if status == 0 {
        Ok(byte)
    } else {
        Err(status)
    }
}

#[test]
fn free_space_shrinks_on_enqueue_and_returns_on_dequeue() {
    let harness = harness(4);
    // SAFETY: the harness owns the queue and the data region.
    assert_eq!(unsafe { serial_queue_free(&harness.handle) }, 4);
    assert_eq!(enqueue(&harness, b'a'), 0);
    // SAFETY: the harness owns the queue and the data region.
    assert_eq!(unsafe { serial_queue_free(&harness.handle) }, 3);
    assert_eq!(dequeue(&harness).unwrap(), b'a');
    // SAFETY: the harness owns the queue and the data region.
    assert_eq!(unsafe { serial_queue_free(&harness.handle) }, 4);
}

#[test]
fn fill_to_capacity_then_dequeue_in_order() {
    let harness = harness(4);
    for byte in [b'w', b'x', b'y', b'z'] {
        assert_eq!(enqueue(&harness, byte), 0);
    }
    assert_eq!(enqueue(&harness, b'!'), -1);

    assert_eq!(dequeue(&harness).unwrap(), b'w');
    assert_eq!(dequeue(&harness).unwrap(), b'x');
    assert_eq!(dequeue(&harness).unwrap(), b'y');
    assert_eq!(dequeue(&harness).unwrap(), b'z');
    assert_eq!(dequeue(&harness).unwrap_err(), -1);
}

#[test]
fn indices_wrap_when_capacity_is_not_a_power_of_two() {
    let harness = harness(3);
    assert_eq!(enqueue(&harness, b'a'), 0);
    assert_eq!(enqueue(&harness, b'b'), 0);
    assert_eq!(enqueue(&harness, b'c'), 0);
    assert_eq!(enqueue(&harness, b'd'), -1);

    assert_eq!(dequeue(&harness).unwrap(), b'a');
    assert_eq!(enqueue(&harness, b'd'), 0);
    assert_eq!(enqueue(&harness, b'e'), -1);
    assert_eq!(dequeue(&harness).unwrap(), b'b');
    assert_eq!(dequeue(&harness).unwrap(), b'c');
    assert_eq!(dequeue(&harness).unwrap(), b'd');
    assert_eq!(dequeue(&harness).unwrap_err(), -1);
}

#[test]
fn shared_head_update_skips_to_that_index() {
    let harness = harness(3);
    assert_eq!(enqueue(&harness, b'a'), 0);
    assert_eq!(enqueue(&harness, b'b'), 0);
    assert_eq!(enqueue(&harness, b'c'), 0);
    // SAFETY: the harness owns the queue. Publishing head 2 drops `a` and `b`.
    unsafe { serial_update_shared_head(&harness.handle, 2) };
    assert_eq!(dequeue(&harness).unwrap(), b'c');
    assert_eq!(dequeue(&harness).unwrap_err(), -1);
}

#[test]
fn local_tail_stays_hidden_until_shared_update() {
    let harness = harness(4);
    let mut local_tail = 0;
    // SAFETY: the harness owns the queue. `local_tail` is the producer copy.
    unsafe {
        assert_eq!(
            serial_enqueue_local(&harness.handle, &mut local_tail, b'p'),
            0
        );
        assert_eq!(
            serial_enqueue_local(&harness.handle, &mut local_tail, b'q'),
            0
        );
    }
    assert_eq!(dequeue(&harness).unwrap_err(), -1);

    // SAFETY: the harness owns the queue. `local_tail` is the value just produced.
    unsafe { serial_update_shared_tail(&harness.handle, local_tail) };
    assert_eq!(dequeue(&harness).unwrap(), b'p');
    assert_eq!(dequeue(&harness).unwrap(), b'q');
    assert_eq!(dequeue(&harness).unwrap_err(), -1);
}

#[test]
fn consumer_signal_request_is_visible() {
    let harness = harness(4);
    // SAFETY: the harness owns the queue.
    unsafe { serial_request_consumer_signal(&harness.handle) };
    assert_eq!(harness.queue.producer_signalled.load(Ordering::Relaxed), 0);
    // SAFETY: the harness owns the queue.
    assert!(unsafe { serial_require_consumer_signal(&harness.handle) });

    // SAFETY: the harness owns the queue.
    unsafe { serial_cancel_consumer_signal(&harness.handle) };
    assert_eq!(harness.queue.producer_signalled.load(Ordering::Relaxed), 1);
    // SAFETY: the harness owns the queue.
    assert!(!unsafe { serial_require_consumer_signal(&harness.handle) });
}
