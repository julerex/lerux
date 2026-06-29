//! PCI configuration space access via x86 I/O ports 0xcf8/0xcfc.

use sel4::with_ipc_buffer_mut;
use virtio_drivers::transport::pci::bus::{ConfigurationAccess, DeviceFunction};

const BASE_IOPORT_CAP: u64 = 394;
const PCI_CONFIG_ADDR_OFFSET: u16 = 0;
const PCI_CONFIG_DATA_OFFSET: u16 = 4;

#[derive(Clone, Debug)]
pub struct IoPortCam {
    ioport_id: u32,
    base_port: u16,
}

impl IoPortCam {
    pub const fn new(ioport_id: u32, base_port: u16) -> Self {
        Self {
            ioport_id,
            base_port,
        }
    }

    fn ioport_cap(&self) -> u64 {
        BASE_IOPORT_CAP + self.ioport_id as u64
    }

    fn config_address(device: DeviceFunction, register_offset: u8) -> u32 {
        0x8000_0000
            | ((device.bus as u32) << 16)
            | ((device.device as u32) << 11)
            | ((device.function as u32) << 8)
            | (register_offset as u32 & 0xfc)
    }

    fn out32(&self, port: u16, value: u32) {
        with_ipc_buffer_mut(|ipc| {
            ipc.inner_mut()
                .seL4_X86_IOPort_Out32(self.ioport_cap(), port.into(), value as u64);
        });
    }

    fn in32(&self, port: u16) -> u32 {
        with_ipc_buffer_mut(|ipc| {
            let ret = ipc
                .inner_mut()
                .seL4_X86_IOPort_In32(self.ioport_cap(), port);
            ret.result as u32
        })
    }
}

impl ConfigurationAccess for IoPortCam {
    fn read_word(&self, device_function: DeviceFunction, register_offset: u8) -> u32 {
        self.out32(
            self.base_port + PCI_CONFIG_ADDR_OFFSET,
            Self::config_address(device_function, register_offset),
        );
        self.in32(self.base_port + PCI_CONFIG_DATA_OFFSET)
    }

    fn write_word(&mut self, device_function: DeviceFunction, register_offset: u8, data: u32) {
        self.out32(
            self.base_port + PCI_CONFIG_ADDR_OFFSET,
            Self::config_address(device_function, register_offset),
        );
        self.out32(self.base_port + PCI_CONFIG_DATA_OFFSET, data);
    }

    unsafe fn unsafe_clone(&self) -> Self {
        self.clone()
    }
}