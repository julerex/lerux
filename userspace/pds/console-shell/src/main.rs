//! Start menu on the VGA text console.
//!
//! Channel 0 is `console_driver` (keyboard bytes in, screen bytes out).
//! On the desk, channel 1 is `serial_driver` (the same output bytes on COM1).
//! On the Swift X, channel 1 is the read-only NVMe result. This image
//! has no page painter, so the browser choice reports that it is unavailable.

#![no_std]
#![no_main]

use embedded_hal_nb::{
    nb,
    serial::{Read, Write},
};
#[cfg(feature = "board-swift_sfx14_41g")]
use sel4_microkit::memory_region_symbol;
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible, MessageInfo};
use sel4_microkit_driver_adapters::serial::client::Client;

use lerux_calc::{eval, write_i64, Error};
#[cfg(feature = "board-swift_sfx14_41g")]
use lerux_swift::{key_ok, write_key};

const CONSOLE: Channel = Channel::new(0);
#[cfg(not(feature = "board-swift_sfx14_41g"))]
const SERIAL: Channel = Channel::new(1);
#[cfg(feature = "board-swift_sfx14_41g")]
const DISK: Channel = Channel::new(1);
#[cfg(feature = "board-swift_sfx14_41g")]
const WIFI: Channel = Channel::new(2);
const LINE_MAX: usize = 80;
#[cfg(not(feature = "board-swift_sfx14_41g"))]
const MENU: &[u8] = b"1 browser\n2 maths\n";
#[cfg(feature = "board-swift_sfx14_41g")]
const MENU: &[u8] = b"1 browser\n2 maths\n3 wifi\n";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Menu,
    Maths,
    #[cfg(feature = "board-swift_sfx14_41g")]
    Ssid,
    #[cfg(feature = "board-swift_sfx14_41g")]
    Psk,
}

struct HandlerImpl {
    screen: Client,
    #[cfg(not(feature = "board-swift_sfx14_41g"))]
    com1: Client,
    #[cfg(feature = "board-swift_sfx14_41g")]
    disk_done: bool,
    #[cfg(feature = "board-swift_sfx14_41g")]
    ssid: [u8; 32],
    #[cfg(feature = "board-swift_sfx14_41g")]
    ssid_len: usize,
    mode: Mode,
    line: [u8; LINE_MAX],
    len: usize,
    overflow: bool,
}

#[protection_domain]
fn init() -> HandlerImpl {
    let mut shell = HandlerImpl {
        screen: Client::new(CONSOLE),
        #[cfg(not(feature = "board-swift_sfx14_41g"))]
        com1: Client::new(SERIAL),
        #[cfg(feature = "board-swift_sfx14_41g")]
        disk_done: false,
        #[cfg(feature = "board-swift_sfx14_41g")]
        ssid: [0; 32],
        #[cfg(feature = "board-swift_sfx14_41g")]
        ssid_len: 0,
        mode: Mode::Menu,
        line: [0; LINE_MAX],
        len: 0,
        overflow: false,
    };
    shell.emit(MENU);
    #[cfg(feature = "board-swift_sfx14_41g")]
    shell.emit_disk();
    shell
}

impl HandlerImpl {
    #[cfg(feature = "board-swift_sfx14_41g")]
    fn emit_disk(&mut self) {
        if self.disk_done {
            return;
        }
        let bytes = disk_payload();
        if bytes.is_empty() {
            return;
        }
        self.disk_done = true;
        self.emit(bytes);
        self.emit(b"\n");
    }

    fn emit(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            push(&mut self.screen, byte);
            #[cfg(not(feature = "board-swift_sfx14_41g"))]
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
                #[cfg(feature = "board-swift_sfx14_41g")]
                b"3" | b"wifi" => {
                    self.mode = Mode::Ssid;
                    self.ssid_len = 0;
                    self.emit(b"ssid> ");
                }
                _ => self.reject(),
            },
            #[cfg(feature = "board-swift_sfx14_41g")]
            Mode::Ssid if line == b"quit" => {
                self.mode = Mode::Menu;
                self.emit(MENU);
            }
            #[cfg(feature = "board-swift_sfx14_41g")]
            Mode::Ssid => {
                if (1..=32).contains(&line.len()) {
                    self.ssid[..line.len()].copy_from_slice(line);
                    self.ssid_len = line.len();
                    self.mode = Mode::Psk;
                    self.emit(b"ssid ok\npsk> ");
                } else {
                    self.wifi_err();
                }
            }
            #[cfg(feature = "board-swift_sfx14_41g")]
            Mode::Psk if line == b"quit" => {
                self.mode = Mode::Menu;
                self.emit(MENU);
            }
            #[cfg(feature = "board-swift_sfx14_41g")]
            Mode::Psk => {
                let name_len = self.ssid_len;
                if key_ok(&self.ssid[..name_len], line) {
                    self.post_wifi(line);
                } else {
                    self.wifi_err();
                }
            }
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
            #[cfg(feature = "board-swift_sfx14_41g")]
            Mode::Ssid | Mode::Psk => self.wifi_err(),
        }
    }

    #[cfg(feature = "board-swift_sfx14_41g")]
    fn wifi_err(&mut self) {
        self.mode = Mode::Menu;
        self.emit(b"wifi err\n");
        self.emit(MENU);
    }

    #[cfg(feature = "board-swift_sfx14_41g")]
    fn post_wifi(&mut self, psk: &[u8]) {
        let name_len = self.ssid_len;
        let mut name = [0u8; 32];
        name[..name_len].copy_from_slice(&self.ssid[..name_len]);
        let ptr = memory_region_symbol!(wifi_key_vaddr: *mut u8).as_ptr();
        let page = unsafe { core::slice::from_raw_parts_mut(ptr, 0x1000) };
        if !write_key(page, &name[..name_len], psk) {
            self.wifi_err();
            return;
        }
        WIFI.notify();
        self.mode = Mode::Menu;
        self.emit(b"psk ok\n");
        self.emit(MENU);
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

#[cfg(feature = "board-swift_sfx14_41g")]
fn disk_payload() -> &'static [u8] {
    let ptr = memory_region_symbol!(disk_result_vaddr: *mut u8).as_ptr();
    let page = unsafe { core::slice::from_raw_parts(ptr, 0x1000) };
    let n = u16::from_le_bytes([page[0], page[1]]) as usize;
    let n = n.min(256).min(page.len().saturating_sub(2));
    &page[2..2 + n]
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
        #[cfg(feature = "board-swift_sfx14_41g")]
        if channels.contains(DISK) {
            self.emit_disk();
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
