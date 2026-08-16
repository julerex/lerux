//! Multi-client serial RPC mux → single device RPC to `serial-driver` (Phase 42).
//!
//! Wire format matches rust-sel4 `SerialClient` / legacy serial-driver handler.
//! Device trust boundary: only this PD may call the UART driver.

use core::convert::Infallible;

use heapless::Deque;
use sel4_microkit::{Channel, ChannelSet, Handler, MessageInfo};
use sel4_microkit_simple_ipc as simple_ipc;

use lerux_driver_protocols::serial::{
    ErrorResponse, NonBlocking, Request, Response, SuccessResponse,
};

pub struct HandlerImpl<const NUM_CLIENTS: usize, const READ_BUF_SIZE: usize = 256> {
    driver: Channel,
    clients: [Channel; NUM_CLIENTS],
    buffer: Deque<u8, READ_BUF_SIZE>,
    /// Phase 65: per-client TX so one writer cannot clobber another's pending byte.
    tx: [Deque<u8, 64>; NUM_CLIENTS],
    notify_rx_client: bool,
}

impl<const NUM_CLIENTS: usize, const READ_BUF_SIZE: usize> HandlerImpl<NUM_CLIENTS, READ_BUF_SIZE> {
    pub fn new(driver: Channel, clients: [Channel; NUM_CLIENTS]) -> Self {
        Self {
            driver,
            clients,
            buffer: Deque::new(),
            tx: [const { Deque::new() }; NUM_CLIENTS],
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

    fn drain_tx(&mut self, idx: usize) {
        while let Some(b) = self.tx[idx].pop_front() {
            match self.driver_request(Request::Write(b)) {
                Ok(SuccessResponse::Write(NonBlocking::Ready(()))) => {}
                Ok(SuccessResponse::Write(NonBlocking::WouldBlock)) => {
                    let _ = self.tx[idx].push_front(b);
                    break;
                }
                _ => break,
            }
        }
    }

    fn handle_request(&mut self, idx: usize, req: Request) -> Response {
        match req {
            Request::Read => {
                self.pull_rx_from_driver();
                let v = self.buffer.pop_front();
                if v.is_some() {
                    self.notify_rx_client = true;
                }
                Ok(SuccessResponse::Read(v.into()))
            }
            Request::Write(c) => {
                if self.tx[idx].push_back(c).is_err() {
                    return Ok(SuccessResponse::Write(NonBlocking::WouldBlock));
                }
                self.drain_tx(idx);
                Ok(SuccessResponse::Write(NonBlocking::Ready(())))
            }
            Request::Flush => {
                self.drain_tx(idx);
                match self.driver_request(Request::Flush) {
                    Ok(SuccessResponse::Flush(nb)) => Ok(SuccessResponse::Flush(nb)),
                    Ok(_) => Err(ErrorResponse::FlushError),
                    Err(e) => Err(e),
                }
            }
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
        let Some(idx) = self.client_index(channel) else {
            unreachable!("unexpected IPC channel");
        };

        Ok(match simple_ipc::recv::<Request>(msg_info) {
            Ok(req) => simple_ipc::send(self.handle_request(idx, req)),
            Err(_) => simple_ipc::send_unspecified_error(),
        })
    }
}
