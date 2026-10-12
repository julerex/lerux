//! Graphics queue operations from `include/sddf/gpu/queue.h`.
//!
//! The producer is the only writer of `tail`. The consumer is the only writer of `head`.
//! `head` and `tail` stay plain `u32` fields. The producer release-stores the index it owns.
//! The consumer acquire-loads the index the other side writes. The C inlines omit those fences.
//! This copy uses the symmetric-multiprocessing pairing from the plan on every new queue.

use core::sync::atomic::{AtomicU32, Ordering};

use crate::gpu::{gpu_queue_handle_t, gpu_req_queue_t, gpu_req_t, gpu_resp_queue_t, gpu_resp_t};

unsafe fn load_relaxed(word: *const u32) -> u32 {
    unsafe { (*word.cast::<AtomicU32>()).load(Ordering::Relaxed) }
}

unsafe fn load_acquire(word: *const u32) -> u32 {
    unsafe { (*word.cast::<AtomicU32>()).load(Ordering::Acquire) }
}

unsafe fn store_release(word: *mut u32, value: u32) {
    unsafe { (*word.cast::<AtomicU32>()).store(value, Ordering::Release) }
}

fn req_slot(queue: *mut gpu_req_queue_t, index: u32, capacity: u32) -> *mut gpu_req_t {
    let base = queue
        .cast::<u8>()
        .wrapping_add(core::mem::size_of::<gpu_req_queue_t>());
    base.cast::<gpu_req_t>()
        .wrapping_add((index % capacity) as usize)
}

fn resp_slot(queue: *mut gpu_resp_queue_t, index: u32, capacity: u32) -> *mut gpu_resp_t {
    let base = queue
        .cast::<u8>()
        .wrapping_add(core::mem::size_of::<gpu_resp_queue_t>());
    base.cast::<gpu_resp_t>()
        .wrapping_add((index % capacity) as usize)
}

/// # Safety
///
/// `request` and `response` must point at shared queues aligned for their indexes.
/// `capacity` must be non-zero. The flexible arrays start at `size_of` of each prefix.
pub unsafe fn gpu_queue_init(
    handle: &mut gpu_queue_handle_t,
    request: *mut gpu_req_queue_t,
    response: *mut gpu_resp_queue_t,
    capacity: u32,
) {
    handle.req_queue = request;
    handle.resp_queue = response;
    handle.capacity = capacity;
}

/// # Safety
///
/// Same pointer rules as [`gpu_queue_init`]. The caller is the producer of requests.
pub unsafe fn gpu_queue_full_req(handle: &gpu_queue_handle_t) -> bool {
    unsafe {
        let queue = handle.req_queue;
        let tail = load_relaxed(&raw const (*queue).tail);
        let head = load_acquire(&raw const (*queue).head);
        tail.wrapping_sub(head) == handle.capacity
    }
}

/// # Safety
///
/// Same pointer rules as [`gpu_queue_init`]. The caller is the producer of responses.
pub unsafe fn gpu_queue_full_resp(handle: &gpu_queue_handle_t) -> bool {
    unsafe {
        let queue = handle.resp_queue;
        let tail = load_relaxed(&raw const (*queue).tail);
        let head = load_acquire(&raw const (*queue).head);
        tail.wrapping_sub(head) == handle.capacity
    }
}

/// Enqueue one request. Returns -1 when `tail.wrapping_sub(head) == capacity`.
///
/// The request must be fully initialised. [`crate::gpu_req_new`] zeroes the padding
/// the C `memcpy` would otherwise copy.
///
/// # Safety
///
/// Same pointer rules as [`gpu_queue_init`]. The caller is the producer of requests.
pub unsafe fn gpu_enqueue_req(handle: &gpu_queue_handle_t, req: gpu_req_t) -> i32 {
    unsafe {
        if gpu_queue_full_req(handle) {
            return -1;
        }
        let queue = handle.req_queue;
        let tail = load_relaxed(&raw const (*queue).tail);
        req_slot(queue, tail, handle.capacity).write(req);
        store_release(&raw mut (*queue).tail, tail.wrapping_add(1));
        0
    }
}

/// Dequeue one request. Returns -1 when the queue is empty.
///
/// # Safety
///
/// Same pointer rules as [`gpu_queue_init`]. The caller is the consumer of requests.
pub unsafe fn gpu_dequeue_req(handle: &gpu_queue_handle_t, req: &mut gpu_req_t) -> i32 {
    unsafe {
        let queue = handle.req_queue;
        let head = load_relaxed(&raw const (*queue).head);
        let tail = load_acquire(&raw const (*queue).tail);
        if head == tail {
            return -1;
        }
        *req = req_slot(queue, head, handle.capacity).read();
        store_release(&raw mut (*queue).head, head.wrapping_add(1));
        0
    }
}

