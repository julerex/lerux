//! Virtio-net driver for the one-client network image.
//!
//! Packet bytes are copied between the shared data regions and virtio-drivers
//! buffers. `net_driver_config_t` has no data-region physical address, and
//! `device_region_resource_t.io_addr` stays 0, so this driver cannot hand the
//! device a shared-region address.

#![no_std]
#![no_main]

extern crate alloc;

use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_logging::{debug, log};
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

mod mmio;

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));

struct HandlerImpl {
    dev: mmio::NetDev,
    rx: net_queue_handle_t,
    tx: net_queue_handle_t,
    irq: Channel,
    rx_ch: Channel,
    tx_ch: Channel,
}

#[protection_domain(heap_size = 256 * 1024)]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    // SAFETY: the build script wrote a `net_driver_config_t`.
    let config = unsafe { net_image::net_config_from_bytes::<net_driver_config_t>(CONFIG) };
    assert_eq!(config.magic, SDDF_NET_MAGIC);
    assert_eq!(config.virt_rx.id, DRIVER_RX_CHANNEL);
    assert_eq!(config.virt_tx.id, DRIVER_TX_CHANNEL);
    mmio::init_hal();
    let mut dev = mmio::create_virtio_net();
    assert_eq!(
        dev.mac_address(),
        CLIENT_MAC.addr,
        "device address is the client address"
    );
    let _ = dev.ack_interrupt();
    let irq = Channel::new(usize::from(DRIVER_IRQ_CHANNEL));
    irq.irq_ack().expect("irq ack");
    log::info!("net_driver: ready");
    HandlerImpl {
        dev,
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
    fn service(&mut self) {
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
                self.send_one(&desc);
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

    fn send_one(&mut self, desc: &net_buff_desc_t) {
        let len = usize::from(desc.len);
        let Some(src) = region_ptr(NET_TX_DATA_VADDR, desc.io_or_offset, len) else {
            return;
        };
        if !self.dev.can_send() {
            return;
        }
        let mut tx = self.dev.new_tx_buffer(len);
        // SAFETY: `src` is `len` bytes inside the mapped transmit data region.
        unsafe {
            core::ptr::copy_nonoverlapping(src, tx.packet_mut().as_mut_ptr(), len);
        }
        // QEMU completes the transmit queue during the notify store, so this
        // returns without waiting for the interrupt handler.
        let _ = self.dev.send(tx);
    }

    fn receive(&mut self) {
        loop {
            if !self.dev.can_recv() {
                return;
            }
            if unsafe { net_queue_empty_free(&self.rx) } {
                // SAFETY: this protection domain consumes the driver receive free queue.
                unsafe { net_request_signal_free(&self.rx) };
                if unsafe { net_queue_empty_free(&self.rx) } {
                    return;
                }
                unsafe { net_cancel_signal_free(&self.rx) };
                continue;
            }
            if !self.deliver_one() {
                return;
            }
        }
    }

    fn deliver_one(&mut self) -> bool {
        let mut desc = net_buff_desc_t::new(0, 0, 0);
        // SAFETY: the empty check is the dequeue condition. This domain consumes the free queue.
        if unsafe { net_dequeue_free(&self.rx, &mut desc) } != 0 {
            return false;
        }
        let Ok(rx) = self.dev.receive() else {
            let _ = unsafe { net_enqueue_free(&self.rx, desc) };
            return false;
        };
        let delivered = {
            let packet = rx.packet();
            let len = packet.len();
            if let Some(dst) = region_ptr(NET_RX_DATA_VADDR, desc.io_or_offset, len) {
                // SAFETY: `dst` is `len` bytes inside the mapped receive data region.
                unsafe {
                    core::ptr::copy_nonoverlapping(packet.as_ptr(), dst, len);
                }
                let out = net_buff_desc_t::new(
                    desc.io_or_offset,
                    u16::try_from(len).expect("packet length fits"),
                    0,
                );
                // SAFETY: this protection domain produces the driver receive active queue.
                let status = unsafe { net_enqueue_active(&self.rx, out) };
                status == 0
            } else {
                false
            }
        };
        if !delivered {
            let back = net_buff_desc_t::new(desc.io_or_offset, 0, 0);
            let _ = unsafe { net_enqueue_free(&self.rx, back) };
        } else if unsafe { net_require_signal_active(&self.rx) } {
            unsafe { net_cancel_signal_active(&self.rx) };
            self.rx_ch.notify();
        }
        // Recycle even when the shared queue rejects the packet. Holding the
        // virtio buffer would stall the receive ring.
        let _ = self.dev.recycle_rx_buffer(rx);
        true
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        let irq = channels.contains(self.irq);
        if irq {
            self.dev.ack_interrupt();
            self.irq.irq_ack().expect("irq ack");
        }
        if irq || channels.contains(self.rx_ch) || channels.contains(self.tx_ch) {
            self.service();
        }
        Ok(())
    }
}
