//! Serial driver IPC handler (single- or multi-client).

use core::convert::Infallible;

use embedded_hal_nb::{nb, serial};
use heapless::Deque;
use sel4_driver_interfaces::HandleInterrupt;
use sel4_microkit::{Channel, ChannelSet, Handler, MessageInfo};
use sel4_microkit_simple_ipc as simple_ipc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum NonBlocking<T> {
    Ready(T),
    WouldBlock,
}

impl<T> NonBlocking<T> {
    fn from_nb_result<E>(r: nb::Result<T, E>) -> Result<Self, E> {
        match r {
            Ok(v) => Ok(Self::Ready(v)),
            Err(nb::Error::WouldBlock) => Ok(Self::WouldBlock),
            Err(nb::Error::Other(err)) => Err(err),
        }
    }
}

impl<T> From<Option<T>> for NonBlocking<T> {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => NonBlocking::Ready(v),
            None => NonBlocking::WouldBlock,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum Request {
    Read,
    Write(u8),
    Flush,
}

type Response = Result<SuccessResponse, ErrorResponse>;

#[derive(Debug, Serialize, Deserialize)]
enum SuccessResponse {
    Read(NonBlocking<u8>),
    Write(NonBlocking<()>),
    Flush(NonBlocking<()>),
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
enum ErrorResponse {
    WriteError,
    FlushError,
}

/// Handle serial IPC from one or more client PDs.
pub struct HandlerImpl<Driver, const NUM_CLIENTS: usize, const READ_BUF_SIZE: usize = 256> {
    driver: Driver,
    device: Channel,
    clients: [Channel; NUM_CLIENTS],
    buffer: Deque<u8, READ_BUF_SIZE>,
    notify: bool,
}

impl<Driver, const NUM_CLIENTS: usize, const READ_BUF_SIZE: usize>
    HandlerImpl<Driver, NUM_CLIENTS, READ_BUF_SIZE>
where
    Driver: serial::Read<u8> + serial::Write<u8> + HandleInterrupt,
{
    pub fn new(driver: Driver, device: Channel, clients: [Channel; NUM_CLIENTS]) -> Self {
        Self {
            driver,
            device,
            clients,
            buffer: Deque::new(),
            notify: true,
        }
    }

    fn client_index(&self, channel: Channel) -> Option<usize> {
        self.clients.iter().position(|c| *c == channel)
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

impl<Driver, const NUM_CLIENTS: usize> Handler for HandlerImpl<Driver, NUM_CLIENTS>
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
                self.clients[0].notify();
                self.notify = false;
            }
        } else {
            unreachable!("unexpected notification channels");
        }
        Ok(())
    }

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if self.client_index(channel).is_none() {
            unreachable!("unexpected IPC channel");
        }

        Ok(match simple_ipc::recv::<Request>(msg_info) {
            Ok(req) => simple_ipc::send(self.handle_request(req)),
            Err(_) => simple_ipc::send_unspecified_error(),
        })
    }
}
