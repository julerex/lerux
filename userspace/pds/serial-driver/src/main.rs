#![no_std]
#![no_main]

mod handler;

use handler::HandlerImpl;
use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Channel, Handler};

#[cfg(feature = "board-qemu_virt_aarch64")]
use sel4_microkit::memory_region_symbol;
#[cfg(feature = "board-qemu_virt_aarch64")]
use sel4_pl011_driver::Driver as Pl011Driver;

#[cfg(feature = "board-x86_64_generic")]
mod ns16550;
#[cfg(feature = "board-x86_64_generic")]
use ns16550::Driver as Ns16550Driver;

#[cfg(feature = "board-qemu_virt_riscv64")]
mod ns16550_mmio;
#[cfg(feature = "board-qemu_virt_riscv64")]
use ns16550_mmio::Driver as Ns16550MmioDriver;

// Channel 0: IRQ notification (<irq id="0">).
const DEVICE: Channel = Channel::new(0);

#[cfg(not(feature = "multi-client-2"))]
const CLIENTS: [Channel; 1] = [Channel::new(1)];

#[cfg(feature = "multi-client-2")]
const CLIENTS: [Channel; 2] = [Channel::new(1), Channel::new(2)];

#[cfg(not(feature = "multi-client-2"))]
type SerialHandler<D> = HandlerImpl<D, 1>;

#[cfg(feature = "multi-client-2")]
type SerialHandler<D> = HandlerImpl<D, 2>;

#[cfg(feature = "board-qemu_virt_aarch64")]
#[protection_domain]
fn init() -> impl Handler {
    debug::init().unwrap();
    log::info!("serial driver: PL011");
    let driver =
        unsafe { Pl011Driver::new(memory_region_symbol!(serial_register_block: *mut ()).as_ptr()) };
    SerialHandler::new(driver, DEVICE, CLIENTS)
}

#[cfg(feature = "board-x86_64_generic")]
#[protection_domain]
fn init() -> impl Handler {
    debug::init().unwrap();
    log::info!("serial driver: NS16550 COM1 (IRQ RX)");
    let driver = Ns16550Driver::from_system_vars();
    SerialHandler::new(driver, DEVICE, CLIENTS)
}

#[cfg(feature = "board-qemu_virt_riscv64")]
#[protection_domain]
fn init() -> impl Handler {
    debug::init().unwrap();
    log::info!("serial driver: NS16550 MMIO (IRQ RX)");
    let driver = Ns16550MmioDriver::from_mmio();
    SerialHandler::new(driver, DEVICE, CLIENTS)
}
