//! NS16550 UART over x86 I/O ports (COM1 at 0x3f8 on QEMU PC).

use core::convert::Infallible;

use embedded_hal_nb::nb;
use embedded_hal_nb::serial;
use sel4::with_ipc_buffer_mut;
use sel4_driver_interfaces::HandleInterrupt;
use sel4_microkit::var;

const BASE_IOPORT_CAP: u64 = 394;

const LSR_THRE: u8 = 0x20;
const LSR_DR: u8 = 0x01;

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
        Self::new(ioport_id, base_port)
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
            let ret = ipc
                .inner_mut()
                .seL4_X86_IOPort_In8(self.ioport_cap(), port);
            ret.result as u8
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
    fn handle_interrupt(&mut self) {}
}

