//! Virtio graphics driver for the one-client browser image.
//!
//! The public framebuffer borrow from `change_resolution` does not outlive that
//! call, and the crate has no later getter. A flush calls `change_resolution`
//! again, copies the client pixels into that borrow, drops it, then flushes.
//! Blob resources are rejected. The headers have no driver configuration struct.

#![no_std]
#![no_main]

extern crate alloc;

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_logging::{debug, log};
use lerux_sddf::{
    gpu_dequeue_req, gpu_enqueue_resp, gpu_events_set_display_info, gpu_events_t, gpu_formats_t,
    gpu_image::{
        DRIVER_IRQ_CHANNEL, DRIVER_VIRT_CHANNEL, GPU_CLIENT_DATA_VADDR, GPU_DRIVER_DATA_VADDR,
        GPU_DRIVER_DMA_SIZE, GPU_DRIVER_EVENTS_VADDR, GPU_DRIVER_REQ_VADDR, GPU_DRIVER_RESP_VADDR,
        GPU_QUEUE_CAPACITY, GPU_REGION_SIZE,
    },
    gpu_queue_full_resp, gpu_queue_handle_t, gpu_queue_init, gpu_rect_t, gpu_req_code_t,
    gpu_req_resource_attach_backing_t, gpu_req_resource_create_2d_t, gpu_req_resource_flush_t,
    gpu_req_set_scanout_t, gpu_req_t, gpu_req_transfer_to_2d_t, gpu_resp_get_display_info_t,
    gpu_resp_status_t, gpu_resp_t, gpu_scanout_t, GPU_BPP_2D, GPU_DISABLE_SCANOUT_RESOURCE_ID,
    GPU_MAX_RESOURCES,
};

mod mmio;

#[derive(Clone, Copy)]
struct Transfer {
    rect: gpu_rect_t,
    mem_offset: u64,
}

struct Resource {
    id: u32,
    width: u32,
    height: u32,
    mem_offset: u64,
    mem_size: u32,
    attached: bool,
    transfer: Option<Transfer>,
}

struct HandlerImpl {
    dev: mmio::GpuDev,
    queue: gpu_queue_handle_t,
    irq: Channel,
    virt_ch: Channel,
    resource: Option<Resource>,
}

#[protection_domain(heap_size = 256 * 1024)]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    mmio::init_hal();
    let mut dev = mmio::create_virtio_gpu();
    let _ = dev.ack_interrupt();
    let irq = Channel::new(usize::from(DRIVER_IRQ_CHANNEL));
    irq.irq_ack().expect("irq ack");
    let mut queue = gpu_queue_handle_t {
        req_queue: core::ptr::null_mut(),
        resp_queue: core::ptr::null_mut(),
        capacity: 0,
    };
    // SAFETY: the template maps the driver request and response queues.
    unsafe {
        gpu_queue_init(
            &mut queue,
            GPU_DRIVER_REQ_VADDR as *mut _,
            GPU_DRIVER_RESP_VADDR as *mut _,
            GPU_QUEUE_CAPACITY,
        );
    }
    log::info!("gpu_driver: ready");
    HandlerImpl {
        dev,
        queue,
        irq,
        virt_ch: Channel::new(usize::from(DRIVER_VIRT_CHANNEL)),
        resource: None,
    }
}

fn rect_inside(rect: gpu_rect_t, width: u32, height: u32) -> bool {
    let Some(x1) = rect.x.checked_add(rect.width) else {
        return false;
    };
    let Some(y1) = rect.y.checked_add(rect.height) else {
        return false;
    };
    rect.width > 0 && rect.height > 0 && x1 <= width && y1 <= height
}

fn region_ptr(offset: u64, len: usize) -> Option<*const u8> {
    let off = usize::try_from(offset).ok()?;
    let end = off.checked_add(len)?;
    if end > usize::try_from(GPU_REGION_SIZE).expect("region size") {
        return None;
    }
    Some((GPU_CLIENT_DATA_VADDR as *const u8).wrapping_add(off))
}

