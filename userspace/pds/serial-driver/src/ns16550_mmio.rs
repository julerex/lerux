//! NS16550 UART over MMIO (QEMU virt RISC-V at 0x1000_0000).

use core::convert::Infallible;
use core::ptr::{read_volatile, write_volatile};

use embedded_hal_nb::nb;
use embedded_hal_nb::serial;
use sel4_driver_interfaces::HandleInterrupt;
use sel4_microkit::memory_region_symbol;

const LSR_THRE: u8 = 0x20;
const LSR_DR: u8 = 0x01;

// IER: enable received-data-available interrupt.
const IER_ERBFI: u8 = 0x01;

// IIR interrupt type: receiver line status (lower 4 bits, when bit 0 clear).
const IIR_RX_LINE_STATUS: u8 = 0x06;

pub struct Driver {
    base: *mut u8,
}

unsafe impl Send for Driver {}
unsafe impl Sync for Driver {}

impl Driver {
    pub fn new(base: *mut u8) -> Self {
        let mut driver = Self { base };
        driver.init();
        driver
    }

    pub fn from_mmio() -> Self {
        Self::new(memory_region_symbol!(serial_register_block: *mut u8).as_ptr())
    }

    /// Enable FIFOs and RX interrupts so the driver is notified on input.
    pub fn init(&mut self) {
        self.out8(1, 0);
        self.out8(3, 0x03);
        self.out8(2, 0x01);
        self.out8(1, IER_ERBFI);
    }

    fn out8(&self, offset: usize, value: u8) {
        unsafe {
            write_volatile(self.base.add(offset), value);
        }
    }

    fn in8(&self, offset: usize) -> u8 {
        unsafe { read_volatile(self.base.add(offset)) }
    }

    fn lsr(&self) -> u8 {
        self.in8(5)
    }
}

impl serial::ErrorType for Driver {
    type Error = Infallible;
}

impl serial::Read for Driver {
    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        if self.lsr() & LSR_DR != 0 {
            Ok(self.in8(0))
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl serial::Write for Driver {
    fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
        while self.lsr() & LSR_THRE == 0 {
            core::hint::spin_loop();
        }
        self.out8(0, byte);
        Ok(())
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        Ok(())
    }
}

impl HandleInterrupt for Driver {
    fn handle_interrupt(&mut self) {
        if (self.in8(2) & 0x0f) == IIR_RX_LINE_STATUS {
            let _ = self.lsr();
        }
    }
}