//! PCIe ECAM access via a minimally mapped MMIO window.

use virtio_drivers::transport::pci::bus::{Cam, ConfigurationAccess, DeviceFunction};

#[derive(Clone, Copy)]
pub struct EcamAccess {
    base_vaddr: usize,
}

impl EcamAccess {
    pub const fn new(base_vaddr: usize) -> Self {
        Self { base_vaddr }
    }

    fn config_offset(device_function: DeviceFunction, register_offset: u8) -> usize {
        Cam::Ecam.cam_offset(device_function, register_offset) as usize
    }
}

impl ConfigurationAccess for EcamAccess {
    fn read_word(&self, device_function: DeviceFunction, register_offset: u8) -> u32 {
        let address = Self::config_offset(device_function, register_offset);
        unsafe { core::ptr::read_volatile((self.base_vaddr + address) as *const u32) }
    }

    fn write_word(&mut self, device_function: DeviceFunction, register_offset: u8, data: u32) {
        let address = Self::config_offset(device_function, register_offset);
        unsafe { core::ptr::write_volatile((self.base_vaddr + address) as *mut u32, data) }
    }

    unsafe fn unsafe_clone(&self) -> Self {
        *self
    }
}