#![no_std]
#![no_main]

#[cfg(not(feature = "device-only"))]
mod handler;

#[cfg(feature = "device-only")]
mod device;

#[cfg(not(feature = "device-only"))]
use handler::HandlerImpl;
use lerux_logging::{debug, log};
#[cfg(not(feature = "device-only"))]
use sel4_microkit::Channel;
use sel4_microkit::{protection_domain, Handler};

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
#[cfg(not(feature = "device-only"))]
const DEVICE: Channel = Channel::new(0);

#[cfg(not(any(
    feature = "multi-client-2",
    feature = "multi-client-3",
    feature = "device-only"
)))]
const CLIENTS: [Channel; 1] = [Channel::new(1)];

#[cfg(all(feature = "multi-client-2", not(feature = "device-only")))]
const CLIENTS: [Channel; 2] = [Channel::new(1), Channel::new(2)];

#[cfg(all(feature = "multi-client-3", not(feature = "device-only")))]
const CLIENTS: [Channel; 3] = [Channel::new(1), Channel::new(2), Channel::new(3)];

#[cfg(not(any(
    feature = "multi-client-2",
    feature = "multi-client-3",
    feature = "device-only"
)))]
type SerialHandler<D> = HandlerImpl<D, 1>;

#[cfg(all(feature = "multi-client-2", not(feature = "device-only")))]
type SerialHandler<D> = HandlerImpl<D, 2>;

#[cfg(all(feature = "multi-client-3", not(feature = "device-only")))]
type SerialHandler<D> = HandlerImpl<D, 3>;

#[cfg(all(feature = "board-qemu_virt_aarch64", not(feature = "device-only")))]
#[protection_domain]
fn init() -> impl Handler {
    debug::init().unwrap();
    log::info!("serial driver: PL011");
    let driver =
        unsafe { Pl011Driver::new(memory_region_symbol!(serial_register_block: *mut ()).as_ptr()) };
    SerialHandler::new(driver, DEVICE, CLIENTS)
}

#[cfg(all(feature = "board-qemu_virt_aarch64", feature = "device-only"))]
#[protection_domain]
fn init() -> impl Handler {
    debug::init().unwrap();
    log::info!("serial driver: PL011 (device-only → serial-virt)");
    let driver =
        unsafe { Pl011Driver::new(memory_region_symbol!(serial_register_block: *mut ()).as_ptr()) };
    device::DeviceHandler::new(driver)
}

#[cfg(all(feature = "board-x86_64_generic", not(feature = "device-only")))]
#[protection_domain]
fn init() -> impl Handler {
    debug::init().unwrap();
    log::info!("serial driver: NS16550 COM1 (IRQ RX)");
    let driver = Ns16550Driver::from_system_vars();
    SerialHandler::new(driver, DEVICE, CLIENTS)
}

#[cfg(all(feature = "board-x86_64_generic", feature = "device-only"))]
#[protection_domain]
fn init() -> impl Handler {
    debug::init().unwrap();
    log::info!("serial driver: NS16550 COM1 (device-only → serial-virt)");
    let driver = Ns16550Driver::from_system_vars();
    device::DeviceHandler::new(driver)
}

#[cfg(all(feature = "board-qemu_virt_riscv64", not(feature = "device-only")))]
#[protection_domain]
fn init() -> impl Handler {
    debug::init().unwrap();
    log::info!("serial driver: NS16550 MMIO (IRQ RX)");
    let driver = Ns16550MmioDriver::from_mmio();
    SerialHandler::new(driver, DEVICE, CLIENTS)
}

#[cfg(all(feature = "board-qemu_virt_riscv64", feature = "device-only"))]
#[protection_domain]
fn init() -> impl Handler {
    debug::init().unwrap();
    log::info!("serial driver: NS16550 MMIO (device-only → serial-virt)");
    let driver = Ns16550MmioDriver::from_mmio();
    device::DeviceHandler::new(driver)
}
