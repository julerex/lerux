//! One shell client. Writes paint the VGA page. Reads return keyboard bytes.
//!
//! The wire format is [`lerux_driver_protocols::serial`], the same protocol
//! `SerialClient` already speaks.

use core::convert::Infallible;

use heapless::Deque;
use sel4_microkit::{Channel, ChannelSet, Handler, MessageInfo};
use sel4_microkit_simple_ipc as simple_ipc;

use lerux_driver_protocols::serial::{NonBlocking, Request, Response, SuccessResponse};

use crate::{
    device::Device,
    scancode::{decode, KeyState},
    screen::{Damage, Screen},
    xhci::UsbKbd,
};

pub struct HandlerImpl {
    device: Device,
    screen: Screen,
    usb: UsbKbd,
    keys: KeyState,
    rx: Deque<u8, 64>,
    ps2_irq: Channel,
    usb_irq: Channel,
    client: Channel,
    /// Notify the shell on the next key only after it has drained the queue.
    notify: bool,
}

impl HandlerImpl {
    pub fn new(
        device: Device,
        screen: Screen,
        usb: UsbKbd,
        ps2_irq: Channel,
        usb_irq: Channel,
        client: Channel,
    ) -> Self {
        Self {
            device,
            screen,
            usb,
            keys: KeyState::default(),
            rx: Deque::new(),
            ps2_irq,
            usb_irq,
            client,
            notify: true,
        }
    }

    fn paint(&mut self, byte: u8) {
        match self.screen.apply_byte(byte) {
            Damage::None => {}
            Damage::Cells { start, end } => self.device.present_cells(&self.screen, start, end),
            Damage::All => self.device.present_all(&self.screen),
        }
        self.device.sync_cursor(&self.screen);
    }

    fn handle_request(&mut self, req: Request) -> Response {
        match req {
            Request::Read => {
                let byte = self.rx.pop_front();
                if byte.is_some() {
                    self.notify = true;
                }
                Ok(SuccessResponse::Read(byte.into()))
            }
            Request::Write(byte) => {
                self.paint(byte);
                Ok(SuccessResponse::Write(NonBlocking::Ready(())))
            }
            Request::Flush => {
                self.device.sync_cursor(&self.screen);
                Ok(SuccessResponse::Flush(NonBlocking::Ready(())))
            }
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        let ps2 = channels.contains(self.ps2_irq);
        let usb = channels.contains(self.usb_irq);
        if !ps2 && !usb {
            unreachable!("unexpected notification");
        }
        if usb {
            // `poll` borrows the controller. Copy bytes out before the queue.
            let mut buf = [0u8; 64];
            let n = self.usb.poll(&mut buf);
            for &byte in &buf[..n] {
                let _ = self.rx.push_back(byte);
            }
            self.usb.ack_irq();
            self.usb_irq.irq_ack().expect("ack usb irq");
        }
        if ps2 {
            // A USB boot keyboard owns the shell. QEMU `send-key` hits PS/2 too.
            if self.usb.ready() {
                while self.device.read_scancode().is_some() {}
            } else {
                while let Some(code) = self.device.read_scancode() {
                    if let Some(byte) = decode(&mut self.keys, code) {
                        let _ = self.rx.push_back(byte);
                    }
                }
            }
            self.ps2_irq.irq_ack().expect("ack keyboard irq");
        }
        if self.notify && !self.rx.is_empty() {
            self.client.notify();
            self.notify = false;
        }
        Ok(())
    }

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != self.client {
            unreachable!("unexpected IPC channel");
        }
        Ok(match simple_ipc::recv::<Request>(msg_info) {
            Ok(req) => simple_ipc::send(self.handle_request(req)),
            Err(_) => simple_ipc::send_unspecified_error(),
        })
    }
}
