#![no_std]
#![no_main]

use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Channel, Handler};
use sel4_microkit_driver_adapters::serial::driver::HandlerImpl;

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

// Channel 1: IPC to the client PD (matches <end pd="serial_driver" id="1">).
const CLIENT: Channel = Channel::new(1);

// Channel 0: IRQ notification (<irq id="0">).
const DEVICE: Channel = Channel::new(0);

#[cfg(feature = "board-qemu_virt_aarch64")]
#[protection_domain]
fn init() -> impl Handler {
    debug::init().unwrap();
    log::info!("serial driver: PL011");
    let driver =
        unsafe { Pl011Driver::new(memory_region_symbol!(serial_register_block: *mut ()).as_ptr()) };
    HandlerImpl::new(driver, DEVICE, CLIENT)
}

#[cfg(feature = "board-x86_64_generic")]
#[protection_domain]
fn init() -> impl Handler {
    debug::init().unwrap();
    log::info!("serial driver: NS16550 COM1 (IRQ RX)");
    let driver = Ns16550Driver::from_system_vars();
    HandlerImpl::new(driver, DEVICE, CLIENT)
}

#[cfg(feature = "board-qemu_virt_riscv64")]
#[protection_domain]
fn init() -> impl Handler {
    debug::init().unwrap();
    log::info!("serial driver: NS16550 MMIO (IRQ RX)");
    let driver = Ns16550MmioDriver::from_mmio();
    HandlerImpl::new(driver, DEVICE, CLIENT)
}