/// Enqueue one response. Returns -1 when `tail.wrapping_sub(head) == capacity`.
///
/// # Safety
///
/// Same pointer rules as [`gpu_queue_init`]. The caller is the producer of responses.
pub unsafe fn gpu_enqueue_resp(handle: &gpu_queue_handle_t, resp: gpu_resp_t) -> i32 {
    unsafe {
        if gpu_queue_full_resp(handle) {
            return -1;
        }
        let queue = handle.resp_queue;
        let tail = load_relaxed(&raw const (*queue).tail);
        resp_slot(queue, tail, handle.capacity).write(resp);
        store_release(&raw mut (*queue).tail, tail.wrapping_add(1));
        0
    }
}

/// Dequeue one response. Returns -1 when the queue is empty.
///
/// # Safety
///
/// Same pointer rules as [`gpu_queue_init`]. The caller is the consumer of responses.
pub unsafe fn gpu_dequeue_resp(handle: &gpu_queue_handle_t, resp: &mut gpu_resp_t) -> i32 {
    unsafe {
        let queue = handle.resp_queue;
        let head = load_relaxed(&raw const (*queue).head);
        let tail = load_acquire(&raw const (*queue).tail);
        if head == tail {
            return -1;
        }
        *resp = resp_slot(queue, head, handle.capacity).read();
        store_release(&raw mut (*queue).head, head.wrapping_add(1));
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::{gpu_req_code_t, gpu_req_new, gpu_resp_status_t, gpu_resp_t};

    #[repr(C, align(8))]
    struct Region([u8; 0x1000]);

    fn handle(req: &mut Region, resp: &mut Region, capacity: u32) -> gpu_queue_handle_t {
        let mut handle = gpu_queue_handle_t {
            req_queue: core::ptr::null_mut(),
            resp_queue: core::ptr::null_mut(),
            capacity: 0,
        };
        unsafe {
            gpu_queue_init(
                &mut handle,
                req.0.as_mut_ptr().cast(),
                resp.0.as_mut_ptr().cast(),
                capacity,
            );
        }
        handle
    }

    #[test]
    fn requests_round_trip_and_the_seventeenth_enqueue_fails() {
        let mut req_region = Region([0; 0x1000]);
        let mut resp_region = Region([0; 0x1000]);
        let handle = handle(&mut req_region, &mut resp_region, 16);
        for id in 0..16 {
            let request = gpu_req_new(gpu_req_code_t::GPU_REQ_RESOURCE_FLUSH, id);
            assert_eq!(unsafe { gpu_enqueue_req(&handle, request) }, 0);
        }
        let extra = gpu_req_new(gpu_req_code_t::GPU_REQ_RESOURCE_UNREF, 99);
        assert_eq!(unsafe { gpu_enqueue_req(&handle, extra) }, -1);
        for id in 0..16 {
            let mut got = gpu_req_new(gpu_req_code_t::GPU_REQ_GET_DISPLAY_INFO, 0);
            assert_eq!(unsafe { gpu_dequeue_req(&handle, &mut got) }, 0);
            assert_eq!(got.id, id);
            assert_eq!(got.code, gpu_req_code_t::GPU_REQ_RESOURCE_FLUSH);
        }
        let mut empty = gpu_req_new(gpu_req_code_t::GPU_REQ_GET_DISPLAY_INFO, 0);
        assert_eq!(unsafe { gpu_dequeue_req(&handle, &mut empty) }, -1);
    }

    #[test]
    fn responses_keep_status_and_id() {
        let mut req_region = Region([0; 0x1000]);
        let mut resp_region = Region([0; 0x1000]);
        let handle = handle(&mut req_region, &mut resp_region, 16);
        let resp = gpu_resp_t {
            status: gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER,
            id: 7,
        };
        assert_eq!(unsafe { gpu_enqueue_resp(&handle, resp) }, 0);
        let mut got = gpu_resp_t {
            status: gpu_resp_status_t::GPU_RESP_OK,
            id: 0,
        };
        assert_eq!(unsafe { gpu_dequeue_resp(&handle, &mut got) }, 0);
        assert_eq!(got, resp);
    }
}
