//! VGA text console and keyboard for the on-screen shell.
//!
//! Channel 0 is the PS/2 IRQ. Channel 1 is the shell, speaking the serial byte
//! protocol. On the desk, channel 2 is the xHCI MSI, channel 3 is the I217
//! window rewrite, and COM1 stays with `serial-driver`. The Inspiron image
//! has neither.

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
#[cfg(all(not(test), feature = "board-pc_z97_d3h"))]
mod i217;
#[cfg(all(not(test), feature = "board-pc_z97_d3h"))]
mod rtl8852be;
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

    #[cfg(feature = "board-pc_z97_d3h")]
    boot_line::install();
    debug::init().expect("debug log");
    let device = device::Device::from_system();
    log_surface(&device);
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
    #[cfg(feature = "board-pc_z97_d3h")]
    rtl8852be::publish(usb.pci_config());
    #[cfg(feature = "board-pc_z97_d3h")]
    i217::program(usb.pci_config());
    #[cfg(any(feature = "board-swift_sfx14_41g", feature = "board-inspiron_n5010"))]
    let usb = usb_off::UsbKbd::off();
    device.enable_cursor();
    let screen = open_screen(&device);
    device.present_all(&screen);
    device.sync_cursor(&screen);
    // The blank page replaces the log lines. Hold it before the shell prints the sign-on.
    #[cfg(feature = "board-pc_z97_d3h")]
    debug::after_boot_line();
    handler::HandlerImpl::new(device, screen, usb, IRQ, USB_IRQ, SHELL)
}

#[cfg(all(not(test), feature = "hardware", feature = "board-swift_sfx14_41g"))]
fn log_surface(device: &device::Device) {
    use lerux_logging::log;

    if device.uses_panel() {
        log::info!("console-driver: panel");
    } else {
        log::info!("console-driver: vga text");
    }
}

#[cfg(all(
    not(test),
    feature = "hardware",
    not(feature = "board-swift_sfx14_41g")
))]
fn log_surface(_device: &device::Device) {
    use lerux_logging::log;

    log::info!("console-driver: vga text");
}

#[cfg(all(not(test), feature = "hardware", feature = "board-swift_sfx14_41g"))]
fn open_screen(device: &device::Device) -> screen::Screen {
    if device.uses_panel() {
        screen::Screen::blank()
    } else {
        screen::Screen::vga()
    }
}

#[cfg(all(
    not(test),
    feature = "hardware",
    not(feature = "board-swift_sfx14_41g")
))]
fn open_screen(_device: &device::Device) -> screen::Screen {
    screen::Screen::blank()
}
