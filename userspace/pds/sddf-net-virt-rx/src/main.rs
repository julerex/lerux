//! Receive virtualiser for the one-client network image.
//!
//! The configuration value is larger than the protection-domain stack, so it
//! lives in static storage. A frame whose destination is not the client
//! address or the broadcast address goes back to the driver. That includes
//! IPv6 multicast. This image has one client, so a broadcast frame is queued
//! once.

#![no_std]
#![no_main]

use core::{cell::UnsafeCell, mem::MaybeUninit};

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_logging::{debug, log};
use lerux_sddf::{
    net_buff_desc_t, net_buffers_init, net_cancel_signal_active, net_cancel_signal_free,
    net_connection_resource_t, net_dequeue_active, net_dequeue_free, net_enqueue_active,
    net_enqueue_free, net_image, net_image::NET_QUEUE_CAPACITY, net_queue_empty_active,
    net_queue_empty_free, net_queue_handle_t, net_queue_init, net_request_signal_active,
    net_request_signal_free, net_require_signal_active, net_require_signal_free,
    net_virt_rx_config_t, MAC802_BYTES, NET_BUFFER_SIZE, SDDF_NET_MAGIC,
};

const CONFIG_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));
const BROADCAST: [u8; MAC802_BYTES] = [0xff; MAC802_BYTES];

struct ConfigCell(UnsafeCell<MaybeUninit<net_virt_rx_config_t>>);

// SAFETY: one kernel thread. `init` writes the cell, then `notified` only reads it.
unsafe impl Sync for ConfigCell {}

static CONFIG: ConfigCell = ConfigCell(UnsafeCell::new(MaybeUninit::uninit()));

fn config() -> &'static net_virt_rx_config_t {
    // SAFETY: `init` finished writing the cell before this runs again.
    unsafe { (*CONFIG.0.get()).assume_init_ref() }
}

struct HandlerImpl {
    driver: net_queue_handle_t,
    copier: net_queue_handle_t,
    driver_ch: Channel,
    copier_ch: Channel,
    /// One reference count per receive buffer. Microkit zeroes the page, and
    /// this protection domain is the only one that maps it.
    refs: *mut u8,
    notify_driver: bool,
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    // SAFETY: the build script wrote a `net_virt_rx_config_t`, and `CONFIG` is aligned for it.
    unsafe {
        net_image::net_config_write_bytes(CONFIG_BYTES, (*CONFIG.0.get()).as_mut_ptr());
    }
    assert_eq!(config().magic, SDDF_NET_MAGIC);
    assert_eq!(config().num_clients, 1, "one client");
    assert!(!config().buffer_metadata.vaddr.is_null(), "offset fits");
    let mut handler = HandlerImpl {
        driver: queue_from(&config().driver),
        copier: queue_from(&config().clients[0].conn),
        driver_ch: Channel::new(usize::from(config().driver.id)),
        copier_ch: Channel::new(usize::from(config().clients[0].conn.id)),
        refs: config().buffer_metadata.vaddr,
        notify_driver: false,
    };
    log::info!("net_virt_rx: ready");
    // The copier has not returned a buffer yet. This only arms the signal.
    handler.rx_provide();
    // SAFETY: this protection domain produces the driver free queue, and it is empty.
    unsafe { net_buffers_init(&handler.driver, io_addr()) };
    if unsafe { net_require_signal_free(&handler.driver) } {
        unsafe { net_cancel_signal_free(&handler.driver) };
        handler.driver_ch.notify();
    }
    handler
}

