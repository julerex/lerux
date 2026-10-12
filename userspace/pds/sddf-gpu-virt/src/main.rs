//! One-client graphics virtualiser.
//!
//! Requests are forwarded with the client's resource ids. A display-info
//! response is copied from the driver data region into the client data region.
//! Blob requests are rejected here and are not forwarded. There is no
//! configuration struct in the pinned graphics headers.

#![no_std]
#![no_main]

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_logging::{debug, log};
use lerux_sddf::{
    gpu_dequeue_req, gpu_dequeue_resp, gpu_enqueue_req, gpu_enqueue_resp,
    gpu_events_check_display_info, gpu_events_clear_display_info, gpu_events_set_display_info,
    gpu_events_t,
    gpu_image::{
        GPU_CLIENT_DATA_VADDR, GPU_CLIENT_EVENTS_VADDR, GPU_CLIENT_REQ_VADDR,
        GPU_CLIENT_RESP_VADDR, GPU_DRIVER_DATA_VADDR, GPU_DRIVER_EVENTS_VADDR,
        GPU_DRIVER_REQ_VADDR, GPU_DRIVER_RESP_VADDR, GPU_QUEUE_CAPACITY, GPU_REGION_SIZE,
        VIRT_CLIENT_CHANNEL, VIRT_DRIVER_CHANNEL,
    },
    gpu_queue_full_req, gpu_queue_full_resp, gpu_queue_handle_t, gpu_queue_init, gpu_req_code_t,
    gpu_req_new, gpu_resp_get_display_info_t, gpu_resp_status_t, gpu_resp_t,
};

const NOTE_SLOTS: usize = 16;

struct Note {
    code: gpu_req_code_t,
    mem_offset: u64,
    live: bool,
}

struct HandlerImpl {
    driver: gpu_queue_handle_t,
    client: gpu_queue_handle_t,
    driver_ch: Channel,
    client_ch: Channel,
    notes: [Note; NOTE_SLOTS],
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    log::info!("gpu_virt: ready");
    HandlerImpl {
        driver: queue_at(GPU_DRIVER_REQ_VADDR, GPU_DRIVER_RESP_VADDR),
        client: queue_at(GPU_CLIENT_REQ_VADDR, GPU_CLIENT_RESP_VADDR),
        driver_ch: Channel::new(usize::from(VIRT_DRIVER_CHANNEL)),
        client_ch: Channel::new(usize::from(VIRT_CLIENT_CHANNEL)),
        notes: [const {
            Note {
                code: gpu_req_code_t::GPU_REQ_GET_DISPLAY_INFO,
                mem_offset: 0,
                live: false,
            }
        }; NOTE_SLOTS],
    }
}

fn queue_at(req: u64, resp: u64) -> gpu_queue_handle_t {
    let mut handle = gpu_queue_handle_t {
        req_queue: core::ptr::null_mut(),
        resp_queue: core::ptr::null_mut(),
        capacity: 0,
    };
    // SAFETY: the template maps these queues into this protection domain.
    unsafe {
        gpu_queue_init(
            &mut handle,
            req as *mut _,
            resp as *mut _,
            GPU_QUEUE_CAPACITY,
        );
    }
    handle
}

fn is_blob(code: gpu_req_code_t) -> bool {
    matches!(
        code,
        gpu_req_code_t::GPU_REQ_RESOURCE_CREATE_BLOB | gpu_req_code_t::GPU_REQ_SET_SCANOUT_BLOB
    )
}

fn copy_display(offset: u64) {
    let len = core::mem::size_of::<gpu_resp_get_display_info_t>();
    let Some(off) = usize::try_from(offset).ok() else {
        return;
    };
    if off
        .checked_add(len)
        .is_none_or(|end| end > usize::try_from(GPU_REGION_SIZE).expect("region"))
    {
        return;
    }
    // SAFETY: both data regions are mapped, and `off + len` fits in each.
    unsafe {
        let src = (GPU_DRIVER_DATA_VADDR as *const u8).add(off);
        let dst = (GPU_CLIENT_DATA_VADDR as *mut u8).add(off);
        core::ptr::copy_nonoverlapping(src, dst, len);
    }
}

impl HandlerImpl {
    fn pump(&mut self) {
        loop {
            let responses = self.forward_responses();
            let requests = self.forward_requests();
            self.forward_events();
            if !responses && !requests {
                break;
            }
        }
    }

    fn forward_requests(&mut self) -> bool {
        let mut moved = false;
        loop {
            // SAFETY: this protection domain produces the driver request queue.
            if unsafe { gpu_queue_full_req(&self.driver) } {
                return moved;
            }
            let mut req = gpu_req_new(gpu_req_code_t::GPU_REQ_GET_DISPLAY_INFO, 0);
            // SAFETY: this protection domain consumes the client request queue.
            if unsafe { gpu_dequeue_req(&self.client, &mut req) } != 0 {
                return moved;
            }
            if is_blob(req.code) {
                self.reject_blob(req.id);
                moved = true;
                continue;
            }
            let mem_offset = if req.code == gpu_req_code_t::GPU_REQ_GET_DISPLAY_INFO {
                unsafe { req.payload.get_display_info.mem_offset }
            } else {
                0
            };
            self.notes[(req.id as usize) % NOTE_SLOTS] = Note {
                code: req.code,
                mem_offset,
                live: true,
            };
            // SAFETY: the full check above is the enqueue condition.
            if unsafe { gpu_enqueue_req(&self.driver, req) } != 0 {
                return moved;
            }
            moved = true;
            self.driver_ch.notify();
        }
    }

    fn reject_blob(&mut self, id: u32) {
        // SAFETY: one client waits for each response, so this queue has room.
        if unsafe {
            gpu_enqueue_resp(
                &self.client,
                gpu_resp_t {
                    status: gpu_resp_status_t::GPU_RESP_ERR_INVALID_PARAMETER,
                    id,
                },
            )
        } != 0
        {
            return;
        }
        self.client_ch.notify();
    }

    fn forward_responses(&mut self) -> bool {
        let mut moved = false;
        loop {
            // SAFETY: this protection domain produces the client response queue.
            if unsafe { gpu_queue_full_resp(&self.client) } {
                return moved;
            }
            let mut resp = gpu_resp_t {
                status: gpu_resp_status_t::GPU_RESP_OK,
                id: 0,
            };
            // SAFETY: this protection domain consumes the driver response queue.
            if unsafe { gpu_dequeue_resp(&self.driver, &mut resp) } != 0 {
                return moved;
            }
            let note = &mut self.notes[(resp.id as usize) % NOTE_SLOTS];
            if note.live
                && note.code == gpu_req_code_t::GPU_REQ_GET_DISPLAY_INFO
                && resp.status == gpu_resp_status_t::GPU_RESP_OK
            {
                copy_display(note.mem_offset);
            }
            note.live = false;
            // SAFETY: the full check above is the enqueue condition.
            if unsafe { gpu_enqueue_resp(&self.client, resp) } != 0 {
                return moved;
            }
            moved = true;
            self.client_ch.notify();
        }
    }

    fn forward_events(&mut self) {
        // SAFETY: the template maps both event words.
        unsafe {
            let driver = GPU_DRIVER_EVENTS_VADDR as *mut gpu_events_t;
            let client = GPU_CLIENT_EVENTS_VADDR as *mut gpu_events_t;
            if gpu_events_check_display_info(driver) {
                gpu_events_set_display_info(client);
                gpu_events_clear_display_info(driver);
            }
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(self.driver_ch) || channels.contains(self.client_ch) {
            self.pump();
        }
        Ok(())
    }
}
