//! Multi-client serial RPC mux → single device RPC to `serial-driver` (Phase 42).
//!
//! Wire format matches rust-sel4 `SerialClient` / legacy serial-driver handler.
//! Device trust boundary: only this PD may call the UART driver.

use core::convert::Infallible;

use heapless::Deque;
use sel4_microkit::{Channel, ChannelSet, Handler, MessageInfo};
use sel4_microkit_simple_ipc as simple_ipc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum NonBlocking<T> {
    Ready(T),
    WouldBlock,
}

impl<T> From<Option<T>> for NonBlocking<T> {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => NonBlocking::Ready(v),
            None => NonBlocking::WouldBlock,
        }
    }
}

impl<T, E> From<NonBlocking<T>> for nb::Result<T, E> {
    fn from(v: NonBlocking<T>) -> Self {
        match v {
            NonBlocking::Ready(v) => Ok(v),
            NonBlocking::WouldBlock => Err(nb::Error::WouldBlock),
        }
    }
}

use embedded_hal_nb::nb;

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

pub struct HandlerImpl<const NUM_CLIENTS: usize, const READ_BUF_SIZE: usize = 256> {
    driver: Channel,
    clients: [Channel; NUM_CLIENTS],
    buffer: Deque<u8, READ_BUF_SIZE>,
    notify_rx_client: bool,
}

impl<const NUM_CLIENTS: usize, const READ_BUF_SIZE: usize> HandlerImpl<NUM_CLIENTS, READ_BUF_SIZE> {
    pub fn new(driver: Channel, clients: [Channel; NUM_CLIENTS]) -> Self {
        Self {
            driver,
            clients,
            buffer: Deque::new(),
            notify_rx_client: true,
        }
    }

    fn client_index(&self, channel: Channel) -> Option<usize> {
        self.clients.iter().position(|c| *c == channel)
    }

    fn driver_request(&self, req: Request) -> Result<SuccessResponse, ErrorResponse> {
        simple_ipc::call::<Request, Response>(self.driver, req)
            .unwrap_or(Err(ErrorResponse::WriteError))
    }

    fn pull_rx_from_driver(&mut self) {
        while !self.buffer.is_full() {
            match self.driver_request(Request::Read) {
                Ok(SuccessResponse::Read(NonBlocking::Ready(b))) => {
                    let _ = self.buffer.push_back(b);
                }
                _ => break,
            }
        }
    }

    fn handle_request(&mut self, req: Request) -> Response {
        match req {
            Request::Read => {
                self.pull_rx_from_driver();
                let v = self.buffer.pop_front();
                if v.is_some() {
                    self.notify_rx_client = true;
                }
                Ok(SuccessResponse::Read(v.into()))
            }
            Request::Write(c) => match self.driver_request(Request::Write(c)) {
                Ok(SuccessResponse::Write(nb)) => Ok(SuccessResponse::Write(nb)),
                Ok(_) => Err(ErrorResponse::WriteError),
                Err(e) => Err(e),
            },
            Request::Flush => match self.driver_request(Request::Flush) {
                Ok(SuccessResponse::Flush(nb)) => Ok(SuccessResponse::Flush(nb)),
                Ok(_) => Err(ErrorResponse::FlushError),
                Err(e) => Err(e),
            },
        }
    }
}

impl<const NUM_CLIENTS: usize> Handler for HandlerImpl<NUM_CLIENTS> {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(self.driver) {
            self.pull_rx_from_driver();
            if self.notify_rx_client && !self.buffer.is_empty() {
                self.clients[0].notify();
                self.notify_rx_client = false;
            }
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
