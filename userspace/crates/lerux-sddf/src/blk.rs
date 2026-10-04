//! Block request and response queues.
//!
//! Field order matches `include/sddf/blk/queue.h`. The queue types here are the
//! fixed prefix. The C types end in a flexible array of requests or responses.

pub const BLK_TRANSFER_SIZE: u32 = 4096;

#[repr(C)]
pub enum blk_req_code_t {
    BLK_REQ_READ = 0,
    BLK_REQ_WRITE = 1,
    BLK_REQ_FLUSH = 2,
    BLK_REQ_BARRIER = 3,
}

#[repr(C)]
pub enum blk_resp_status_t {
    BLK_RESP_OK = 0,
    BLK_RESP_ERR_UNSPEC = 1,
    BLK_RESP_ERR_INVALID_PARAM = 2,
    BLK_RESP_ERR_IO = 3,
    BLK_RESP_ERR_NO_DEVICE = 4,
}

#[repr(C)]
pub struct blk_req_t {
    pub code: blk_req_code_t,
    pub io_or_offset: u64,
    pub block_number: u64,
    pub count: u16,
    pub id: u32,
}

#[repr(C)]
pub struct blk_resp_t {
    pub status: blk_resp_status_t,
    pub success_count: u16,
    pub id: u32,
}

/// `#[repr(C, align(8))]` matches a GCC flexible array of 8-byte-aligned requests.
#[repr(C, align(8))]
pub struct blk_req_queue_t {
    pub head: u32,
    pub tail: u32,
    pub plugged: bool,
}

#[repr(C)]
pub struct blk_resp_queue_t {
    pub head: u32,
    pub tail: u32,
}

#[repr(C)]
pub struct blk_queue_handle_t {
    pub req_queue: *mut blk_req_queue_t,
    pub resp_queue: *mut blk_resp_queue_t,
    pub capacity: u32,
}
