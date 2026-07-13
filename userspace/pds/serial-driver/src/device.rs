//! Device-only serial path (Phase 42): UART MMIO/IRQ + **one** client (`serial-virt`).
//!
//! Uses the same postcard serial RPC as the legacy multi-client handler, but only
//! accepts channel 1 (the virtualiser). Clients never map into this PD.

use core::convert::Infallible;

use embedded_hal_nb::{nb, serial};
use heapless::Deque;
use sel4_driver_interfaces::HandleInterrupt;
use sel4_microkit::{Channel, ChannelSet, Handler, MessageInfo};
use sel4_microkit_simple_ipc as simple_ipc;

use lerux_driver_protocols::serial::{
    ErrorResponse, NonBlocking, Request, Response, SuccessResponse,
};

/// Channel 0 = device IRQ; channel 1 = protected RPC from `serial-virt`.
pub const DEVICE: Channel = Channel::new(0);
pub const VIRT: Channel = Channel::new(1);

pub struct DeviceHandler<Driver, const READ_BUF_SIZE: usize = 256> {
    driver: Driver,
    device: Channel,
    virt: Channel,
    buffer: Deque<u8, READ_BUF_SIZE>,
    notify: bool,
}

impl<Driver, const READ_BUF_SIZE: usize> DeviceHandler<Driver, READ_BUF_SIZE>
where
    Driver: serial::Read<u8> + serial::Write<u8> + HandleInterrupt,
{
    pub fn new(driver: Driver) -> Self {
        Self {
            driver,
            device: DEVICE,
            virt: VIRT,
            buffer: Deque::new(),
            notify: true,
        }
    }

    fn handle_request(&mut self, req: Request) -> Response {
        match req {
            Request::Read => {
                let v = self.buffer.pop_front();
                if v.is_some() {
                    self.notify = true;
                }
                Ok(SuccessResponse::Read(v.into()))
            }
            Request::Write(c) => NonBlocking::from_nb_result(self.driver.write(c))
                .map(SuccessResponse::Write)
                .map_err(|_| ErrorResponse::WriteError),
            Request::Flush => NonBlocking::from_nb_result(self.driver.flush())
                .map(SuccessResponse::Flush)
                .map_err(|_| ErrorResponse::FlushError),
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
            while !self.buffer.is_full() {
                match self.driver.read() {
                    Ok(v) => {
                        self.buffer.push_back(v).unwrap();
                    }
                    Err(err) => {
                        if let nb::Error::Other(err) = err {
                            log::debug!("read error: {err:?}");
                        }
                        break;
                    }
                }
            }
            self.driver.handle_interrupt();
            self.device.irq_ack().unwrap();
            if self.notify {
                self.virt.notify();
                self.notify = false;
            }
        }
        Ok(())
    }

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != self.virt {
            unreachable!("unexpected IPC channel (device-only expects serial-virt)");
        }
        Ok(match simple_ipc::recv::<Request>(msg_info) {
            Ok(req) => simple_ipc::send(self.handle_request(req)),
            Err(_) => simple_ipc::send_unspecified_error(),
        })
    }
}
