//! VGA text console and keyboard for the on-screen shell.
//!
//! Channel 0 is the PS/2 IRQ. Channel 1 is the shell, speaking the serial byte
//! protocol. Channel 2 is the xHCI MSI. COM1 stays with `serial-driver`.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

mod hid;
mod scancode;
mod screen;
mod usb;

#[cfg(all(not(test), feature = "hardware"))]
mod device;
#[cfg(all(not(test), feature = "hardware"))]
mod handler;
#[cfg(all(not(test), feature = "hardware"))]
mod xhci;

#[cfg(all(not(test), feature = "hardware"))]
use sel4_microkit::{protection_domain, Channel};

#[cfg(all(not(test), feature = "hardware"))]
const IRQ: Channel = Channel::new(0);
#[cfg(all(not(test), feature = "hardware"))]
const SHELL: Channel = Channel::new(1);
#[cfg(all(not(test), feature = "hardware"))]
const USB_IRQ: Channel = Channel::new(2);

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
    let usb = xhci::UsbKbd::bring_up();
    device.enable_cursor();
    let screen = Screen::with_banner();
    device.present_all(&screen);
    device.sync_cursor(&screen);
    handler::HandlerImpl::new(device, screen, usb, IRQ, USB_IRQ, SHELL)
}
