//! 82574 driver for the one-client network image.
//!
//! Packet bytes move between the shared data regions and a private DMA window.
//! A shared descriptor holds an offset. `io_addr` stays 0.

#![no_std]
#![no_main]

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_logging::{debug, log};
use lerux_machine::e1000e::{self, e1000e_map};
use lerux_sddf::{
    net_buff_desc_t, net_cancel_signal_active, net_cancel_signal_free, net_connection_resource_t,
    net_dequeue_active, net_dequeue_free, net_driver_config_t, net_enqueue_active,
    net_enqueue_free,
    net_image::{
        self, CLIENT_MAC, DRIVER_IRQ_CHANNEL, DRIVER_RX_CHANNEL, DRIVER_TX_CHANNEL, NET_DATA_SIZE,
        NET_RX_DATA_VADDR, NET_TX_DATA_VADDR,
    },
    net_queue_empty_active, net_queue_empty_free, net_queue_handle_t, net_queue_init,
    net_request_signal_active, net_request_signal_free, net_require_signal_active,
    net_require_signal_free, NET_BUFFER_SIZE, SDDF_NET_MAGIC,
};

mod engine;

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));

const _: () = assert!(engine::BUF_SIZE == NET_BUFFER_SIZE as usize);

struct HandlerImpl {
    engine: engine::Engine,
    rx: net_queue_handle_t,
    tx: net_queue_handle_t,
    irq: Channel,
    rx_ch: Channel,
    tx_ch: Channel,
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    // SAFETY: the build script wrote a zero-filled `net_driver_config_t`.
    let config = unsafe { net_image::net_config_from_bytes::<net_driver_config_t>(CONFIG) };
    assert_eq!(config.magic, SDDF_NET_MAGIC);
    assert_eq!(config.virt_rx.id, DRIVER_RX_CHANNEL);
    assert_eq!(config.virt_tx.id, DRIVER_TX_CHANNEL);

    let engine = engine::Engine::open(e1000e_map(), CLIENT_MAC.addr).expect("e1000e engine");
    engine.ack();
    let irq = Channel::new(usize::from(DRIVER_IRQ_CHANNEL));
    irq.irq_ack().expect("irq ack");

    log::info!("{}", e1000e::READY_LINE);
    HandlerImpl {
        engine,
        rx: queue_from(&config.virt_rx),
        tx: queue_from(&config.virt_tx),
        irq,
        rx_ch: Channel::new(usize::from(config.virt_rx.id)),
        tx_ch: Channel::new(usize::from(config.virt_tx.id)),
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

fn region_ptr(base: u64, offset: u64, len: usize) -> Option<*mut u8> {
    if len == 0 || len > usize::try_from(NET_BUFFER_SIZE).expect("buffer size") {
        return None;
    }
    if !offset.is_multiple_of(u64::from(NET_BUFFER_SIZE)) {
        return None;
    }
    let len_u64 = u64::try_from(len).expect("length fits");
    let end = offset.checked_add(len_u64)?;
    if end > NET_DATA_SIZE {
        return None;
    }
    let base = usize::try_from(base).expect("region address");
    let offset = usize::try_from(offset).expect("offset");
    Some(base.wrapping_add(offset) as *mut u8)
}

impl HandlerImpl {
    fn serve(&mut self) {
        self.transmit();
        self.receive();
    }

    fn transmit(&mut self) {
        let mut returned = false;
        let mut reprocess = true;
        while reprocess {
            while unsafe { !net_queue_empty_active(&self.tx) } {
                let mut desc = net_buff_desc_t::new(0, 0, 0);
                // SAFETY: this protection domain consumes the driver transmit active queue.
                if unsafe { net_dequeue_active(&self.tx, &mut desc) } != 0 {
                    break;
                }
                let len = usize::from(desc.len);
                if let Some(src) = region_ptr(NET_TX_DATA_VADDR, desc.io_or_offset, len) {
                    // SAFETY: `region_ptr` bounded `src` by `len` inside the mapped transmit region.
                    let frame = unsafe { core::slice::from_raw_parts(src, len) };
                    self.engine.submit_tx(frame);
                }
                let back = net_buff_desc_t::new(desc.io_or_offset, 0, 0);
                // SAFETY: this protection domain produces the driver transmit free queue.
                let _ = unsafe { net_enqueue_free(&self.tx, back) };
                returned = true;
            }
            // SAFETY: this protection domain consumes the driver transmit active queue.
            unsafe { net_request_signal_active(&self.tx) };
            reprocess = false;
            if unsafe { !net_queue_empty_active(&self.tx) } {
                unsafe { net_cancel_signal_active(&self.tx) };
                reprocess = true;
            }
        }
        if returned && unsafe { net_require_signal_free(&self.tx) } {
            unsafe { net_cancel_signal_free(&self.tx) };
            self.tx_ch.notify();
        }
    }

    fn receive(&mut self) {
        let buf = usize::try_from(NET_BUFFER_SIZE).expect("buffer size");
        loop {
            if unsafe { net_queue_empty_free(&self.rx) } {
                // SAFETY: this protection domain consumes the driver receive free queue.
                unsafe { net_request_signal_free(&self.rx) };
                if unsafe { net_queue_empty_free(&self.rx) } {
                    return;
                }
                unsafe { net_cancel_signal_free(&self.rx) };
                continue;
            }
            let mut desc = net_buff_desc_t::new(0, 0, 0);
            // SAFETY: the empty check is the dequeue condition.
            if unsafe { net_dequeue_free(&self.rx, &mut desc) } != 0 {
                return;
            }
            let published = region_ptr(NET_RX_DATA_VADDR, desc.io_or_offset, buf).and_then(|dst| {
                // SAFETY: `region_ptr` bounded `dst` by one shared buffer.
                let dst = unsafe { core::slice::from_raw_parts_mut(dst, buf) };
                self.engine.copy_rx(dst)
            });
            let Some(len) = published else {
                self.restore_free(desc);
                return;
            };
            let out = net_buff_desc_t::new(desc.io_or_offset, len, 0);
            // SAFETY: this protection domain produces the driver receive active queue.
            if unsafe { net_enqueue_active(&self.rx, out) } != 0 {
                self.restore_free(desc);
                return;
            }
            if unsafe { net_require_signal_active(&self.rx) } {
                unsafe { net_cancel_signal_active(&self.rx) };
                self.rx_ch.notify();
            }
        }
    }

    fn restore_free(&mut self, desc: net_buff_desc_t) {
        let back = net_buff_desc_t::new(desc.io_or_offset, 0, 0);
        // SAFETY: this protection domain produces the driver receive free queue
        // on the failure path. The other producer is `net_virt_rx`. One domain
        // runs at a time.
        let _ = unsafe { net_enqueue_free(&self.rx, back) };
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        let irq = channels.contains(self.irq);
        if irq {
            self.engine.ack();
            self.irq.irq_ack().expect("irq ack");
        }
        if irq || channels.contains(self.rx_ch) || channels.contains(self.tx_ch) {
            self.serve();
        }
        Ok(())
    }
}
