//! I217-V register window. This protection domain owns port `0xcf8`.
//!
//! `CTRL.RST` in the Ethernet driver reloads the BAR and can enable MSI.
//! Init writes the window once, and the driver's protected call writes it again.

use sel4::with_ipc_buffer_mut;

use lerux_machine::e1000e::{
    i217_map, I217_REVISION, I217_SUBSYSTEM, I217_V, REGISTER_BAR, REGISTER_BAR_SIZE,
};

const PCI_VENDOR: u8 = 0x00;
const PCI_REVISION: u8 = 0x08;
const PCI_SUBSYSTEM: u8 = 0x2c;
const PCI_COMMAND: u8 = 0x04;
const PCI_CAP_PTR: u8 = 0x34;
const CMD_MEMORY: u32 = 1 << 1;
const CMD_BUS_MASTER: u32 = 1 << 2;
const CMD_INTX_DISABLE: u32 = 1 << 10;
const CAP_MSI: u8 = 0x05;
const CAP_MSIX: u8 = 0x11;
const MSI_ENABLE: u32 = 1 << 16;
const MSIX_ENABLE: u32 = 1 << 31;

/// Bus address `00:19.0` recorded for the onboard NIC.
const BUS: u8 = 0;
const DEV: u8 = 0x19;
const FUNC: u8 = 0;

struct Pci {
    cap: u64,
    base: u16,
}

impl Pci {
    fn matches_i217(&self) -> bool {
        let id = self.read_config(PCI_VENDOR);
        let revision = self.read_config(PCI_REVISION);
        let subsystem = self.read_config(PCI_SUBSYSTEM);
        let vendor = (id & 0xffff) as u16;
        let device = (id >> 16) as u16;
        let subsystem_vendor = (subsystem & 0xffff) as u16;
        let subsystem_device = (subsystem >> 16) as u16;
        (vendor, device) == I217_V
            && (revision & 0xff) as u8 == I217_REVISION
            && (subsystem_vendor, subsystem_device) == I217_SUBSYSTEM
    }

    fn write_window(&self) -> bool {
        let bar = u32::try_from(i217_map().regs_paddr.value()).expect("regs paddr");
        if bar & 0xf != 0 {
            return false;
        }
        let offset = 0x10 + REGISTER_BAR * 4;
        self.write_config(offset, bar);
        let got = self.read_config(offset);
        let mask = !u32::try_from(REGISTER_BAR_SIZE - 1).expect("bar mask");
        if got & mask != bar & mask {
            return false;
        }
        let cmd = self.read_config(PCI_COMMAND);
        self.write_config(
            PCI_COMMAND,
            (cmd | CMD_MEMORY | CMD_BUS_MASTER) & !CMD_INTX_DISABLE,
        );
        true
    }

    fn force_pin(&self) -> bool {
        let mut cap_ptr = (self.read_config(PCI_CAP_PTR) & 0xff) as u8;
        for _ in 0..48 {
            if cap_ptr == 0 {
                return true;
            }
            let header = self.read_config(cap_ptr);
            let cap_id = (header & 0xff) as u8;
            let next = ((header >> 8) & 0xfc) as u8;
            if cap_id == CAP_MSI {
                self.write_config(cap_ptr, header & !MSI_ENABLE);
            } else if cap_id == CAP_MSIX {
                self.write_config(cap_ptr, header & !MSIX_ENABLE);
            }
            cap_ptr = next;
        }
        false
    }

    fn config_address(reg: u8) -> u32 {
        0x8000_0000
            | (u32::from(BUS) << 16)
            | (u32::from(DEV) << 11)
            | (u32::from(FUNC) << 8)
            | (u32::from(reg) & 0xfc)
    }

    fn read_config(&self, reg: u8) -> u32 {
        self.out32(self.base, Self::config_address(reg));
        self.in32(self.base + 4)
    }

    fn write_config(&self, reg: u8, value: u32) {
        self.out32(self.base, Self::config_address(reg));
        self.out32(self.base + 4, value);
    }

    fn out32(&self, port: u16, value: u32) {
        with_ipc_buffer_mut(|ipc| {
            ipc.inner_mut()
                .seL4_X86_IOPort_Out32(self.cap, u64::from(port), u64::from(value));
        });
    }

    fn in32(&self, port: u16) -> u32 {
        with_ipc_buffer_mut(|ipc| {
            let ret = ipc.inner_mut().seL4_X86_IOPort_In32(self.cap, port);
            ret.result
        })
    }
}

/// Write the recorded BAR and force the legacy pin. False when the port is
/// absent or `00:19.0` is not the recorded I217-V.
///
/// `pci` is the config port xHCI stored. A second `var!` of that symbol does
/// not compile in this crate.
pub fn rewrite(pci: Option<(u64, u16)>) -> bool {
    let Some((cap, base)) = pci else {
        return false;
    };
    let pci = Pci { cap, base };
    if !pci.matches_i217() {
        return false;
    }
    pci.write_window() && pci.force_pin()
}
