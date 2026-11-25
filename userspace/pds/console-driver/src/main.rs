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
#[cfg(all(not(test), feature = "board-pc_z97_d3h_i217"))]
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
/// The I217 image assigns both IRQs before links, so the shell is channel 2
/// and the xHCI MSI is channel 1. Other images keep the desk template's ids.
#[cfg(all(not(test), feature = "hardware", feature = "board-pc_z97_d3h_i217"))]
const SHELL: Channel = Channel::new(2);
#[cfg(all(not(test), feature = "hardware", feature = "board-pc_z97_d3h_i217"))]
const USB_IRQ: Channel = Channel::new(1);
#[cfg(all(not(test), feature = "hardware", feature = "board-pc_z97_d3h_i217"))]
const I217_WINDOW: Channel = Channel::new(3);
#[cfg(all(
    not(test),
    feature = "hardware",
    not(feature = "board-pc_z97_d3h_i217")
))]
const SHELL: Channel = Channel::new(1);
#[cfg(all(
    not(test),
    feature = "hardware",
    not(feature = "board-pc_z97_d3h_i217")
))]
const USB_IRQ: Channel = Channel::new(2);

#[cfg(all(not(test), feature = "hardware"))]
#[protection_domain]
fn init() -> handler::HandlerImpl {
    use lerux_logging::{debug, log};

    #[cfg(feature = "board-pc_z97_d3h")]
    boot_line::install();
    debug::init().expect("debug log");
    let device = device::Device::from_system();
    #[cfg(feature = "board-pc_z97_d3h_i217")]
    if device.paints_gop() {
        log::info!("console-driver: gop");
    } else {
        log::info!("console-driver: vga text");
    }
    #[cfg(all(
        not(feature = "board-swift_sfx14_41g"),
        not(feature = "board-pc_z97_d3h_i217")
    ))]
    log::info!("console-driver: vga text");
    #[cfg(feature = "board-swift_sfx14_41g")]
    if device.uses_panel() {
        log::info!("console-driver: panel");
    } else {
        log::info!("console-driver: vga text");
    }
    if device.init_keyboard() {
        log::info!("console-driver: i8042 ready");
    } else {
        log::info!("console-driver: i8042 timeout");
    }
    open_shell(device)
}

#[cfg(all(not(test), feature = "hardware"))]
fn open_shell(device: device::Device) -> handler::HandlerImpl {
    use crate::screen::Screen;

    #[cfg(all(
        not(feature = "board-swift_sfx14_41g"),
        not(feature = "board-inspiron_n5010")
    ))]
    let usb = xhci::UsbKbd::bring_up();
    #[cfg(feature = "board-pc_z97_d3h")]
    rtl8852be::publish(usb.pci_config());
    #[cfg(feature = "board-pc_z97_d3h_i217")]
    note_i217(&usb);
    #[cfg(any(feature = "board-swift_sfx14_41g", feature = "board-inspiron_n5010"))]
    let usb = usb_off::UsbKbd::off();
    device.enable_cursor();
    #[cfg(not(feature = "board-swift_sfx14_41g"))]
    let screen = Screen::blank();
    #[cfg(feature = "board-swift_sfx14_41g")]
    let screen = if device.uses_panel() {
        Screen::blank()
    } else {
        Screen::vga()
    };
    device.present_all(&screen);
    device.sync_cursor(&screen);
    // The blank page replaces the log lines. Hold it before the shell prints the sign-on.
    #[cfg(feature = "board-pc_z97_d3h")]
    lerux_logging::debug::after_boot_line();
    handler::HandlerImpl::new(
        device,
        screen,
        usb,
        IRQ,
        USB_IRQ,
        SHELL,
        #[cfg(feature = "board-pc_z97_d3h_i217")]
        I217_WINDOW,
    )
}

#[cfg(all(not(test), feature = "board-pc_z97_d3h_i217"))]
fn note_i217(usb: &xhci::UsbKbd) {
    use lerux_logging::log;
    if i217::rewrite(usb.pci_config()) {
        log::info!("console-driver: i217 window");
    } else {
        log::info!("console-driver: i217 absent");
    }
}
