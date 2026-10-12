//! Graphics request and response queues from `include/sddf/gpu/gpu.h`,
//! `include/sddf/gpu/queue.h`, and `include/sddf/gpu/events.h`.
//!
//! The pinned headers have no `gpu_driver_config_t`. These types are the
//! request, response, and event structs. The queue prefixes here are the fixed
//! head and tail. The C queues end in a flexible array.

use core::sync::atomic::{AtomicU8, Ordering};

/// Resource ids are `1..=GPU_MAX_RESOURCES`. Zero disables a scanout.
pub const GPU_MAX_RESOURCES: usize = 1024;
pub const GPU_DISABLE_SCANOUT_RESOURCE_ID: u32 = 0;
pub const GPU_BPP_2D: u32 = 4;
pub const GPU_MAX_SCANOUTS: usize = 16;
pub const GPU_EVENTS_REGION_SIZE: usize = 0x1000;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_rect_t {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum gpu_formats_t {
    GPU_FORMAT_B8G8R8A8_UNORM = 0,
    GPU_FORMAT_B8G8R8X8_UNORM = 1,
    GPU_FORMAT_A8R8G8B8_UNORM = 2,
    GPU_FORMAT_X8R8G8B8_UNORM = 3,
    GPU_FORMAT_R8G8B8A8_UNORM = 4,
    GPU_FORMAT_X8B8G8R8_UNORM = 5,
    GPU_FORMAT_A8B8G8R8_UNORM = 6,
    GPU_FORMAT_R8G8B8X8_UNORM = 7,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_scanout_t {
    pub rect: gpu_rect_t,
    pub enabled: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_events_t {
    pub display_info: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_req_get_display_info_t {
    pub mem_offset: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_resp_get_display_info_t {
    pub scanouts: [gpu_scanout_t; GPU_MAX_SCANOUTS],
    pub num_scanouts: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_req_resource_create_blob_t {
    pub resource_id: u32,
    pub mem_offset: u64,
    pub mem_size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_req_set_scanout_blob_t {
    pub resource_id: u32,
    pub scanout_id: u32,
    pub width: u32,
    pub height: u32,
    pub format: gpu_formats_t,
    pub stride: u32,
    pub offset: u32,
    pub rect: gpu_rect_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_req_resource_create_2d_t {
    pub resource_id: u32,
    pub width: u32,
    pub height: u32,
    pub format: gpu_formats_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_req_resource_attach_backing_t {
    pub resource_id: u32,
    pub mem_offset: u64,
    pub mem_size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_req_resource_detach_backing_t {
    pub resource_id: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_req_set_scanout_t {
    pub resource_id: u32,
    pub scanout_id: u32,
    pub rect: gpu_rect_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_req_transfer_to_2d_t {
    pub resource_id: u32,
    pub rect: gpu_rect_t,
    pub mem_offset: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_req_resource_flush_t {
    pub resource_id: u32,
    pub rect: gpu_rect_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_req_resource_unref_t {
    pub resource_id: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum gpu_req_code_t {
    GPU_REQ_GET_DISPLAY_INFO = 0,
    GPU_REQ_RESOURCE_CREATE_BLOB = 1,
    GPU_REQ_SET_SCANOUT_BLOB = 2,
    GPU_REQ_RESOURCE_CREATE_2D = 3,
    GPU_REQ_SET_SCANOUT = 4,
    GPU_REQ_RESOURCE_ATTACH_BACKING = 5,
    GPU_REQ_RESOURCE_DETACH_BACKING = 6,
    GPU_REQ_TRANSFER_TO_2D = 7,
    GPU_REQ_RESOURCE_FLUSH = 8,
    GPU_REQ_RESOURCE_UNREF = 9,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum gpu_resp_status_t {
    GPU_RESP_OK = 0,
    GPU_RESP_ERR_UNSPEC = 1,
    GPU_RESP_ERR_INVALID_SCANOUT_ID = 2,
    GPU_RESP_ERR_INVALID_RESOURCE_ID = 3,
    GPU_RESP_ERR_INVALID_BOUNDS = 4,
    GPU_RESP_ERR_INVALID_PARAMETER = 5,
}

/// Anonymous in the C header. Field offsets of the members are the offset of this union.
#[repr(C)]
#[derive(Clone, Copy)]
pub union gpu_req_payload_t {
    pub get_display_info: gpu_req_get_display_info_t,
    pub resource_create_blob: gpu_req_resource_create_blob_t,
    pub set_scanout_blob: gpu_req_set_scanout_blob_t,
    pub resource_create_2d: gpu_req_resource_create_2d_t,
    pub resource_attach_backing: gpu_req_resource_attach_backing_t,
    pub resource_detach_backing: gpu_req_resource_detach_backing_t,
    pub set_scanout: gpu_req_set_scanout_t,
    pub transfer_to_2d: gpu_req_transfer_to_2d_t,
    pub resource_flush: gpu_req_resource_flush_t,
    pub resource_unref: gpu_req_resource_unref_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct gpu_req_t {
    pub code: gpu_req_code_t,
    pub id: u32,
    pub payload: gpu_req_payload_t,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct gpu_resp_t {
    pub status: gpu_resp_status_t,
    pub id: u32,
}

/// `#[repr(C, align(8))]` matches a GCC flexible array of 8-byte-aligned requests.
#[repr(C, align(8))]
pub struct gpu_req_queue_t {
    pub head: u32,
    pub tail: u32,
}

#[repr(C)]
pub struct gpu_resp_queue_t {
    pub head: u32,
    pub tail: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct gpu_queue_handle_t {
    pub req_queue: *mut gpu_req_queue_t,
    pub resp_queue: *mut gpu_resp_queue_t,
    pub capacity: u32,
}

/// Zero the request, then set the code and id. The payload stays zero until the caller assigns it.
pub fn gpu_req_new(code: gpu_req_code_t, id: u32) -> gpu_req_t {
    let mut req: gpu_req_t = unsafe { core::mem::zeroed() };
    req.code = code;
    req.id = id;
    req
}

/// Set the display-info event with a release store. The C helper stores `true`.
///
/// # Safety
///
/// `events` must point at a mapped `gpu_events_t`.
pub unsafe fn gpu_events_set_display_info(events: *mut gpu_events_t) {
    unsafe {
        (*(&raw mut (*events).display_info).cast::<AtomicU8>()).store(1, Ordering::Release);
    }
}

/// Clear the display-info event with a release store.
///
/// # Safety
///
/// `events` must point at a mapped `gpu_events_t`.
pub unsafe fn gpu_events_clear_display_info(events: *mut gpu_events_t) {
    unsafe {
        (*(&raw mut (*events).display_info).cast::<AtomicU8>()).store(0, Ordering::Release);
    }
}

/// Acquire-load the display-info event. A byte other than 0 is set.
///
/// # Safety
///
/// `events` must point at a mapped `gpu_events_t`.
pub unsafe fn gpu_events_check_display_info(events: *const gpu_events_t) -> bool {
    unsafe {
        (*(&raw const (*events).display_info).cast::<AtomicU8>()).load(Ordering::Acquire) != 0
    }
}

const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<gpu_rect_t>() == 16 && align_of::<gpu_rect_t>() == 4);
    assert!(size_of::<gpu_scanout_t>() == 20 && align_of::<gpu_scanout_t>() == 4);
    assert!(offset_of!(gpu_scanout_t, enabled) == 16);
    assert!(size_of::<gpu_events_t>() == 1 && align_of::<gpu_events_t>() == 1);
    assert!(size_of::<gpu_req_get_display_info_t>() == 8);
    assert!(size_of::<gpu_resp_get_display_info_t>() == 324);
    assert!(offset_of!(gpu_resp_get_display_info_t, num_scanouts) == 320);
    assert!(size_of::<gpu_req_resource_create_blob_t>() == 24);
    assert!(align_of::<gpu_req_resource_create_blob_t>() == 8);
    assert!(size_of::<gpu_req_set_scanout_blob_t>() == 44);
    assert!(size_of::<gpu_req_resource_create_2d_t>() == 16);
    assert!(size_of::<gpu_req_resource_attach_backing_t>() == 24);
    assert!(align_of::<gpu_req_resource_attach_backing_t>() == 8);
    assert!(size_of::<gpu_req_resource_detach_backing_t>() == 4);
    assert!(size_of::<gpu_req_set_scanout_t>() == 24);
    assert!(size_of::<gpu_req_transfer_to_2d_t>() == 32);
    assert!(align_of::<gpu_req_transfer_to_2d_t>() == 8);
    assert!(offset_of!(gpu_req_transfer_to_2d_t, mem_offset) == 24);
    assert!(size_of::<gpu_req_resource_flush_t>() == 20);
    assert!(size_of::<gpu_req_resource_unref_t>() == 4);
    assert!(size_of::<gpu_req_t>() == 56 && align_of::<gpu_req_t>() == 8);
    assert!(offset_of!(gpu_req_t, code) == 0);
    assert!(offset_of!(gpu_req_t, id) == 4);
    assert!(offset_of!(gpu_req_t, payload) == 8);
    assert!(size_of::<gpu_resp_t>() == 8 && align_of::<gpu_resp_t>() == 4);
    assert!(size_of::<gpu_req_queue_t>() == 8 && align_of::<gpu_req_queue_t>() == 8);
    assert!(size_of::<gpu_resp_queue_t>() == 8 && align_of::<gpu_resp_queue_t>() == 4);
    assert!(size_of::<gpu_queue_handle_t>() == 24 && align_of::<gpu_queue_handle_t>() == 8);
};
