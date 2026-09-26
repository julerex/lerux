//! VGA text console and PS/2 keyboard for the on-screen shell.
//!
//! Channel 0 is the keyboard IRQ. Channel 1 is the shell, speaking the serial
//! byte protocol. COM1 stays with `serial-driver`; this PD does not touch it.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

mod scancode;
mod screen;

#[cfg(all(not(test), feature = "hardware"))]
mod device;
#[cfg(all(not(test), feature = "hardware"))]
mod handler;

#[cfg(all(not(test), feature = "hardware"))]
use sel4_microkit::{protection_domain, Channel};

#[cfg(all(not(test), feature = "hardware"))]
const IRQ: Channel = Channel::new(0);
#[cfg(all(not(test), feature = "hardware"))]
const SHELL: Channel = Channel::new(1);

#[cfg(all(not(test), feature = "hardware"))]
#[protection_domain]
fn init() -> handler::HandlerImpl {
    use lerux_logging::{debug, log};

    use crate::screen::Screen;

    debug::init().expect("debug log");
    log::info!("console-driver: vga text");
    let device = device::Device::from_system();
    if device.init_keyboard() {
        log::info!("console-driver: i8042 ready");
    } else {
        log::info!("console-driver: i8042 timeout");
    }
    device.enable_cursor();
    let screen = Screen::with_banner();
    device.present_all(&screen);
    device.sync_cursor(&screen);
    handler::HandlerImpl::new(device, screen, IRQ, SHELL)
}
