//! Paint vendor, device, BAR, and interrupt pin for the six recorded locations.
//!
//! Windows assigned these bus numbers. A line that reads `ffff` is a location
//! this boot did not find. Firmware assigned the NVMe and xHCI base addresses.
//! This domain sets bus master and does not move those addresses.

#![no_std]
#![no_main]

use sel4::with_ipc_buffer_mut;
use sel4_microkit::{
    memory_region_symbol, protection_domain, var, ChannelSet, Handler, Infallible,
};

use lerux_desk::FRAMEBUFFER_BYTES;
use lerux_swift::{
    class_code, config_address, describe, paint_row, store_header, vendor_device, LOCATIONS,
    NVME_BDF, NVME_CLASS, XHCI_BDFS, XHCI_CLASS,
};

const BASE_IOPORT_CAP: u64 = 394;
const ROW: usize = lerux_desk::CONSOLE_ROWS;

struct HandlerImpl;

#[protection_domain]
fn init() -> HandlerImpl {
    let cap_id = *var!(pci_ioport_id: usize = usize::MAX) as u64;
    let addr = *var!(pci_ioport_addr: usize = usize::MAX) as u16;
    let cap = BASE_IOPORT_CAP + cap_id;
    let page = memory_region_symbol!(pci_headers_vaddr: *mut u8).as_ptr();
    let page = unsafe { core::slice::from_raw_parts_mut(page, 0x1000) };
    for (index, (bus, dev, func)) in LOCATIONS.iter().copied().enumerate() {
        let header = read_header(cap, addr, bus, dev, func);
        store_header(page, index, &header);
        let mut fallback = [b' '; 40];
        write_bdf(&mut fallback, bus, dev, func);
        fallback[8..14].copy_from_slice(b"absent");
        let line = describe(bus, dev, func, &header).unwrap_or(fallback);
        paint(ROW + index, &line);
        enable_claimed(cap, addr, bus, dev, func, &header);
    }
    HandlerImpl
}

fn enable_claimed(cap: u64, port: u16, bus: u8, dev: u8, func: u8, header: &[u8]) {
    if vendor_device(header).is_none() {
        return;
    }
    let bdf = (bus, dev, func);
    let nvme = bdf == NVME_BDF && class_code(header) == Some(NVME_CLASS);
    let xhci = XHCI_BDFS.contains(&bdf) && class_code(header) == Some(XHCI_CLASS);
    if nvme || xhci {
        enable_mem(cap, port, bus, dev, func);
    }
}

/// Memory space and bus master. The base address stays the firmware value.
fn enable_mem(cap: u64, port: u16, bus: u8, dev: u8, func: u8) {
    let data = port.wrapping_add(4);
    out32(cap, port, config_address(bus, dev, func, 0x04));
    let command = in32(cap, data);
    out32(cap, port, config_address(bus, dev, func, 0x04));
    out32(cap, data, command | 0x06);
}

fn paint(row: usize, text: &[u8]) {
    let mode = memory_region_symbol!(mode_record_vaddr: *mut u8).as_ptr();
    let frame = memory_region_symbol!(fb_vaddr: *mut u8).as_ptr();
    let record = unsafe { core::slice::from_raw_parts(mode, 40) };
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, FRAMEBUFFER_BYTES as usize) };
    paint_row(record, frame, row, text);
}

fn write_bdf(line: &mut [u8], bus: u8, dev: u8, func: u8) {
    line[0] = hex(bus >> 4);
    line[1] = hex(bus);
    line[2] = b':';
    line[3] = hex(dev >> 4);
    line[4] = hex(dev);
    line[5] = b'.';
    line[6] = b'0' + (func & 0xf);
}

fn hex(nibble: u8) -> u8 {
    let nibble = nibble & 0xf;
    if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + (nibble - 10)
    }
}

fn read_header(cap: u64, addr: u16, bus: u8, dev: u8, func: u8) -> [u8; 64] {
    let mut header = [0u8; 64];
    let data = addr.wrapping_add(4);
    for reg in (0u8..64).step_by(4) {
        out32(cap, addr, config_address(bus, dev, func, reg));
        let word = in32(cap, data);
        let offset = usize::from(reg);
        header[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
    }
    header
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

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, _channels: ChannelSet) -> Result<(), Self::Error> {
        Ok(())
    }
}
