//! Device-only serial path: UART + shared queues to `serial-virt` (Phase 42).
//!
//! No client postcard RPC; clients talk to `serial-virt` only.

use core::convert::Infallible;

use embedded_hal_nb::{nb, serial};
use lerux_serial_queue::{SerialQueue, SerialQueueHandle, DEFAULT_CAPACITY};
use sel4_driver_interfaces::HandleInterrupt;
use sel4_microkit::{memory_region_symbol, Channel, ChannelSet, Handler};

/// Channel 0 = device IRQ; channel 1 = notify `serial-virt`.
pub const DEVICE: Channel = Channel::new(0);
pub const VIRT: Channel = Channel::new(1);

pub struct DeviceHandler<Driver> {
    driver: Driver,
    device: Channel,
    virt: Channel,
    tx: SerialQueueHandle,
    rx: SerialQueueHandle,
    /// Byte dequeued from TX but not yet accepted by UART.
    pending_tx: Option<u8>,
}

impl<Driver> DeviceHandler<Driver>
where
    Driver: serial::Read<u8> + serial::Write<u8> + HandleInterrupt,
{
    /// # Safety
    /// Queue symbols must be mapped shared with `serial-virt`.
    pub unsafe fn new(driver: Driver) -> Self {
        let tx_q = memory_region_symbol!(serial_tx_queue: *mut SerialQueue).as_ptr();
        let tx_d = memory_region_symbol!(
            serial_tx_data: *mut [u8],
            n = DEFAULT_CAPACITY
        )
        .as_ptr()
        .cast::<u8>();
        let rx_q = memory_region_symbol!(serial_rx_queue: *mut SerialQueue).as_ptr();
        let rx_d = memory_region_symbol!(
            serial_rx_data: *mut [u8],
            n = DEFAULT_CAPACITY
        )
        .as_ptr()
        .cast::<u8>();
        let tx = unsafe { SerialQueueHandle::new(tx_q, tx_d, DEFAULT_CAPACITY) };
        let rx = unsafe { SerialQueueHandle::new(rx_q, rx_d, DEFAULT_CAPACITY) };
        tx.init_shared();
        rx.init_shared();
        Self {
            driver,
            device: DEVICE,
            virt: VIRT,
            tx,
            rx,
            pending_tx: None,
        }
    }

    fn drain_tx_to_uart(&mut self) {
        loop {
            let b = match self.pending_tx.take() {
                Some(b) => b,
                None => match self.tx.dequeue() {
                    Some(b) => b,
                    None => break,
                },
            };
            match self.driver.write(b) {
                Ok(()) => {}
                Err(nb::Error::WouldBlock) => {
                    self.pending_tx = Some(b);
                    break;
                }
                Err(nb::Error::Other(_)) => {
                    self.pending_tx = Some(b);
                    break;
                }
            }
        }
        let _ = self.driver.flush();
        if self.tx.consumer_should_signal_producer() {
            self.virt.notify();
        }
    }

    fn fill_rx_from_uart(&mut self) {
        let mut any = false;
        while !self.rx.is_full() {
            match self.driver.read() {
                Ok(b) => {
                    if self.rx.enqueue(b) {
                        any = true;
                    } else {
                        break;
                    }
                }
                Err(nb::Error::WouldBlock) => break,
                Err(nb::Error::Other(_)) => break,
            }
        }
        if any {
            self.virt.notify();
        }
    }
}

impl<Driver> Handler for DeviceHandler<Driver>
where
    Driver: serial::Read<u8> + serial::Write<u8> + HandleInterrupt,
{
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(self.device) {
            self.fill_rx_from_uart();
            self.driver.handle_interrupt();
            self.device.irq_ack().unwrap();
            // IRQ may also free TX capacity on some UARTs.
            self.drain_tx_to_uart();
        }
        if channels.contains(self.virt) {
            self.drain_tx_to_uart();
            self.fill_rx_from_uart();
        }
        Ok(())
    }
}
