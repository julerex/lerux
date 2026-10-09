//! Transmit virtualiser for the one-client network image.
//!
//! The configuration value is larger than the protection-domain stack, so it
//! lives in static storage. This image has one client and one data region.

#![no_std]
#![no_main]

use core::{cell::UnsafeCell, mem::MaybeUninit};

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_logging::{debug, log};
use lerux_sddf::{
    net_buff_desc_t, net_cancel_signal_active, net_cancel_signal_free, net_connection_resource_t,
    net_dequeue_active, net_dequeue_free, net_enqueue_active, net_enqueue_free, net_image,
    net_queue_empty_active, net_queue_empty_free, net_queue_full_active, net_queue_handle_t,
    net_queue_init, net_request_signal_active, net_request_signal_free, net_require_signal_active,
    net_require_signal_free, net_virt_tx_config_t, NET_BUFFER_SIZE, SDDF_NET_MAGIC,
};

const CONFIG_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));

struct ConfigCell(UnsafeCell<MaybeUninit<net_virt_tx_config_t>>);

// SAFETY: one kernel thread. `init` writes the cell, then `notified` only reads it.
unsafe impl Sync for ConfigCell {}

static CONFIG: ConfigCell = ConfigCell(UnsafeCell::new(MaybeUninit::uninit()));

fn config() -> &'static net_virt_tx_config_t {
    // SAFETY: `init` finished writing the cell before this runs again.
    unsafe { (*CONFIG.0.get()).assume_init_ref() }
}

struct HandlerImpl {
    driver: net_queue_handle_t,
    client: net_queue_handle_t,
    driver_ch: Channel,
    client_ch: Channel,
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    // SAFETY: the build script wrote a `net_virt_tx_config_t`, and `CONFIG` is aligned for it.
    let config = unsafe {
        net_image::net_config_write_bytes(CONFIG_BYTES, (*CONFIG.0.get()).as_mut_ptr());
        (*CONFIG.0.get()).assume_init_ref()
    };
    assert_eq!(config.magic, SDDF_NET_MAGIC);
    assert_eq!(config.num_clients, 1, "one transmit client");
    assert_eq!(config.clients[0].num_regions, 1, "one transmit region");
    let mut handler = HandlerImpl {
        driver: queue_from(&config.driver),
        client: queue_from(&config.clients[0].conn),
        driver_ch: Channel::new(usize::from(config.driver.id)),
        client_ch: Channel::new(usize::from(config.clients[0].conn.id)),
    };
    log::info!("net_virt_tx: ready");
    handler.tx_provide();
    handler
}

fn queue_from(conn: &net_connection_resource_t) -> net_queue_handle_t {
    let mut handle = net_queue_handle_t {
        free: core::ptr::null_mut(),
        active: core::ptr::null_mut(),
        capacity: 0,
    };
    // SAFETY: the template maps these queues into this protection domain.
    unsafe {
        net_queue_init(
            &mut handle,
            conn.free_queue.vaddr.cast(),
            conn.active_queue.vaddr.cast(),
            u32::from(conn.num_buffers),
        );
    }
    handle
}

impl HandlerImpl {
    fn tx_provide(&mut self) {
        let mut enqueued = false;
        let mut notify_client = false;
        let mut reprocess = true;
        while reprocess {
            while unsafe { !net_queue_empty_active(&self.client) } {
                if unsafe { net_queue_full_active(&self.driver) } {
                    break;
                }
                let mut buffer = net_buff_desc_t::new(0, 0, 0);
                // SAFETY: this protection domain consumes the client transmit active queue.
                let err = unsafe { net_dequeue_active(&self.client, &mut buffer) };
                assert_eq!(err, 0, "active client buffer is present");
                if !buffer_ok(&buffer) {
                    // SAFETY: this protection domain produces the client transmit free queue
                    // only for a buffer it just rejected. The client does not enqueue free
                    // after its own initial fill.
                    let err = unsafe { net_enqueue_free(&self.client, buffer) };
                    assert_eq!(err, 0, "rejected buffer returns to the client");
                    notify_client = true;
                    continue;
                }
                let region = &config().clients[0].regions[usize::from(buffer.oid())];
                buffer.io_or_offset += u64::try_from(region.data.io_addr).expect("offset fits");
                // SAFETY: the full check above is the enqueue condition.
                let err = unsafe { net_enqueue_active(&self.driver, buffer) };
                assert_eq!(err, 0, "driver transmit queue accepts the buffer");
                enqueued = true;
            }
            // SAFETY: this protection domain consumes the client transmit active queue.
            unsafe { net_request_signal_active(&self.client) };
            reprocess = false;
            if unsafe {
                !net_queue_empty_active(&self.client) && !net_queue_full_active(&self.driver)
            } {
                unsafe { net_cancel_signal_active(&self.client) };
                reprocess = true;
            }
        }
        if notify_client && unsafe { net_require_signal_free(&self.client) } {
            unsafe { net_cancel_signal_free(&self.client) };
            self.client_ch.notify();
        }
        if enqueued && unsafe { net_require_signal_active(&self.driver) } {
            unsafe { net_cancel_signal_active(&self.driver) };
            self.driver_ch.notify();
        }
    }

    fn tx_return(&mut self) {
        let mut notify_client = false;
        let mut reprocess = true;
        while reprocess {
            while unsafe { !net_queue_empty_free(&self.driver) } {
                let mut buffer = net_buff_desc_t::new(0, 0, 0);
                // SAFETY: this protection domain consumes the driver transmit free queue.
                let err = unsafe { net_dequeue_free(&self.driver, &mut buffer) };
                assert_eq!(err, 0, "returned driver buffer is present");
                let oid = extract_offset(&mut buffer.io_or_offset);
                buffer.set_oid(oid);
                // SAFETY: this protection domain produces the client transmit free queue.
                let err = unsafe { net_enqueue_free(&self.client, buffer) };
                assert_eq!(err, 0, "client transmit free queue accepts the buffer");
                notify_client = true;
            }
            // SAFETY: this protection domain consumes the driver transmit free queue.
            unsafe { net_request_signal_free(&self.driver) };
            reprocess = false;
            if unsafe { !net_queue_empty_free(&self.driver) } {
                unsafe { net_cancel_signal_free(&self.driver) };
                reprocess = true;
            }
        }
        if notify_client && unsafe { net_require_signal_free(&self.client) } {
            unsafe { net_cancel_signal_free(&self.client) };
            self.client_ch.notify();
        }
    }
}

fn buffer_ok(buffer: &net_buff_desc_t) -> bool {
    let client = &config().clients[0];
    if buffer.len == 0 || u32::from(buffer.len) > NET_BUFFER_SIZE {
        return false;
    }
    if usize::from(buffer.oid()) >= usize::from(client.num_regions) {
        return false;
    }
    let region = &client.regions[usize::from(buffer.oid())];
    buffer
        .io_or_offset
        .is_multiple_of(u64::from(NET_BUFFER_SIZE))
        && buffer.io_or_offset < u64::from(NET_BUFFER_SIZE) * u64::from(region.num_buffers)
}

fn extract_offset(offset: &mut u64) -> u8 {
    let region = &config().clients[0].regions[0];
    let start = u64::try_from(region.data.io_addr).expect("offset fits");
    let end = start + u64::from(region.num_buffers) * u64::from(NET_BUFFER_SIZE);
    assert!(
        *offset >= start && *offset < end,
        "returned buffer belongs to the client region"
    );
    *offset -= start;
    0
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, _channels: ChannelSet) -> Result<(), Self::Error> {
        self.tx_return();
        self.tx_provide();
        Ok(())
    }
}
