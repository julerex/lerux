//! Copy the RTL8852BE headers and enable memory space on the endpoint.
//!
//! `console_driver` holds `0xcf8`. The radio reads the copied headers and does
//! not get a port.

use sel4::with_ipc_buffer_mut;
use sel4_microkit::memory_region_symbol;

use lerux_rtw8852be::{card_state, decode_bridge, CardState, MAPPED_MMIO_PHYS};

const ROOT_BUS: u8 = 0;
const ROOT_DEV: u8 = 0x1c;
const ROOT_FUNC: u8 = 6;

pub fn publish(ports: Option<(u64, u16)>) {
    let Some((cap, addr)) = ports else {
        return;
    };
    let root = read_config(cap, addr, ROOT_BUS, ROOT_DEV, ROOT_FUNC);
    let Ok(bridge) = decode_bridge(&root) else {
        return;
    };
    let endpoint = read_config(cap, addr, bridge.secondary_bus, 0, 0);
    store_headers(&root, &endpoint);
    if let CardState::Programmed(programmed) = card_state(&root, &endpoint, MAPPED_MMIO_PHYS) {
        write_config(
            cap,
            addr,
            bridge.secondary_bus,
            0,
            0,
            0x04,
            u32::from(programmed.command),
        );
    }
}

fn store_headers(root: &[u8; 64], endpoint: &[u8; 64]) {
    let page = memory_region_symbol!(rtl8852be_cfg_vaddr: *mut u8).as_ptr();
    // SAFETY: the desk template maps a 4 KiB config page at this symbol. The
    // two headers occupy the first 128 bytes. This domain is the only writer.
    unsafe {
        store_volatile(page, root);
        store_volatile(page.add(64), endpoint);
    }
}

/// # Safety
///
/// `dst` must be writable for `src.len()` bytes.
unsafe fn store_volatile(dst: *mut u8, src: &[u8]) {
    for (i, byte) in src.iter().enumerate() {
        unsafe { dst.add(i).write_volatile(*byte) }
    }
}

fn read_config(cap: u64, addr: u16, bus: u8, dev: u8, func: u8) -> [u8; 64] {
    let mut header = [0u8; 64];
    for reg in (0u8..64).step_by(4) {
        let word = read_dword(cap, addr, bus, dev, func, reg);
        let offset = usize::from(reg);
        header[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
    }
    header
}

pub(crate) fn read_dword(cap: u64, addr: u16, bus: u8, dev: u8, func: u8, reg: u8) -> u32 {
    out32(cap, addr, config_address(bus, dev, func, reg));
    in32(cap, addr + 4)
}

pub(crate) fn write_config(cap: u64, addr: u16, bus: u8, dev: u8, func: u8, reg: u8, value: u32) {
    out32(cap, addr, config_address(bus, dev, func, reg));
    out32(cap, addr + 4, value);
}

fn config_address(bus: u8, dev: u8, func: u8, reg: u8) -> u32 {
    0x8000_0000
        | (u32::from(bus) << 16)
        | (u32::from(dev) << 11)
        | (u32::from(func) << 8)
        | (u32::from(reg) & 0xfc)
}

fn out32(cap: u64, port: u16, value: u32) {
    with_ipc_buffer_mut(|ipc| {
        ipc.inner_mut()
            .seL4_X86_IOPort_Out32(cap, u64::from(port), u64::from(value));
    });
}

fn in32(cap: u64, port: u16) -> u32 {
    with_ipc_buffer_mut(|ipc| {
        let ret = ipc.inner_mut().seL4_X86_IOPort_In32(cap, port);
        ret.result
    })
}
