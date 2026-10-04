//! Block request and response queues, and the block configuration pages.
//!
//! Field order matches `include/sddf/blk/queue.h`, `include/sddf/blk/config.h`,
//! and `include/sddf/blk/storage_info.h`. The queue types here are the fixed
//! prefix. The C types end in a flexible array of requests or responses.

use core::sync::atomic::{AtomicU8, Ordering};

use crate::{device_region_resource_t, region_resource_t};

pub const BLK_TRANSFER_SIZE: u32 = 4096;

pub const SDDF_BLK_MAX_CLIENTS: usize = 64;
pub const SDDF_BLK_MAGIC_LEN: usize = 5;

/// Header magic `sDDF` followed by `0x02`.
pub const SDDF_BLK_MAGIC: [u8; SDDF_BLK_MAGIC_LEN] = [b's', b'D', b'D', b'F', 0x02];

pub const BLK_STORAGE_INFO_REGION_SIZE: usize = 0x1000;
pub const BLK_MAX_SERIAL_NUMBER: usize = 63;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum blk_req_code_t {
    BLK_REQ_READ = 0,
    BLK_REQ_WRITE = 1,
    BLK_REQ_FLUSH = 2,
    BLK_REQ_BARRIER = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy)]
pub struct blk_queue_handle_t {
    pub req_queue: *mut blk_req_queue_t,
    pub resp_queue: *mut blk_resp_queue_t,
    pub capacity: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct blk_connection_resource_t {
    pub storage_info: region_resource_t,
    pub req_queue: region_resource_t,
    pub resp_queue: region_resource_t,
    pub num_buffers: u16,
    pub id: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct blk_driver_config_t {
    pub magic: [u8; SDDF_BLK_MAGIC_LEN],
    pub virt: blk_connection_resource_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct blk_virt_config_driver_t {
    pub conn: blk_connection_resource_t,
    pub data: device_region_resource_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct blk_virt_config_client_t {
    pub conn: blk_connection_resource_t,
    pub data: device_region_resource_t,
    pub partition: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct blk_virt_config_t {
    pub magic: [u8; SDDF_BLK_MAGIC_LEN],
    pub num_clients: u64,
    pub driver: blk_virt_config_driver_t,
    pub clients: [blk_virt_config_client_t; SDDF_BLK_MAX_CLIENTS],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct blk_client_config_t {
    pub magic: [u8; SDDF_BLK_MAGIC_LEN],
    pub virt: blk_connection_resource_t,
    pub data: region_resource_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct blk_storage_info_t {
    pub serial_number: [u8; BLK_MAX_SERIAL_NUMBER + 1],
    pub read_only: bool,
    pub ready: bool,
    pub sector_size: u16,
    pub block_size: u16,
    pub queue_depth: u16,
    pub cylinders: u16,
    pub heads: u16,
    pub blocks: u16,
    pub capacity: u64,
}

/// True when `config` starts with [`SDDF_BLK_MAGIC`].
pub fn blk_config_check_magic(config: &[u8]) -> bool {
    config.len() >= SDDF_BLK_MAGIC_LEN && config[..SDDF_BLK_MAGIC_LEN] == SDDF_BLK_MAGIC
}

/// Release-store the `ready` byte.
///
/// # Safety
///
/// `storage_info` must point at a shared `blk_storage_info_t`. The caller is the
/// only writer of `ready`.
pub unsafe fn blk_storage_set_ready(storage_info: *mut blk_storage_info_t, ready: bool) {
    let byte = u8::from(ready);
    unsafe {
        let ready_byte = &raw mut (*storage_info).ready;
        (*ready_byte.cast::<AtomicU8>()).store(byte, Ordering::Release);
    }
}

/// Acquire-load the `ready` byte.
///
/// # Safety
///
/// `storage_info` must point at a shared `blk_storage_info_t`.
pub unsafe fn blk_storage_is_ready(storage_info: *const blk_storage_info_t) -> bool {
    unsafe {
        let ready_byte = &raw const (*storage_info).ready;
        (*ready_byte.cast::<AtomicU8>()).load(Ordering::Acquire) != 0
    }
}