fn copy_rect(dst: &mut [u8], origin: u64, rect: gpu_rect_t, device_width: u32) -> bool {
    let bpp = usize::try_from(GPU_BPP_2D).expect("bytes per pixel");
    let width = usize::try_from(rect.width).expect("width");
    let height = usize::try_from(rect.height).expect("height");
    let x = usize::try_from(rect.x).expect("x");
    let y = usize::try_from(rect.y).expect("y");
    let dst_stride = usize::try_from(device_width).expect("device width") * bpp;
    let row_bytes = width * bpp;
    for row in 0..height {
        let src_off = origin + u64::try_from(row * row_bytes).expect("source row");
        let dst_off = (y + row) * dst_stride + x * bpp;
        let Some(src) = region_ptr(src_off, row_bytes) else {
            return false;
        };
        let Some(dst_row) = dst.get_mut(dst_off..dst_off + row_bytes) else {
            return false;
        };
        // SAFETY: `src` is `row_bytes` inside the mapped client data region.
        unsafe { core::ptr::copy_nonoverlapping(src, dst_row.as_mut_ptr(), row_bytes) };
    }
    true
}

impl HandlerImpl {
    fn service(&mut self) {
        loop {
            // SAFETY: this protection domain produces the driver response queue.
            if unsafe { gpu_queue_full_resp(&self.queue) } {
                return;
            }
            let mut req = lerux_sddf::gpu_req_new(gpu_req_code_t::GPU_REQ_GET_DISPLAY_INFO, 0);
            // SAFETY: this protection domain consumes the driver request queue.
            if unsafe { gpu_dequeue_req(&self.queue, &mut req) } != 0 {
                return;
            }
            let status = self.handle(&req);
            let resp = gpu_resp_t { status, id: req.id };
            // SAFETY: the full check above is the enqueue condition.
            if unsafe { gpu_enqueue_resp(&self.queue, resp) } != 0 {
                return;
            }
            self.virt_ch.notify();
        }
    }

    fn handle(&mut self, req: &gpu_req_t) -> gpu_resp_status_t {
        match req.code {
            gpu_req_code_t::GPU_REQ_GET_DISPLAY_INFO => {
                let offset = unsafe { req.payload.get_display_info.mem_offset };
                self.display_info(offset)
            }
            gpu_req_code_t::GPU_REQ_RESOURCE_CREATE_2D => {
                self.create_2d(unsafe { req.payload.resource_create_2d })
            }
            gpu_req_code_t::GPU_REQ_RESOURCE_ATTACH_BACKING => {
                self.attach(unsafe { req.payload.resource_attach_backing })
            }
            gpu_req_code_t::GPU_REQ_RESOURCE_DETACH_BACKING => {
                self.detach(unsafe { req.payload.resource_detach_backing.resource_id })
            }
            gpu_req_code_t::GPU_REQ_SET_SCANOUT => self.scanout(unsafe { req.payload.set_scanout }),
            gpu_req_code_t::GPU_REQ_TRANSFER_TO_2D => {
                self.transfer(unsafe { req.payload.transfer_to_2d })
            }
            gpu_req_code_t::GPU_REQ_RESOURCE_FLUSH => {
                self.flush(unsafe { req.payload.resource_flush })
            }
            gpu_req_code_t::GPU_REQ_RESOURCE_UNREF => {
                self.unref(unsafe { req.payload.resource_unref.resource_id })
            }
            gpu_req_code_t::GPU_REQ_RESOURCE_CREATE_BLOB
            | gpu_req_code_t::GPU_REQ_SET_SCANOUT_BLOB => {
                gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER
            }
        }
    }

    fn display_info(&mut self, offset: u64) -> gpu_resp_status_t {
        let Ok((width, height)) = self.dev.resolution() else {
            return gpu_resp_status_t::GPU_RESP_ERR_UNSPEC;
        };
        if !write_display_info(offset, width, height) {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_BOUNDS;
        }
        // SAFETY: the template maps the driver event word.
        unsafe { gpu_events_set_display_info(GPU_DRIVER_EVENTS_VADDR as *mut gpu_events_t) };
        gpu_resp_status_t::GPU_RESP_OK
    }