fn io_addr() -> u64 {
    u64::try_from(config().data.io_addr).expect("offset fits")
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

enum Dest {
    Client,
    Broadcast,
    Drop,
}

impl HandlerImpl {
    fn ref_cell(&self, offset: u64) -> *mut u8 {
        let buf = u64::from(NET_BUFFER_SIZE);
        assert!(offset.is_multiple_of(buf), "offset fits");
        let index = offset / buf;
        assert!(index < u64::from(NET_QUEUE_CAPACITY), "offset fits");
        let index = usize::try_from(index).expect("offset fits");
        // SAFETY: `index` is below the queue capacity, and the metadata page is
        // one mapped page that only this protection domain writes.
        unsafe { self.refs.add(index) }
    }

    fn destination(&self, buffer: &net_buff_desc_t) -> Dest {
        if u32::from(buffer.len) > NET_BUFFER_SIZE
            || buffer.len < u16::try_from(MAC802_BYTES).expect("address length")
        {
            return Dest::Drop;
        }
        let vaddr = config()
            .data
            .region
            .vaddr
            .wrapping_add(usize::try_from(buffer.io_or_offset).expect("offset fits"));
        // SAFETY: the frame is at least the address length and sits in the mapped data region.
        let dest = unsafe { core::slice::from_raw_parts(vaddr, MAC802_BYTES) };
        let client = &config().clients[0];
        let macs = usize::from(client.num_macs);
        assert!(macs <= client.mac_addrs.len(), "one client");
        for mac in &client.mac_addrs[..macs] {
            if dest == mac.addr {
                return Dest::Client;
            }
        }
        if dest == BROADCAST {
            Dest::Broadcast
        } else {
            Dest::Drop
        }
    }

    fn rx_return(&mut self) {
        let mut notify_copier = false;
        let mut reprocess = true;
        while reprocess {
            while unsafe { !net_queue_empty_active(&self.driver) } {
                let mut buffer = net_buff_desc_t::new(0, 0, 0);
                // SAFETY: this protection domain consumes the driver receive active queue.
                let err = unsafe { net_dequeue_active(&self.driver, &mut buffer) };
                assert_eq!(err, 0, "active driver buffer is present");
                let io_addr = io_addr();
                assert!(buffer.io_or_offset >= io_addr, "offset fits");
                buffer.io_or_offset -= io_addr;
                let buf = u64::from(NET_BUFFER_SIZE);
                assert!(
                    buffer.io_or_offset.is_multiple_of(buf)
                        && buffer.io_or_offset < buf * u64::from(NET_QUEUE_CAPACITY),
                    "offset fits"
                );
                let dest = self.destination(&buffer);
                match dest {
                    Dest::Client | Dest::Broadcast => {
                        let cell = self.ref_cell(buffer.io_or_offset);
                        // SAFETY: `cell` is one reference count in the metadata page.
                        assert_eq!(unsafe { *cell }, 0, "offset fits");
                        let count = if matches!(dest, Dest::Broadcast) {
                            config().num_clients
                        } else {
                            1
                        };
                        // SAFETY: same cell as the load above. This domain is the only writer.
                        unsafe { *cell = count };
                        // SAFETY: this protection domain produces the copier active queue.
                        let err = unsafe { net_enqueue_active(&self.copier, buffer) };
                        assert_eq!(err, 0, "copier queue accepts the buffer");
                        notify_copier = true;
                    }
                    Dest::Drop => {
                        buffer.io_or_offset += io_addr;
                        // SAFETY: this protection domain produces the driver free queue.
                        let err = unsafe { net_enqueue_free(&self.driver, buffer) };
                        assert_eq!(err, 0, "driver free queue accepts the buffer");
                        self.notify_driver = true;
                    }
                }
            }
            // SAFETY: this protection domain consumes the driver receive active queue.
            unsafe { net_request_signal_active(&self.driver) };
            reprocess = false;
            if unsafe { !net_queue_empty_active(&self.driver) } {
                unsafe { net_cancel_signal_active(&self.driver) };
                reprocess = true;
            }
        }
        if notify_copier && unsafe { net_require_signal_active(&self.copier) } {
            unsafe { net_cancel_signal_active(&self.copier) };
            self.copier_ch.notify();
        }
    }

    fn rx_provide(&mut self) {
        let mut reprocess = true;
        while reprocess {
            while unsafe { !net_queue_empty_free(&self.copier) } {
                let mut buffer = net_buff_desc_t::new(0, 0, 0);
                // SAFETY: this protection domain consumes the copier free queue.
                let err = unsafe { net_dequeue_free(&self.copier, &mut buffer) };
                assert_eq!(err, 0, "returned copier buffer is present");
                let buf = u64::from(NET_BUFFER_SIZE);
                assert!(
                    buffer.io_or_offset.is_multiple_of(buf)
                        && buffer.io_or_offset < buf * u64::from(self.driver.capacity),
                    "offset fits"
                );
                let cell = self.ref_cell(buffer.io_or_offset);
                // SAFETY: `cell` is one reference count in the metadata page.
                let refs = unsafe { &mut *cell };
                assert_ne!(*refs, 0, "offset fits");
                *refs -= 1;
                if *refs != 0 {
                    continue;
                }
                buffer.io_or_offset += io_addr();
                // SAFETY: this protection domain produces the driver free queue.
                let err = unsafe { net_enqueue_free(&self.driver, buffer) };
                assert_eq!(err, 0, "driver free queue accepts the buffer");
                self.notify_driver = true;
            }
            // SAFETY: this protection domain consumes the copier free queue.
            unsafe { net_request_signal_free(&self.copier) };
            reprocess = false;
            if unsafe { !net_queue_empty_free(&self.copier) } {
                unsafe { net_cancel_signal_free(&self.copier) };
                reprocess = true;
            }
        }
        if self.notify_driver && unsafe { net_require_signal_free(&self.driver) } {
            unsafe { net_cancel_signal_free(&self.driver) };
            self.driver_ch.notify();
            self.notify_driver = false;
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, _channels: ChannelSet) -> Result<(), Self::Error> {
        self.rx_return();
        self.rx_provide();
        Ok(())
    }
}
