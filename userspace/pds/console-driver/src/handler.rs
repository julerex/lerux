//! One shell client. Writes paint the VGA page. Reads return keyboard bytes.
//!
//! The wire format is [`lerux_driver_protocols::serial`], the same protocol
//! `SerialClient` already speaks.

use core::convert::Infallible;

use heapless::Deque;
use sel4_microkit::{Channel, ChannelSet, Handler, MessageInfo};
use sel4_microkit_simple_ipc as simple_ipc;

use lerux_driver_protocols::serial::{ErrorResponse, NonBlocking, Request, SuccessResponse};

#[cfg(feature = "board-swift_sfx14_41g")]
use crate::usb_off::UsbKbd;
#[cfg(not(feature = "board-swift_sfx14_41g"))]
use crate::xhci::UsbKbd;

use crate::{
    device::Device,
    scancode::{decode, KeyState},
    screen::{Damage, Screen},
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
        self.sync_cursor();
    }

    fn sync_cursor(&self) {
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        self.device.sync_cursor(&self.screen);
        #[cfg(feature = "board-swift_sfx14_41g")]
        Device::sync_cursor(&self.screen);
    }

    fn handle_request(&mut self, req: &Request) -> SuccessResponse {
        match req {
            Request::Read => {
                let byte = self.rx.pop_front();
                if byte.is_some() {
                    self.notify = true;
                }
                SuccessResponse::Read(byte.into())
            }
            Request::Write(byte) => {
                self.paint(*byte);
                SuccessResponse::Write(NonBlocking::Ready(()))
            }
            Request::Flush => {
                self.sync_cursor();
                SuccessResponse::Flush(NonBlocking::Ready(()))
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
            let n = poll_usb(&mut self.usb, &mut buf);
            for &byte in &buf[..n] {
                let _ = self.rx.push_back(byte);
            }
            ack_usb(&mut self.usb);
            self.usb_irq.irq_ack().expect("ack usb irq");
        }
        if ps2 {
            // A USB boot keyboard owns the shell. QEMU `send-key` hits PS/2 too.
            if usb_ready(&self.usb) {
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
            Ok(req) => simple_ipc::send(Ok::<_, ErrorResponse>(self.handle_request(&req))),
            Err(_) => simple_ipc::send_unspecified_error(),
        })
    }
}

#[cfg(not(feature = "board-swift_sfx14_41g"))]
fn usb_ready(usb: &UsbKbd) -> bool {
    usb.ready()
}

#[cfg(feature = "board-swift_sfx14_41g")]
fn usb_ready(_usb: &UsbKbd) -> bool {
    UsbKbd::ready()
}

#[cfg(not(feature = "board-swift_sfx14_41g"))]
fn poll_usb(usb: &mut UsbKbd, buf: &mut [u8]) -> usize {
    usb.poll(buf)
}

#[cfg(feature = "board-swift_sfx14_41g")]
fn poll_usb(_usb: &mut UsbKbd, buf: &mut [u8]) -> usize {
    UsbKbd::poll(buf)
}

#[cfg(not(feature = "board-swift_sfx14_41g"))]
fn ack_usb(usb: &mut UsbKbd) {
    usb.ack_irq();
}

#[cfg(feature = "board-swift_sfx14_41g")]
fn ack_usb(_usb: &mut UsbKbd) {
    UsbKbd::ack_irq();
}