    fn create_2d(&mut self, body: gpu_req_resource_create_2d_t) -> gpu_resp_status_t {
        if self.resource.is_some() {
            return gpu_resp_status_t::GPU_RESP_ERR_UNSPEC;
        }
        let Ok(max_resources) = u32::try_from(GPU_MAX_RESOURCES) else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER;
        };
        if body.resource_id == GPU_DISABLE_SCANOUT_RESOURCE_ID || body.resource_id > max_resources {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER;
        }
        if body.format != gpu_formats_t::GPU_FORMAT_B8G8R8A8_UNORM
            || body.width == 0
            || body.height == 0
        {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER;
        }
        let Some(pixels) = body.width.checked_mul(body.height) else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER;
        };
        let Some(bytes) = pixels.checked_mul(GPU_BPP_2D) else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER;
        };
        if u64::from(bytes) > GPU_DRIVER_DMA_SIZE / 2 {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER;
        }
        if self.dev.change_resolution(body.width, body.height).is_err() {
            return gpu_resp_status_t::GPU_RESP_ERR_UNSPEC;
        }
        self.resource = Some(Resource {
            id: body.resource_id,
            width: body.width,
            height: body.height,
            mem_offset: 0,
            mem_size: 0,
            attached: false,
            transfer: None,
        });
        gpu_resp_status_t::GPU_RESP_OK
    }

    fn attach(&mut self, body: gpu_req_resource_attach_backing_t) -> gpu_resp_status_t {
        let Some(resource) = &mut self.resource else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_RESOURCE_ID;
        };
        if resource.id != body.resource_id {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_RESOURCE_ID;
        }
        if resource.attached {
            return gpu_resp_status_t::GPU_RESP_ERR_UNSPEC;
        }
        let Some(pixels) = resource.width.checked_mul(resource.height) else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER;
        };
        let Some(need) = pixels.checked_mul(GPU_BPP_2D) else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER;
        };
        if body.mem_size < need {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER;
        }
        let Some(end) = body.mem_offset.checked_add(u64::from(body.mem_size)) else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_BOUNDS;
        };
        if end > GPU_REGION_SIZE {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_BOUNDS;
        }
        resource.mem_offset = body.mem_offset;
        resource.mem_size = body.mem_size;
        resource.attached = true;
        gpu_resp_status_t::GPU_RESP_OK
    }

    fn detach(&mut self, resource_id: u32) -> gpu_resp_status_t {
        let Some(resource) = &mut self.resource else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_RESOURCE_ID;
        };
        if resource.id != resource_id {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_RESOURCE_ID;
        }
        resource.attached = false;
        resource.transfer = None;
        gpu_resp_status_t::GPU_RESP_OK
    }

    fn scanout(&mut self, body: gpu_req_set_scanout_t) -> gpu_resp_status_t {
        if body.scanout_id != 0 {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_SCANOUT_ID;
        }
        if body.resource_id == GPU_DISABLE_SCANOUT_RESOURCE_ID {
            return gpu_resp_status_t::GPU_RESP_OK;
        }
        let Some(resource) = &self.resource else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_RESOURCE_ID;
        };
        if resource.id != body.resource_id {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_RESOURCE_ID;
        }
        if !rect_inside(body.rect, resource.width, resource.height) {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_BOUNDS;
        }
        gpu_resp_status_t::GPU_RESP_OK
    }

    fn transfer(&mut self, body: gpu_req_transfer_to_2d_t) -> gpu_resp_status_t {
        let Some(resource) = &mut self.resource else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_RESOURCE_ID;
        };
        if resource.id != body.resource_id || !resource.attached {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_RESOURCE_ID;
        }
        if !rect_inside(body.rect, resource.width, resource.height) {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_BOUNDS;
        }
        let Some(origin) = resource.mem_offset.checked_add(body.mem_offset) else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_BOUNDS;
        };
        let Some(bytes) = u64::from(body.rect.width)
            .checked_mul(u64::from(body.rect.height))
            .and_then(|pixels| pixels.checked_mul(u64::from(GPU_BPP_2D)))
        else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER;
        };
        let Some(end) = origin.checked_add(bytes) else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_BOUNDS;
        };
        let backing_end = resource.mem_offset + u64::from(resource.mem_size);
        if end > backing_end || end > GPU_REGION_SIZE {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_BOUNDS;
        }
        resource.transfer = Some(Transfer {
            rect: body.rect,
            mem_offset: body.mem_offset,
        });
        gpu_resp_status_t::GPU_RESP_OK
    }

    fn flush(&mut self, body: gpu_req_resource_flush_t) -> gpu_resp_status_t {
        let (width, height, origin, rect) = {
            let Some(resource) = &self.resource else {
                return gpu_resp_status_t::GPU_RESP_ERR_INVALID_RESOURCE_ID;
            };
            if resource.id != body.resource_id || !resource.attached {
                return gpu_resp_status_t::GPU_RESP_ERR_INVALID_RESOURCE_ID;
            }
            if !rect_inside(body.rect, resource.width, resource.height) {
                return gpu_resp_status_t::GPU_RESP_ERR_INVALID_BOUNDS;
            }
            let Some(transfer) = resource.transfer else {
                return gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER;
            };
            let Some(origin) = resource.mem_offset.checked_add(transfer.mem_offset) else {
                return gpu_resp_status_t::GPU_RESP_ERR_INVALID_BOUNDS;
            };
            (resource.width, resource.height, origin, transfer.rect)
        };
        let buf = match self.dev.change_resolution(width, height) {
            Ok(buf) => buf,
            Err(_) => return gpu_resp_status_t::GPU_RESP_ERR_UNSPEC,
        };
        if !copy_rect(buf, origin, rect, width) {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_BOUNDS;
        }
        if self.dev.flush().is_err() {
            return gpu_resp_status_t::GPU_RESP_ERR_UNSPEC;
        }
        gpu_resp_status_t::GPU_RESP_OK
    }

    fn unref(&mut self, resource_id: u32) -> gpu_resp_status_t {
        let Some(resource) = &self.resource else {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_RESOURCE_ID;
        };
        if resource.id != resource_id {
            return gpu_resp_status_t::GPU_RESP_ERR_INVALID_RESOURCE_ID;
        }
        self.resource = None;
        gpu_resp_status_t::GPU_RESP_OK
    }
}

fn write_display_info(offset: u64, width: u32, height: u32) -> bool {
    let len = core::mem::size_of::<gpu_resp_get_display_info_t>();
    let Some(off) = usize::try_from(offset).ok() else {
        return false;
    };
    if off
        .checked_add(len)
        .is_none_or(|end| end > usize::try_from(GPU_REGION_SIZE).expect("region"))
    {
        return false;
    }
    let mut info: gpu_resp_get_display_info_t = unsafe { core::mem::zeroed() };
    info.num_scanouts = 1;
    info.scanouts[0] = gpu_scanout_t {
        rect: gpu_rect_t {
            x: 0,
            y: 0,
            width,
            height,
        },
        enabled: width > 0 && height > 0,
    };
    // SAFETY: `off + len` fits in the mapped driver data region.
    unsafe {
        let dst = (GPU_DRIVER_DATA_VADDR as *mut u8).add(off);
        core::ptr::copy_nonoverlapping((&raw const info).cast::<u8>(), dst, len);
    }
    true
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        let irq = channels.contains(self.irq);
        if irq {
            self.dev.ack_interrupt();
            self.irq.irq_ack().expect("irq ack");
        }
        if irq || channels.contains(self.virt_ch) {
            self.service();
        }
        Ok(())
    }
}
