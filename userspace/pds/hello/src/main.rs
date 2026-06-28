#![no_std]
#![no_main]

use embedded_hal_nb::serial::Write as _;
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};
use sel4_microkit_driver_adapters::serial::client::Client as SerialClient;

const SERIAL_DRIVER: Channel = Channel::new(0);

const MESSAGE: &str = "lerux: Hello from Rust on seL4 Microkit!\n";

#[protection_domain]
fn init() -> HandlerImpl {
    let mut serial = SerialClient::new(SERIAL_DRIVER);
    write_all(&mut serial, MESSAGE);
    HandlerImpl
}

struct HandlerImpl;

impl Handler for HandlerImpl {
    type Error = Infallible;
}

fn write_all(serial: &mut SerialClient, s: &str) {
    for b in s.bytes() {
        serial.write(b).unwrap();
    }
}