#![no_std]
#![no_main]

use embedded_hal_nb::serial::{Read, Write};
use sel4_driver_interfaces::HandleInterrupt;
use sel4_microkit::{
    memory_region_symbol, protection_domain, Channel, ChannelSet, Handler, Infallible,
};
use sel4_pl011_driver::Driver as Pl011Driver;

use lerux_logging::{debug, log};
use lerux_sddf::{
    serial_cancel_consumer_signal, serial_dequeue, serial_driver_config_t, serial_enqueue,
    serial_handle_from_connection,
    serial_image::{self, DRIVER_IRQ_CHANNEL},
    serial_queue_free, serial_queue_handle_t, serial_request_consumer_signal,
    serial_require_consumer_signal, SDDF_SERIAL_MAGIC,
};

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));

struct HandlerImpl {
    uart: Pl011Driver,
    /// `serial_driver_config_t` has no interrupt field. The system template's
    /// `<irq id>` is [`DRIVER_IRQ_CHANNEL`].
    irq: Channel,
    tx: serial_queue_handle_t,
    tx_ch: Channel,
    rx: serial_queue_handle_t,
    rx_ch: Channel,
    rx_enabled: bool,
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug logger");
    // SAFETY: the build script wrote a `serial_driver_config_t`.
    let config =
        unsafe { serial_image::serial_config_from_bytes::<serial_driver_config_t>(CONFIG) };
    assert_eq!(config.magic, SDDF_SERIAL_MAGIC);
    // SAFETY: this protection domain is the only one that maps the serial device.
    let uart =
        unsafe { Pl011Driver::new(memory_region_symbol!(serial_register_block: *mut ()).as_ptr()) };
    // SAFETY: the template maps these queue and data regions into this domain.
    let tx = unsafe { serial_handle_from_connection(&config.tx) };
    let rx = unsafe { serial_handle_from_connection(&config.rx) };
    log::info!("serial_driver: pl011");
    HandlerImpl {
        uart,
        irq: Channel::new(usize::from(DRIVER_IRQ_CHANNEL)),
        tx,
        tx_ch: Channel::new(usize::from(config.tx.id)),
        rx,
        rx_ch: Channel::new(usize::from(config.rx.id)),
        rx_enabled: config.rx_enabled,
    }
}

impl HandlerImpl {
    /// `sel4_pl011_driver` writes the data register only after the transmit
    /// FIFO has room, so this path does not arm the transmit interrupt.
    fn drain_tx(&mut self) {
        let mut transferred = false;
        loop {
            let mut byte = 0;
            // SAFETY: `tx` points at the mapped driver transmit queue.
            if unsafe { serial_dequeue(&self.tx, &mut byte) } != 0 {
                break;
            }
            self.uart
                .write(byte)
                .expect("pl011 transmit accepts the byte");
            transferred = true;
        }
        // SAFETY: `tx` points at the mapped driver transmit queue.
        if transferred && unsafe { serial_require_consumer_signal(&self.tx) } {
            unsafe { serial_cancel_consumer_signal(&self.tx) };
            self.tx_ch.notify();
        }
    }

    fn drain_rx(&mut self) {
        if !self.rx_enabled {
            return;
        }
        let mut enqueued = false;
        let mut blocked = false;
        loop {
            // SAFETY: `rx` points at the mapped driver receive queue. This
            // domain is the producer.
            if unsafe { serial_queue_free(&self.rx) } == 0 {
                unsafe { serial_request_consumer_signal(&self.rx) };
                blocked = true;
                break;
            }
            let byte = match self.uart.read() {
                Ok(byte) => byte,
                Err(_) => break,
            };
            // SAFETY: the free-space check above left room for one byte.
            let status = unsafe { serial_enqueue(&self.rx, byte) };
            assert_eq!(status, 0, "receive queue had room");
            enqueued = true;
        }
        if enqueued {
            self.rx_ch.notify();
        }
        if enqueued && !blocked {
            // SAFETY: `rx` points at the mapped driver receive queue.
            unsafe { serial_cancel_consumer_signal(&self.rx) };
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(self.irq) {
            self.drain_rx();
            self.drain_tx();
            self.uart.handle_interrupt();
            self.irq.irq_ack().expect("ack the serial interrupt");
        }
        if channels.contains(self.tx_ch) {
            self.drain_tx();
        }
        if channels.contains(self.rx_ch) {
            self.drain_rx();
        }
        Ok(())
    }
}
