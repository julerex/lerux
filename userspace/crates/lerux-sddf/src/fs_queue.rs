//! Filesystem queue operations from `include/lions/fs/protocol.h`.
//!
//! A push is the queue half of `fs_command_issue`: refuse a full ring, write
//! one `fs_msg_t` at the producer index, then publish that index with a release
//! store. The notification to the server is the caller's job.

use core::sync::atomic::Ordering;

use crate::fs::{fs_cmd_t, fs_msg_t, fs_queue_t, FS_QUEUE_CAPACITY};

/// # Safety
///
/// `queue` must point at a shared `fs_queue_t`. The caller is the consumer and
/// is the only writer of `head`.
pub unsafe fn fs_queue_length_consumer(queue: *const fs_queue_t) -> u64 {
    unsafe {
        let tail = (*queue).tail.load(Ordering::Acquire);
        let head = (*queue).head.load(Ordering::Relaxed);
        tail.wrapping_sub(head)
    }
}

/// # Safety
///
/// `queue` must point at a shared `fs_queue_t`. The caller is the producer and
/// is the only writer of `tail`.
pub unsafe fn fs_queue_length_producer(queue: *const fs_queue_t) -> u64 {
    unsafe {
        let tail = (*queue).tail.load(Ordering::Relaxed);
        let head = (*queue).head.load(Ordering::Acquire);
        tail.wrapping_sub(head)
    }
}

/// # Safety
///
/// `queue` must point at a shared `fs_queue_t`. The caller is the consumer.
pub unsafe fn fs_queue_publish_consumption(queue: *mut fs_queue_t, amount_consumed: u64) {
    unsafe {
        let head = (*queue).head.load(Ordering::Relaxed);
        (*queue)
            .head
            .store(head.wrapping_add(amount_consumed), Ordering::Release);
    }
}

/// # Safety
///
/// `queue` must point at a shared `fs_queue_t`. The caller is the producer.
pub unsafe fn fs_queue_publish_production(queue: *mut fs_queue_t, amount_produced: u64) {
    unsafe {
        let tail = (*queue).tail.load(Ordering::Relaxed);
        (*queue)
            .tail
            .store(tail.wrapping_add(amount_produced), Ordering::Release);
    }
}

/// Enqueue one command. Returns -1 when the ring already holds [`FS_QUEUE_CAPACITY`] messages.
///
/// # Safety
///
/// `queue` must point at a shared `fs_queue_t`. The caller is the producer.
pub unsafe fn fs_command_enqueue(queue: *mut fs_queue_t, cmd: fs_cmd_t) -> i32 {
    unsafe {
        let tail = (*queue).tail.load(Ordering::Relaxed);
        let head = (*queue).head.load(Ordering::Acquire);
        if tail.wrapping_sub(head) == FS_QUEUE_CAPACITY as u64 {
            return -1;
        }
        let index = (tail % FS_QUEUE_CAPACITY as u64) as usize;
        let slot = core::ptr::addr_of_mut!((*queue).buffer).cast::<fs_msg_t>();
        slot.add(index).write(fs_msg_t { cmd });
        (*queue).tail.store(tail.wrapping_add(1), Ordering::Release);
        0
    }
}

/// Dequeue one message. Returns -1 when the ring is empty.
///
/// # Safety
///
/// `queue` must point at a shared `fs_queue_t`. The caller is the consumer.
pub unsafe fn fs_message_dequeue(queue: *mut fs_queue_t, msg: &mut fs_msg_t) -> i32 {
    unsafe {
        let head = (*queue).head.load(Ordering::Relaxed);
        let tail = (*queue).tail.load(Ordering::Acquire);
        if head == tail {
            return -1;
        }
        let index = (head % FS_QUEUE_CAPACITY as u64) as usize;
        let slot = core::ptr::addr_of!((*queue).buffer).cast::<fs_msg_t>();
        *msg = slot.add(index).read();
        (*queue).head.store(head.wrapping_add(1), Ordering::Release);
        0
    }
}
