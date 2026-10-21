//! Graphics client for the browser image.
//!
//! The worker submits one request at a time. The root installs the queue
//! handles before the worker starts. A display-info response uses the client
//! data page, so pixels are written after that response returns.

use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicU32, Ordering},
};

use sel4_microkit::{Channel, ChannelSet};

use lerux_cothread::{microkit_cothread_recv_ntfn, microkit_cothread_wait_on_channel};
use lerux_sddf::{
    gpu_dequeue_resp, gpu_enqueue_req, gpu_formats_t, gpu_image, gpu_queue_handle_t,
    gpu_queue_init, gpu_rect_t, gpu_req_code_t, gpu_req_get_display_info_t, gpu_req_new,
    gpu_req_resource_attach_backing_t, gpu_req_resource_create_2d_t, gpu_req_resource_flush_t,
    gpu_req_set_scanout_t, gpu_req_t, gpu_req_transfer_to_2d_t, gpu_resp_status_t, gpu_resp_t,
};
use lerux_web::{PaintedPage, PixelLine};

pub const GPU_CH: usize = gpu_image::SHELL_GPU_CHANNEL as usize;

const _: () = assert!(
    PaintedPage::BYTES as u64 == gpu_image::GPU_CLIENT_DATA_SIZE,
    "client data region matches the painted frame"
);

const RESOURCE_ID: u32 = 1;

struct QueueCell(UnsafeCell<MaybeUninit<gpu_queue_handle_t>>);

// SAFETY: init writes this cell before the worker starts. The worker is the only caller afterwards.
unsafe impl Sync for QueueCell {}

static QUEUE: QueueCell = QueueCell(UnsafeCell::new(MaybeUninit::uninit()));
static NEXT_ID: AtomicU32 = AtomicU32::new(1);

pub fn install() {
    let mut handle = gpu_queue_handle_t {
        req_queue: core::ptr::null_mut(),
        resp_queue: core::ptr::null_mut(),
        capacity: 0,
    };
    // SAFETY: the template maps the client request and response queues.
    unsafe {
        gpu_queue_init(
            &mut handle,
            gpu_image::GPU_CLIENT_REQ_VADDR as *mut _,
            gpu_image::GPU_CLIENT_RESP_VADDR as *mut _,
            gpu_image::GPU_QUEUE_CAPACITY,
        );
        (*QUEUE.0.get()).write(handle);
    }
}

pub fn on_notify(channels: ChannelSet) {
    if channels.contains(Channel::new(GPU_CH)) {
        microkit_cothread_recv_ntfn(GPU_CH);
    }
}

/// Paint `html` and return the serial line sampled from that paint.
pub fn show_page(html: &[u8]) -> Result<PixelLine, ()> {
    let page = PaintedPage::render(html).map_err(|_| ())?;
    let rect = gpu_rect_t {
        x: 0,
        y: 0,
        width: page.width(),
        height: page.height(),
    };
    let mut info = gpu_req_new(gpu_req_code_t::GPU_REQ_GET_DISPLAY_INFO, next_id());
    info.payload.get_display_info = gpu_req_get_display_info_t { mem_offset: 0 };
    submit(info)?;
    copy_frame(&page);
    let mut created = gpu_req_new(gpu_req_code_t::GPU_REQ_RESOURCE_CREATE_2D, next_id());
    created.payload.resource_create_2d = gpu_req_resource_create_2d_t {
        resource_id: RESOURCE_ID,
        width: page.width(),
        height: page.height(),
        format: gpu_formats_t::GPU_FORMAT_B8G8R8A8_UNORM,
    };
    submit(created)?;
    let mem_size = u32::try_from(page.bytes().len()).map_err(|_| ())?;
    let mut attached = gpu_req_new(gpu_req_code_t::GPU_REQ_RESOURCE_ATTACH_BACKING, next_id());
    attached.payload.resource_attach_backing = gpu_req_resource_attach_backing_t {
        resource_id: RESOURCE_ID,
        mem_offset: 0,
        mem_size,
    };
    submit(attached)?;
    let mut scanout = gpu_req_new(gpu_req_code_t::GPU_REQ_SET_SCANOUT, next_id());
    scanout.payload.set_scanout = gpu_req_set_scanout_t {
        resource_id: RESOURCE_ID,
        scanout_id: 0,
        rect,
    };
    submit(scanout)?;
    let mut transfer = gpu_req_new(gpu_req_code_t::GPU_REQ_TRANSFER_TO_2D, next_id());
    transfer.payload.transfer_to_2d = gpu_req_transfer_to_2d_t {
        resource_id: RESOURCE_ID,
        rect,
        mem_offset: 0,
    };
    submit(transfer)?;
    let mut flush = gpu_req_new(gpu_req_code_t::GPU_REQ_RESOURCE_FLUSH, next_id());
    flush.payload.resource_flush = gpu_req_resource_flush_t {
        resource_id: RESOURCE_ID,
        rect,
    };
    submit(flush)?;
    Ok(*page.pixel_line())
}

fn copy_frame(page: &PaintedPage) {
    let dst = gpu_image::GPU_CLIENT_DATA_VADDR as *mut u8;
    let src = page.bytes();
    // SAFETY: the template maps `GPU_CLIENT_DATA_SIZE` bytes at this address.
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
    }
}

fn next_id() -> u32 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

fn queue() -> &'static gpu_queue_handle_t {
    // SAFETY: `install` wrote this cell before the worker was spawned.
    unsafe { (*QUEUE.0.get()).assume_init_ref() }
}

fn submit(req: gpu_req_t) -> Result<(), ()> {
    let handle = queue();
    // SAFETY: this client produces the request queue. One request is in flight.
    if unsafe { gpu_enqueue_req(handle, req) } != 0 {
        return Err(());
    }
    Channel::new(GPU_CH).notify();
    microkit_cothread_wait_on_channel(GPU_CH);
    let mut resp = gpu_resp_t {
        status: gpu_resp_status_t::GPU_RESP_ERR_UNSPEC,
        id: 0,
    };
    // SAFETY: this client consumes the response queue.
    if unsafe { gpu_dequeue_resp(handle, &mut resp) } != 0 {
        return Err(());
    }
    if resp.id != req.id || resp.status != gpu_resp_status_t::GPU_RESP_OK {
        return Err(());
    }
    Ok(())
}
