//! NS16550 UART over x86 I/O ports (COM1 at 0x3f8 on QEMU PC).

use core::convert::Infallible;

use embedded_hal_nb::{nb, serial};
use sel4::with_ipc_buffer_mut;
use sel4_driver_interfaces::HandleInterrupt;
use sel4_microkit::var;

const BASE_IOPORT_CAP: u64 = 394;

const LSR_THRE: u8 = 0x20;
const LSR_DR: u8 = 0x01;

// IER: enable received-data-available interrupt.
const IER_ERBFI: u8 = 0x01;

// IIR interrupt type: receiver line status (lower 4 bits, when bit 0 clear).
const IIR_RX_LINE_STATUS: u8 = 0x06;

pub struct Driver {
    ioport_id: u32,
    base_port: u16,
}

unsafe impl Send for Driver {}
unsafe impl Sync for Driver {}

impl Driver {
    pub fn new(ioport_id: u32, base_port: u16) -> Self {
        Self {
            ioport_id,
            base_port,
        }
    }

    pub fn from_system_vars() -> Self {
        let ioport_id = *var!(com1_ioport_id: usize = usize::MAX) as u32;
        let base_port = *var!(com1_ioport_addr: usize = usize::MAX) as u16;
        let mut driver = Self::new(ioport_id, base_port);
        driver.init();
        driver
    }

    /// Enable FIFOs and RX interrupts so the driver is notified on input.
    pub fn init(&mut self) {
        // Disable interrupts while configuring.
        self.out8(1, 0);
        // 8N1, no divisor latch.
        self.out8(3, 0x03);
        // Enable and reset FIFOs.
        self.out8(2, 0x01);
        // Enable received-data-available interrupts.
        self.out8(1, IER_ERBFI);
    }

    fn ioport_cap(&self) -> u64 {
        BASE_IOPORT_CAP + self.ioport_id as u64
    }

    fn out8(&self, offset: u16, value: u8) {
        let port = self.base_port + offset;
        with_ipc_buffer_mut(|ipc| {
            ipc.inner_mut()
                .seL4_X86_IOPort_Out8(self.ioport_cap(), port as u64, value as u64);
        });
    }

    fn in8(&self, offset: u16) -> u8 {
        let port = self.base_port + offset;
        with_ipc_buffer_mut(|ipc| {
            let ret = ipc.inner_mut().seL4_X86_IOPort_In8(self.ioport_cap(), port);
            ret.result
        })
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
        // Acknowledge UART interrupts. HandlerImpl drains RBR via read() before calling
        // this; a receiver line-status interrupt still requires an LSR read to deassert IRQ.
        if (self.in8(2) & 0x0f) == IIR_RX_LINE_STATUS {
            let _ = self.lsr();
        }
    }
}
