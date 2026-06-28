#![no_std]
#![no_main]

use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

#[cfg(feature = "board-qemu_virt_aarch64")]
use embedded_hal_nb::serial::Write as _;
#[cfg(feature = "board-qemu_virt_aarch64")]
use sel4_microkit_driver_adapters::serial::client::Client as SerialClient;

const MESSAGE: &str = "lerux: Hello from Rust on seL4 Microkit!\n";

#[cfg(feature = "board-qemu_virt_aarch64")]
const SERIAL_DRIVER: Channel = Channel::new(0);

#[protection_domain]
fn init() -> HandlerImpl {
    write_message();
    HandlerImpl
}

struct HandlerImpl;

impl Handler for HandlerImpl {
    type Error = Infallible;
}

fn write_message() {
    #[cfg(feature = "board-qemu_virt_aarch64")]
    {
        let mut serial = SerialClient::new(SERIAL_DRIVER);
        for b in MESSAGE.bytes() {
            serial.write(b).unwrap();
        }
    }

    #[cfg(not(feature = "board-qemu_virt_aarch64"))]
    {
        sel4_microkit::debug_println!("{MESSAGE}");
    }
}