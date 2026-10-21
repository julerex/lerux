#![no_std]
#![no_main]

use embedded_hal_nb::serial::{Read, Write};
use sel4_driver_interfaces::HandleInterrupt;
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
use sel4_microkit::memory_region_symbol;
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

use lerux_logging::{debug, log};
use lerux_machine::{serial, Serial};
use lerux_sddf::{
    serial_cancel_consumer_signal, serial_dequeue, serial_driver_config_t, serial_enqueue,
    serial_handle_from_connection,
    serial_image::{self, DRIVER_IRQ_CHANNEL},
    serial_queue_free, serial_queue_handle_t, serial_request_consumer_signal,
    serial_require_consumer_signal, SDDF_SERIAL_MAGIC,
};

#[cfg(target_arch = "aarch64")]
use sel4_pl011_driver::Driver as Pl011Driver;

#[cfg(target_arch = "riscv64")]
mod ns16550_mmio;

#[cfg(target_arch = "x86_64")]
mod ns16550_port;

const CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config.bin"));

enum Uart {
    #[cfg(target_arch = "aarch64")]
    Pl011(Pl011Driver),
    #[cfg(target_arch = "riscv64")]
    Ns16550Mmio(ns16550_mmio::Driver),
    #[cfg(target_arch = "x86_64")]
    Ns16550Port(ns16550_port::Driver),
}

impl embedded_hal_nb::serial::ErrorType for Uart {
    type Error = core::convert::Infallible;
}

impl Read for Uart {
    fn read(&mut self) -> embedded_hal_nb::nb::Result<u8, Self::Error> {
        match self {
            #[cfg(target_arch = "aarch64")]
            Uart::Pl011(uart) => uart.read(),
            #[cfg(target_arch = "riscv64")]
            Uart::Ns16550Mmio(uart) => uart.read(),
            #[cfg(target_arch = "x86_64")]
            Uart::Ns16550Port(uart) => uart.read(),
        }
    }
}

impl Write for Uart {
    fn write(&mut self, byte: u8) -> embedded_hal_nb::nb::Result<(), Self::Error> {
        match self {
            #[cfg(target_arch = "aarch64")]
            Uart::Pl011(uart) => uart.write(byte),
            #[cfg(target_arch = "riscv64")]
            Uart::Ns16550Mmio(uart) => uart.write(byte),
            #[cfg(target_arch = "x86_64")]
            Uart::Ns16550Port(uart) => uart.write(byte),
        }
    }

    fn flush(&mut self) -> embedded_hal_nb::nb::Result<(), Self::Error> {
        match self {
            #[cfg(target_arch = "aarch64")]
            Uart::Pl011(uart) => uart.flush(),
            #[cfg(target_arch = "riscv64")]
            Uart::Ns16550Mmio(uart) => uart.flush(),
            #[cfg(target_arch = "x86_64")]
            Uart::Ns16550Port(uart) => uart.flush(),
        }
    }
}

impl HandleInterrupt for Uart {
    fn handle_interrupt(&mut self) {
        match self {
            #[cfg(target_arch = "aarch64")]
            Uart::Pl011(uart) => uart.handle_interrupt(),
            #[cfg(target_arch = "riscv64")]
            Uart::Ns16550Mmio(uart) => uart.handle_interrupt(),
            #[cfg(target_arch = "x86_64")]
            Uart::Ns16550Port(uart) => uart.handle_interrupt(),
        }
    }
}

fn open_uart() -> Uart {
    match serial() {
        Serial::Pl011 => {
            #[cfg(target_arch = "aarch64")]
            {
                // SAFETY: this protection domain is the only one that maps the serial device.
                let uart = unsafe {
                    Pl011Driver::new(memory_region_symbol!(serial_register_block: *mut ()).as_ptr())
                };
                log::info!("serial_driver: pl011");
                Uart::Pl011(uart)
            }
            #[cfg(not(target_arch = "aarch64"))]
            unreachable!("PL011 is aarch64 only")
        }
        Serial::Ns16550Mmio => {
            #[cfg(target_arch = "riscv64")]
            {
                // SAFETY: this protection domain is the only one that maps the serial device.
                let uart = ns16550_mmio::Driver::new(
                    memory_region_symbol!(serial_register_block: *mut u8).as_ptr(),
                );
                log::info!("serial_driver: ns16550");
                Uart::Ns16550Mmio(uart)
            }
            #[cfg(not(target_arch = "riscv64"))]
            unreachable!("NS16550 MMIO is riscv64 only")
        }
        Serial::Ns16550Port => {
            #[cfg(target_arch = "x86_64")]
            {
                let uart = ns16550_port::Driver::from_system_vars();
                log::info!("serial_driver: ns16550");
                Uart::Ns16550Port(uart)
            }
            #[cfg(not(target_arch = "x86_64"))]
            unreachable!("NS16550 port is x86_64 only")
        }
    }
}

struct HandlerImpl {
    uart: Uart,
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
    let uart = open_uart();
    // SAFETY: the template maps these queue and data regions into this domain.
    let tx = unsafe { serial_handle_from_connection(&config.tx) };
    let rx = unsafe { serial_handle_from_connection(&config.rx) };
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
    /// The PL011 and NS16550 drivers write the data register only after the
    /// transmit holding register has room, so this path does not arm the
    /// transmit interrupt.
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
                .expect("serial transmit accepts the byte");
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
            // The PL011 raises the receive interrupt when the FIFO count
            // crosses the trigger. It does not raise it again while the count
            // stays above that trigger. A byte that arrives after the drain
            // and before the interrupt clear sits in the FIFO with the
            // interrupt already cleared, and the rest of the line never
            // wakes the driver. `write NOTE.TXT hello` is longer than the
            // 16-byte FIFO, so drain once more after the clear.
            self.uart.handle_interrupt();
            self.drain_rx();
            self.drain_tx();
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
