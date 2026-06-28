#![no_std]
#![no_main]

use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

#[cfg(feature = "serial-ipc")]
use embedded_hal_nb::serial::Write as _;
#[cfg(feature = "serial-ipc")]
use sel4_microkit_driver_adapters::serial::client::Client as SerialClient;

const MESSAGE: &str = "lerux: Hello from Rust on seL4 Microkit!\n";

#[cfg(feature = "serial-ipc")]
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
    #[cfg(feature = "serial-ipc")]
    {
        let mut serial = SerialClient::new(SERIAL_DRIVER);
        for b in MESSAGE.bytes() {
            serial.write(b).unwrap();
        }
    }

    #[cfg(not(feature = "serial-ipc"))]
    {
        sel4_microkit::debug_println!("{MESSAGE}");
    }
}