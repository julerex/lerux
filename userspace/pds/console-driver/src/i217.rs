//! Program the I217-V window. This domain holds `0xcf8`.
//!
//! Init writes the window before `net_driver` starts. Channel 3 asks for the
//! same write after the MAC reset. That reset reloads the command word and can
//! enable MSI, which masks the pin.

use log::info;

use lerux_machine::e1000e::{i217_map, UnclaimedNic, REGISTER_BAR, REGISTER_BAR_SIZE};

use crate::rtl8852be::{read_dword, write_config};

const PCI_COMMAND: u8 = 0x04;
const PCI_REVISION: u8 = 0x08;
const PCI_SUBSYSTEM: u8 = 0x2c;
const PCI_CAP_PTR: u8 = 0x34;
const CMD_MEMORY: u32 = 1 << 1;
const CMD_BUS_MASTER: u32 = 1 << 2;
const CMD_INTX_DISABLE: u32 = 1 << 10;
const CAP_MSI: u8 = 0x05;
const CAP_MSIX: u8 = 0x11;
const MSI_ENABLE: u32 = 1 << 16;
const MSIX_ENABLE: u32 = 1 << 31;

pub fn program(ports: Option<(u64, u16)>) {
    let Some((cap, addr)) = ports else {
        info!("console-driver: i217 port missing");
        return;
    };
    let nic = UnclaimedNic::i217().address();
    let bus = nic.bus();
    let dev = nic.device();
    let func = nic.function();
    let id = read_dword(cap, addr, bus, dev, func, 0);
    let revision = read_dword(cap, addr, bus, dev, func, PCI_REVISION);
    let subsystem = read_dword(cap, addr, bus, dev, func, PCI_SUBSYSTEM);
    if UnclaimedNic::i217()
        .claim_i217(
            u16::try_from(id & 0xffff).expect("vendor"),
            u16::try_from(id >> 16).expect("device"),
            u8::try_from(revision & 0xff).expect("revision"),
            u16::try_from(subsystem & 0xffff).expect("subsystem vendor"),
            u16::try_from(subsystem >> 16).expect("subsystem device"),
        )
        .is_err()
    {
        info!("console-driver: i217 miss");
        return;
    }
    if !write_window(cap, addr, bus, dev, func) || !force_pin(cap, addr, bus, dev, func) {
        info!("console-driver: i217 err");
        return;
    }
    info!("console-driver: i217 ready");
}

fn write_window(cap: u64, addr: u16, bus: u8, dev: u8, func: u8) -> bool {
    let bar = u32::try_from(i217_map().regs_paddr.value()).expect("regs paddr");
    if bar & 0xf != 0 {
        return false;
    }
    let offset = u8::try_from(0x10 + u32::from(REGISTER_BAR) * 4).expect("bar offset");
    write_config(cap, addr, bus, dev, func, offset, bar);
    let got = read_dword(cap, addr, bus, dev, func, offset);
    let mask = !u32::try_from(REGISTER_BAR_SIZE - 1).expect("bar mask");
    if got & mask != bar & mask {
        return false;
    }
    let cmd = read_dword(cap, addr, bus, dev, func, PCI_COMMAND);
    write_config(
        cap,
        addr,
        bus,
        dev,
        func,
        PCI_COMMAND,
        (cmd | CMD_MEMORY | CMD_BUS_MASTER) & !CMD_INTX_DISABLE,
    );
    true
}

fn force_pin(cap: u64, addr: u16, bus: u8, dev: u8, func: u8) -> bool {
    let mut cap_ptr =
        u8::try_from(read_dword(cap, addr, bus, dev, func, PCI_CAP_PTR) & 0xff).expect("cap ptr");
    for _ in 0..48 {
        if cap_ptr == 0 {
            return true;
        }
        let header = read_dword(cap, addr, bus, dev, func, cap_ptr);
        let cap_id = u8::try_from(header & 0xff).expect("cap id");
        let next = u8::try_from((header >> 8) & 0xfc).expect("next cap");
        if cap_id == CAP_MSI {
            write_config(cap, addr, bus, dev, func, cap_ptr, header & !MSI_ENABLE);
        } else if cap_id == CAP_MSIX {
            write_config(cap, addr, bus, dev, func, cap_ptr, header & !MSIX_ENABLE);
        }
        cap_ptr = next;
    }
    false
}
