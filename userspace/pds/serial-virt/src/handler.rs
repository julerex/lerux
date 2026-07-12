//! Multi-client serial RPC (same wire format as legacy serial-driver handler).

use core::convert::Infallible;

use heapless::Deque;
use lerux_serial_queue::SerialQueueHandle;
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
    /// Local RX staging (from driver RX queue) so Read RPC stays non-blocking.
    buffer: Deque<u8, READ_BUF_SIZE>,
    tx: SerialQueueHandle,
    rx: SerialQueueHandle,
    notify_rx_client: bool,
}

impl<const NUM_CLIENTS: usize, const READ_BUF_SIZE: usize> HandlerImpl<NUM_CLIENTS, READ_BUF_SIZE> {
    pub fn new(
        driver: Channel,
        clients: [Channel; NUM_CLIENTS],
        tx: SerialQueueHandle,
        rx: SerialQueueHandle,
    ) -> Self {
        Self {
            driver,
            clients,
            buffer: Deque::new(),
            tx,
            rx,
            notify_rx_client: true,
        }
    }

    fn client_index(&self, channel: Channel) -> Option<usize> {
        self.clients.iter().position(|c| *c == channel)
    }

    fn pull_rx_from_driver(&mut self) {
        while !self.buffer.is_full() {
            match self.rx.dequeue() {
                Some(b) => {
                    let _ = self.buffer.push_back(b);
                }
                None => break,
            }
        }
        if self.rx.consumer_should_signal_producer() {
            self.driver.notify();
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
            Request::Write(c) => {
                if self.tx.enqueue(c) {
                    self.driver.notify();
                    Ok(SuccessResponse::Write(NonBlocking::Ready(())))
                } else {
                    self.tx.request_signal();
                    // Double-check after flag (sDDF protocol).
                    if self.tx.enqueue(c) {
                        self.driver.notify();
                        Ok(SuccessResponse::Write(NonBlocking::Ready(())))
                    } else {
                        Ok(SuccessResponse::Write(NonBlocking::WouldBlock))
                    }
                }
            }
            Request::Flush => {
                // Bytes already in TX queue; notify driver to drain.
                self.driver.notify();
                Ok(SuccessResponse::Flush(NonBlocking::Ready(())))
            }
        }
    }
}

impl<const NUM_CLIENTS: usize> Handler for HandlerImpl<NUM_CLIENTS> {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(self.driver) {
            self.pull_rx_from_driver();
            // Free TX space may have opened.
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
