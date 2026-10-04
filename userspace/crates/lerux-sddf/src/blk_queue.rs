//! Block queue operations from `include/sddf/blk/queue.h`.
//!
//! The producer is the only writer of `tail`. The consumer is the only writer of `head`.
//! `head` and `tail` stay plain `u32` fields. The producer release-stores the index it owns.
//! The consumer acquire-loads the index the other side writes.

use core::sync::atomic::{AtomicU32, Ordering};

use crate::blk::{
    blk_queue_handle_t, blk_req_code_t, blk_req_queue_t, blk_req_t, blk_resp_queue_t,
    blk_resp_status_t, blk_resp_t,
};

unsafe fn load_relaxed(word: *const u32) -> u32 {
    unsafe { (*word.cast::<AtomicU32>()).load(Ordering::Relaxed) }
}

unsafe fn load_acquire(word: *const u32) -> u32 {
    unsafe { (*word.cast::<AtomicU32>()).load(Ordering::Acquire) }
}

unsafe fn store_release(word: *mut u32, value: u32) {
    unsafe { (*word.cast::<AtomicU32>()).store(value, Ordering::Release) }
}

fn req_slot(queue: *mut blk_req_queue_t, index: u32, capacity: u32) -> *mut blk_req_t {
    let base = queue
        .cast::<u8>()
        .wrapping_add(core::mem::size_of::<blk_req_queue_t>());
    base.cast::<blk_req_t>()
        .wrapping_add((index % capacity) as usize)
}

fn resp_slot(queue: *mut blk_resp_queue_t, index: u32, capacity: u32) -> *mut blk_resp_t {
    let base = queue
        .cast::<u8>()
        .wrapping_add(core::mem::size_of::<blk_resp_queue_t>());
    base.cast::<blk_resp_t>()
        .wrapping_add((index % capacity) as usize)
}

/// # Safety
///
/// `request` and `response` must point at shared queues aligned for their indexes.
/// `capacity` must be non-zero. The flexible arrays start at `size_of` of each prefix.
pub unsafe fn blk_queue_init(
    handle: &mut blk_queue_handle_t,
    request: *mut blk_req_queue_t,
    response: *mut blk_resp_queue_t,
    capacity: u32,
) {
    handle.req_queue = request;
    handle.resp_queue = response;
    handle.capacity = capacity;
}

/// # Safety
///
/// Same pointer rules as [`blk_queue_init`]. The caller is the producer of requests.
pub unsafe fn blk_queue_full_req(handle: &blk_queue_handle_t) -> bool {
    unsafe {
        let queue = handle.req_queue;
        let tail = load_relaxed(&raw const (*queue).tail);
        let head = load_acquire(&raw const (*queue).head);
        tail.wrapping_sub(head) == handle.capacity
    }
}

/// # Safety
///
/// Same pointer rules as [`blk_queue_init`]. The caller is the producer of responses.
pub unsafe fn blk_queue_full_resp(handle: &blk_queue_handle_t) -> bool {
    unsafe {
        let queue = handle.resp_queue;
        let tail = load_relaxed(&raw const (*queue).tail);
        let head = load_acquire(&raw const (*queue).head);
        tail.wrapping_sub(head) == handle.capacity
    }
}

/// Enqueue one request. Returns -1 when `tail.wrapping_sub(head) == capacity`.
///
/// # Safety
///
/// Same pointer rules as [`blk_queue_init`]. The caller is the producer of requests.
pub unsafe fn blk_enqueue_req(
    handle: &blk_queue_handle_t,
    code: blk_req_code_t,
    io_or_offset: u64,
    block_number: u64,
    count: u16,
    id: u32,
) -> i32 {
    unsafe {
        if blk_queue_full_req(handle) {
            return -1;
        }
        let queue = handle.req_queue;
        let tail = load_relaxed(&raw const (*queue).tail);
        let mut req = core::mem::zeroed::<blk_req_t>();
        req.code = code;
        req.io_or_offset = io_or_offset;
        req.block_number = block_number;
        req.count = count;
        req.id = id;
        req_slot(queue, tail, handle.capacity).write(req);
        store_release(&raw mut (*queue).tail, tail.wrapping_add(1));
        0
    }
}

/// Dequeue one request. Returns -1 when the queue is empty.
///
/// # Safety
///
/// Same pointer rules as [`blk_queue_init`]. The caller is the consumer of requests.
pub unsafe fn blk_dequeue_req(
    handle: &blk_queue_handle_t,
    code: &mut blk_req_code_t,
    io_or_offset: &mut u64,
    block_number: &mut u64,
    count: &mut u16,
    id: &mut u32,
) -> i32 {
    unsafe {
        let queue = handle.req_queue;
        let head = load_relaxed(&raw const (*queue).head);
        let tail = load_acquire(&raw const (*queue).tail);
        if head == tail {
            return -1;
        }
        let req = req_slot(queue, head, handle.capacity).read();
        *code = req.code;
        *io_or_offset = req.io_or_offset;
        *block_number = req.block_number;
        *count = req.count;
        *id = req.id;
        store_release(&raw mut (*queue).head, head.wrapping_add(1));
        0
    }
}

/// Enqueue one response. Returns -1 when `tail.wrapping_sub(head) == capacity`.
///
/// # Safety
///
/// Same pointer rules as [`blk_queue_init`]. The caller is the producer of responses.
pub unsafe fn blk_enqueue_resp(
    handle: &blk_queue_handle_t,
    status: blk_resp_status_t,
    success_count: u16,
    id: u32,
) -> i32 {
    unsafe {
        if blk_queue_full_resp(handle) {
            return -1;
        }
        let queue = handle.resp_queue;
        let tail = load_relaxed(&raw const (*queue).tail);
        let mut resp = core::mem::zeroed::<blk_resp_t>();
        resp.status = status;
        resp.success_count = success_count;
        resp.id = id;
        resp_slot(queue, tail, handle.capacity).write(resp);
        store_release(&raw mut (*queue).tail, tail.wrapping_add(1));
        0
    }
}

/// Dequeue one response. Returns -1 when the queue is empty.
///
/// # Safety
///
/// Same pointer rules as [`blk_queue_init`]. The caller is the consumer of responses.
pub unsafe fn blk_dequeue_resp(
    handle: &blk_queue_handle_t,
    status: &mut blk_resp_status_t,
    success_count: &mut u16,
    id: &mut u32,
) -> i32 {
    unsafe {
        let queue = handle.resp_queue;
        let head = load_relaxed(&raw const (*queue).head);
        let tail = load_acquire(&raw const (*queue).tail);
        if head == tail {
            return -1;
        }
        let resp = resp_slot(queue, head, handle.capacity).read();
        *status = resp.status;
        *success_count = resp.success_count;
        *id = resp.id;
        store_release(&raw mut (*queue).head, head.wrapping_add(1));
        0
    }
}
