//! Start menu on the VGA text console.
//!
//! Channel 0 is `console_driver` (keyboard bytes in, screen bytes out).
//! Channel 1 is `serial_driver` (the same output bytes on COM1). This image
//! has no page painter, so the browser choice reports that it is unavailable.

#![no_std]
#![no_main]

use embedded_hal_nb::{
    nb,
    serial::{Read, Write},
};
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible, MessageInfo};
use sel4_microkit_driver_adapters::serial::client::Client;

use lerux_calc::{eval, write_i64, Error};

const CONSOLE: Channel = Channel::new(0);
const SERIAL: Channel = Channel::new(1);
const LINE_MAX: usize = 80;
const MENU: &[u8] = b"1 browser\n2 maths\n";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Menu,
    Maths,
}

struct HandlerImpl {
    screen: Client,
    com1: Client,
    mode: Mode,
    line: [u8; LINE_MAX],
    len: usize,
    overflow: bool,
}

#[protection_domain]
fn init() -> HandlerImpl {
    let mut shell = HandlerImpl {
        screen: Client::new(CONSOLE),
        com1: Client::new(SERIAL),
        mode: Mode::Menu,
        line: [0; LINE_MAX],
        len: 0,
        overflow: false,
    };
    shell.emit(MENU);
    shell
}

impl HandlerImpl {
    fn emit(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            push(&mut self.screen, byte);
            push(&mut self.com1, byte);
        }
    }

    fn emit_value(&mut self, prefix: &[u8], value: i64) {
        let mut line = [0u8; 32];
        line[..prefix.len()].copy_from_slice(prefix);
        let mut len = prefix.len();
        len += write_i64(&mut line[len..], value);
        line[len] = b'\n';
        len += 1;
        self.emit(&line[..len]);
    }

    fn drain(&mut self) {
        loop {
            match self.screen.read() {
                Ok(byte) => self.on_byte(byte),
                Err(nb::Error::WouldBlock) => return,
                Err(nb::Error::Other(_)) => unreachable!("console read"),
            }
        }
    }

    fn on_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {}
            b'\r' => {
                self.emit(b"\n");
                let overflow = self.overflow;
                let len = self.len;
                self.len = 0;
                self.overflow = false;
                if overflow {
                    self.reject();
                } else {
                    let mut owned = [0u8; LINE_MAX];
                    owned[..len].copy_from_slice(&self.line[..len]);
                    self.dispatch(&owned[..len]);
                }
            }
            0x7f => {
                if self.len > 0 && !self.overflow {
                    self.len -= 1;
                    self.emit(&[0x08, b' ', 0x08]);
                }
            }
            _ if self.len == LINE_MAX => self.overflow = true,
            byte => {
                self.line[self.len] = byte;
                self.len += 1;
                self.emit(&[byte]);
            }
        }
    }

    fn dispatch(&mut self, raw: &[u8]) {
        let line = trim(raw);
        match self.mode {
            Mode::Menu => match line {
                b"1" | b"browser" => {
                    self.emit(b"browser unavailable\n");
                    self.emit(MENU);
                }
                b"2" | b"maths" => {
                    self.mode = Mode::Maths;
                    self.emit(b"maths> ");
                }
                _ => self.reject(),
            },
            Mode::Maths if line == b"quit" => {
                self.mode = Mode::Menu;
                self.emit(MENU);
            }
            Mode::Maths => {
                match eval(line) {
                    Ok(value) => self.emit_value(b"maths ", value),
                    Err(Error) => self.emit(b"maths err\n"),
                }
                self.emit(b"maths> ");
            }
        }
    }

    fn reject(&mut self) {
        match self.mode {
            Mode::Menu => {
                self.emit(b"err\n");
                self.emit(MENU);
            }
            Mode::Maths => {
                self.emit(b"maths err\n");
                self.emit(b"maths> ");
            }
        }
    }
}

fn push(client: &mut Client, byte: u8) {
    loop {
        match client.write(byte) {
            Ok(()) => return,
            Err(nb::Error::WouldBlock) => core::hint::spin_loop(),
            Err(nb::Error::Other(_)) => unreachable!("serial write"),
        }
    }
}

fn trim(line: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = line.len();
    while start < end && line[start] == b' ' {
        start += 1;
    }
    while end > start && line[end - 1] == b' ' {
        end -= 1;
    }
    &line[start..end]
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(CONSOLE) {
            self.drain();
        }
        Ok(())
    }

    fn protected(
        &mut self,
        _channel: Channel,
        _msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        unreachable!("the shell calls the drivers");
    }
}
