//! Copies one receive frame from the virtualiser data region into the client
//! data region. The client does not map the device queue.
//!
//! This protection domain fills the client free queue once, during init. After
//! that only the client produces that free queue.

#![no_std]
#![no_main]

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_logging::{debug, log};
use lerux_sddf::{
    net_buff_desc_t, net_buffers_init, net_cancel_signal_active, net_cancel_signal_free,
    net_connection_resource_t, net_copy_config_t, net_dequeue_active, net_dequeue_free,
    net_enqueue_active, net_enqueue_free, net_image, net_queue_empty_active, net_queue_empty_free,
    net_queue_handle_t, net_queue_init, net_request_signal_active, net_require_signal_active,
    net_require_signal_free, NET_BUFFER_SIZE, SDDF_NET_MAGIC,
};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));

struct HandlerImpl {
    virt: net_queue_handle_t,
    client: net_queue_handle_t,
    virt_data: *mut u8,
    client_data: *mut u8,
    virt_ch: Channel,
    client_ch: Channel,
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    // SAFETY: the build script wrote a `net_copy_config_t`.
    let config = unsafe { net_image::net_config_from_bytes::<net_copy_config_t>(CONFIG) };
    assert_eq!(config.magic, SDDF_NET_MAGIC);
    assert!(
        !config.rx_data[0].vaddr.is_null(),
        "packet comes from the one receive region"
    );
    let client = queue_from(&config.client);
    // SAFETY: this protection domain produces the client free queue, and it is empty.
    // The client starts later and does not fill this queue again.
    unsafe { net_buffers_init(&client, 0) };
    log::info!("net_copy: ready");
    HandlerImpl {
        virt: queue_from(&config.rx),
        client,
        virt_data: config.rx_data[0].vaddr,
        client_data: config.client_data.vaddr,
        virt_ch: Channel::new(usize::from(config.rx.id)),
        client_ch: Channel::new(usize::from(config.client.id)),
    }
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

fn offset_ok(offset: u64, capacity: u32) -> bool {
    let buf = u64::from(NET_BUFFER_SIZE);
    offset.is_multiple_of(buf) && offset < buf * u64::from(capacity)
}

impl HandlerImpl {
    fn rx_return(&mut self) {
        let mut client_enqueued = false;
        let mut virt_enqueued = false;
        let mut reprocess = true;
        while reprocess {
            while unsafe { !net_queue_empty_active(&self.virt) } {
                if unsafe { !net_queue_empty_free(&self.client) } {
                    let mut client_buffer = net_buff_desc_t::new(0, 0, 0);
                    // SAFETY: this protection domain consumes the client free queue.
                    let err = unsafe { net_dequeue_free(&self.client, &mut client_buffer) };
                    assert_eq!(err, 0, "client free buffer is present");
                    assert!(
                        offset_ok(client_buffer.io_or_offset, self.client.capacity),
                        "offset fits"
                    );
                    let mut virt_buffer = net_buff_desc_t::new(0, 0, 0);
                    // SAFETY: this protection domain consumes the virtualiser active queue.
                    let err = unsafe { net_dequeue_active(&self.virt, &mut virt_buffer) };
                    assert_eq!(err, 0, "virt buffer is present");
                    assert_eq!(
                        virt_buffer.oid(),
                        0,
                        "packet comes from the one receive region"
                    );
                    assert!(
                        u32::from(virt_buffer.len) <= NET_BUFFER_SIZE
                            && offset_ok(virt_buffer.io_or_offset, self.virt.capacity),
                        "offset fits"
                    );
                    let len = usize::from(virt_buffer.len);
                    // SAFETY: both offsets were checked, and both data regions are mapped here.
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            self.virt_data.add(
                                usize::try_from(virt_buffer.io_or_offset).expect("offset fits"),
                            ),
                            self.client_data.add(
                                usize::try_from(client_buffer.io_or_offset).expect("offset fits"),
                            ),
                            len,
                        );
                    }
                    client_buffer.len = virt_buffer.len;
                    // SAFETY: this protection domain produces the client active queue.
                    let err = unsafe { net_enqueue_active(&self.client, client_buffer) };
                    assert_eq!(err, 0, "client active queue accepts the buffer");
                    // SAFETY: this protection domain produces the virtualiser free queue.
                    // The length stays so a later owner can reuse the descriptor.
                    let err = unsafe { net_enqueue_free(&self.virt, virt_buffer) };
                    assert_eq!(err, 0, "virt free queue accepts the buffer");
                    client_enqueued = true;
                } else {
                    let mut virt_buffer = net_buff_desc_t::new(0, 0, 0);
                    // SAFETY: this protection domain consumes the virtualiser active queue.
                    let err = unsafe { net_dequeue_active(&self.virt, &mut virt_buffer) };
                    assert_eq!(err, 0, "virt buffer is present");
                    // SAFETY: there is no client buffer, so the frame goes back unused.
                    let err = unsafe { net_enqueue_free(&self.virt, virt_buffer) };
                    assert_eq!(err, 0, "virt free queue accepts the buffer");
                }
                virt_enqueued = true;
            }
            // SAFETY: this protection domain consumes the virtualiser active queue.
            unsafe { net_request_signal_active(&self.virt) };
            reprocess = false;
            if unsafe { !net_queue_empty_active(&self.virt) } {
                unsafe { net_cancel_signal_active(&self.virt) };
                reprocess = true;
            }
        }
        if client_enqueued && unsafe { net_require_signal_active(&self.client) } {
            unsafe { net_cancel_signal_active(&self.client) };
            self.client_ch.notify();
        }
        if virt_enqueued && unsafe { net_require_signal_free(&self.virt) } {
            unsafe { net_cancel_signal_free(&self.virt) };
            self.virt_ch.notify();
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, _channels: ChannelSet) -> Result<(), Self::Error> {
        self.rx_return();
        Ok(())
    }
}
