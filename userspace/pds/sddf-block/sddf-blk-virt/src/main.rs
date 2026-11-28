#![no_std]
#![no_main]

use core::{cell::UnsafeCell, mem::MaybeUninit};

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_logging::{debug, log};
use lerux_sddf::{
    blk_dequeue_req, blk_dequeue_resp, blk_enqueue_req, blk_enqueue_resp,
    blk_image::{self, BLK_CLIENT_PARTITION, BLK_VIRT_NUM_CLIENTS},
    blk_queue_full_req, blk_queue_full_resp, blk_queue_handle_t, blk_queue_init, blk_req_code_t,
    blk_resp_status_t, blk_virt_config_t, SDDF_BLK_MAGIC,
};

const CONFIG_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));

struct ConfigCell(UnsafeCell<MaybeUninit<blk_virt_config_t>>);

// SAFETY: one kernel thread. `init` writes the cell, then `notified` only reads it.
unsafe impl Sync for ConfigCell {}

static CONFIG: ConfigCell = ConfigCell(UnsafeCell::new(MaybeUninit::uninit()));

struct HandlerImpl {
    driver: blk_queue_handle_t,
    client: blk_queue_handle_t,
    driver_ch: Channel,
    client_ch: Channel,
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    // SAFETY: the build script wrote a `blk_virt_config_t`, and `CONFIG` is aligned for it.
    let config = unsafe {
        blk_image::blk_config_write_bytes(CONFIG_BYTES, (*CONFIG.0.get()).as_mut_ptr());
        (*CONFIG.0.get()).assume_init_ref()
    };
    assert_eq!(config.magic, SDDF_BLK_MAGIC);
    assert_eq!(config.num_clients, BLK_VIRT_NUM_CLIENTS);
    assert_eq!(config.clients[0].partition, BLK_CLIENT_PARTITION);
    let driver = queue_from(&config.driver.conn);
    let client = queue_from(&config.clients[0].conn);
    let driver_ch = Channel::new(usize::from(config.driver.conn.id));
    let client_ch = Channel::new(usize::from(config.clients[0].conn.id));
    log::info!("blk_virt: ready");
    HandlerImpl {
        driver,
        client,
        driver_ch,
        client_ch,
    }
}

fn queue_from(conn: &lerux_sddf::blk_connection_resource_t) -> blk_queue_handle_t {
    let mut handle = blk_queue_handle_t {
        req_queue: core::ptr::null_mut(),
        resp_queue: core::ptr::null_mut(),
        capacity: 0,
    };
    // SAFETY: the template maps these queues into this protection domain.
    unsafe {
        blk_queue_init(
            &mut handle,
            conn.req_queue.vaddr.cast(),
            conn.resp_queue.vaddr.cast(),
            u32::from(conn.num_buffers),
        );
    }
    handle
}

impl HandlerImpl {
    fn pump(&mut self) {
        loop {
            let responses = self.forward_responses();
            let requests = self.forward_requests();
            if !responses && !requests {
                break;
            }
        }
    }

    fn forward_requests(&mut self) -> bool {
        let mut moved = false;
        loop {
            // SAFETY: this protection domain is the producer on the driver request queue.
            if unsafe { blk_queue_full_req(&self.driver) } {
                return moved;
            }
            let mut code = blk_req_code_t::BLK_REQ_READ;
            let mut offset = 0u64;
            let mut block = 0u64;
            let mut count = 0u16;
            let mut id = 0u32;
            // SAFETY: this protection domain is the consumer on the client request queue.
            if unsafe {
                blk_dequeue_req(
                    &self.client,
                    &mut code,
                    &mut offset,
                    &mut block,
                    &mut count,
                    &mut id,
                )
            } != 0
            {
                return moved;
            }
            // SAFETY: the full check above is the enqueue condition. One producer owns this queue.
            if unsafe { blk_enqueue_req(&self.driver, code, offset, block, count, id) } != 0 {
                return moved;
            }
            moved = true;
            self.driver_ch.notify();
        }
    }

    fn forward_responses(&mut self) -> bool {
        let mut moved = false;
        loop {
            // SAFETY: this protection domain is the producer on the client response queue.
            if unsafe { blk_queue_full_resp(&self.client) } {
                return moved;
            }
            let mut status = blk_resp_status_t::BLK_RESP_OK;
            let mut success = 0u16;
            let mut id = 0u32;
            // SAFETY: this protection domain is the consumer on the driver response queue.
            if unsafe { blk_dequeue_resp(&self.driver, &mut status, &mut success, &mut id) } != 0 {
                return moved;
            }
            // SAFETY: the full check above is the enqueue condition. One producer owns this queue.
            if unsafe { blk_enqueue_resp(&self.client, status, success, id) } != 0 {
                return moved;
            }
            moved = true;
            self.client_ch.notify();
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
