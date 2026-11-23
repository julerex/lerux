//! COM1 on the desk image has no client.
//!
//! The UART interrupt is level-triggered. A receive interrupt has to empty the
//! FIFO before the ack, and it must not notify a channel the system does not
//! connect. `Channel::notify` panics when that mask bit is clear.

use core::convert::Infallible;

use embedded_hal_nb::{nb, serial};
use sel4_driver_interfaces::HandleInterrupt;
use sel4_microkit::{Channel, ChannelSet, Handler};

/// `<irq id="0">` on `serial_driver`.
const DEVICE: Channel = Channel::new(0);

pub struct DiscardHandler<Driver> {
    driver: Driver,
    device: Channel,
}

impl<Driver> DiscardHandler<Driver>
where
    Driver: serial::Read<u8> + HandleInterrupt,
{
    pub fn new(driver: Driver) -> Self {
        Self {
            driver,
            device: DEVICE,
        }
    }
}

impl<Driver> Handler for DiscardHandler<Driver>
where
    Driver: serial::Read<u8> + HandleInterrupt,
{
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if !channels.contains(self.device) {
            unreachable!("unexpected notification channels");
        }
        loop {
            match self.driver.read() {
                Ok(_) => {}
                Err(nb::Error::WouldBlock) => break,
                Err(nb::Error::Other(err)) => {
                    log::debug!("read error: {err:?}");
                    break;
                }
            }
        }
        self.driver.handle_interrupt();
        self.device.irq_ack().expect("ack the serial interrupt");
        Ok(())
    }
}
