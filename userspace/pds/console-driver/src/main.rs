//! VGA text console and keyboard for the on-screen shell.
//!
//! Channel 0 is the PS/2 IRQ. Channel 1 is the shell, speaking the serial byte
//! protocol. On the desk, channel 2 is the xHCI MSI and COM1 stays with
//! `serial-driver`. The Inspiron image has neither.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(all(
    not(feature = "board-swift_sfx14_41g"),
    not(feature = "board-inspiron_n5010")
))]
mod hid;
mod scancode;
mod screen;
#[cfg(all(
    not(feature = "board-swift_sfx14_41g"),
    not(feature = "board-inspiron_n5010")
))]
mod usb;

#[cfg(all(not(test), feature = "board-pc_z97_d3h"))]
mod boot_line;
#[cfg(all(not(test), feature = "hardware"))]
mod device;
#[cfg(all(not(test), feature = "hardware"))]
mod handler;
#[cfg(all(
    not(test),
    feature = "hardware",
    any(feature = "board-swift_sfx14_41g", feature = "board-inspiron_n5010")
))]
mod usb_off;
#[cfg(all(
    not(test),
    feature = "hardware",
    not(feature = "board-swift_sfx14_41g"),
    not(feature = "board-inspiron_n5010")
))]
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

    #[cfg(feature = "board-pc_z97_d3h")]
    boot_line::install();
    debug::init().expect("debug log");
    #[cfg(not(feature = "board-swift_sfx14_41g"))]
    log::info!("console-driver: vga text");
    #[cfg(feature = "board-swift_sfx14_41g")]
    log::info!("console-driver: panel");
    let device = device::Device::from_system();
    if device.init_keyboard() {
        log::info!("console-driver: i8042 ready");
    } else {
        log::info!("console-driver: i8042 timeout");
    }
    #[cfg(all(
        not(feature = "board-swift_sfx14_41g"),
        not(feature = "board-inspiron_n5010")
    ))]
    let usb = xhci::UsbKbd::bring_up();
    #[cfg(any(feature = "board-swift_sfx14_41g", feature = "board-inspiron_n5010"))]
    let usb = usb_off::UsbKbd::off();
    #[cfg(not(feature = "board-swift_sfx14_41g"))]
    device.enable_cursor();
    #[cfg(feature = "board-swift_sfx14_41g")]
    device::Device::enable_cursor();
    let screen = Screen::with_banner();
    device.present_all(&screen);
    #[cfg(not(feature = "board-swift_sfx14_41g"))]
    device.sync_cursor(&screen);
    #[cfg(feature = "board-swift_sfx14_41g")]
    device::Device::sync_cursor(&screen);
    // The banner replaces the log lines. Hold it before the shell prints the menu.
    #[cfg(feature = "board-pc_z97_d3h")]
    debug::after_boot_line();
    handler::HandlerImpl::new(device, screen, usb, IRQ, USB_IRQ, SHELL)
}
