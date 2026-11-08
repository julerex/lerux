//! Paint vendor, device, BAR, and interrupt pin for the six recorded locations.
//!
//! Windows assigned these bus numbers. A line that reads `ffff` is a location
//! this boot did not find. Firmware assigned the `NVMe`, xHCI, and MT7921 base
//! addresses. This domain sets bus master and does not move those addresses.
//! Row 15 reads the MT7921 command word back after bus master is set.
//! Row 16 reads the exclusion range back after that programming.

#![no_std]
#![no_main]

use sel4::with_ipc_buffer_mut;
use sel4_microkit::{
    memory_region_symbol, protection_domain, var, ChannelSet, Handler, Infallible,
};

use lerux_desk::FRAMEBUFFER_BYTES;
use lerux_swift::{
    class_code, config_address, describe, exclusion_base, exclusion_limit, iommu_base,
    iommu_with_bit, paint_row, store_header, store_iommu, vendor_device, write_cmd_line,
    write_excl_line, IOMMU_CTRL, IOMMU_EXCL_BASE, IOMMU_EXCL_LIMIT, IOMMU_OFF_BITS, LOCATIONS,
    MT7921_BDF, MT7921_IDS, NVME_BDF, NVME_CLASS, SWIFT_DMA_END, SWIFT_DMA_START, XHCI_BDFS,
    XHCI_CLASS,
};

const BASE_IOPORT_CAP: u64 = 394;
const ROW: usize = lerux_desk::CONSOLE_ROWS;
const ROW_CMD: usize = 15;
const ROW_EXCL: usize = 16;

struct HandlerImpl;

#[protection_domain]
fn init() -> HandlerImpl {
    let cap_id = *var!(pci_ioport_id: usize = usize::MAX) as u64;
    let addr =
        u16::try_from(*var!(pci_ioport_addr: usize = usize::MAX) & 0xffff).expect("low half");
    let cap = BASE_IOPORT_CAP + cap_id;
    let page = memory_region_symbol!(pci_headers_vaddr: *mut u8).as_ptr();
    let page = unsafe { core::slice::from_raw_parts_mut(page, 0x1000) };
    let mut wifi_cmd = None;
    for (index, (bus, dev, func)) in LOCATIONS.iter().copied().enumerate() {
        let header = read_header(cap, addr, bus, dev, func);
        store_header(page, index, &header);
        let mut fallback = [b' '; 40];
        write_bdf(&mut fallback, bus, dev, func);
        fallback[8..14].copy_from_slice(b"absent");
        let line = describe(bus, dev, func, &header).unwrap_or(fallback);
        paint(ROW + index, &line);
        enable_claimed(cap, addr, bus, dev, func, &header);
        if (bus, dev, func) == MT7921_BDF {
            wifi_cmd = command_word(cap, addr, bus, dev, func, &header);
        }
    }
    let mut line = [b' '; 8];
    let n = write_cmd_line(&mut line, wifi_cmd);
    paint(ROW_CMD, &line[..n.max(8)]);
    let base = find_iommu(cap, addr);
    store_iommu(page, base);
    let seen = pass_dma(base);
    let mut line = [b' '; 28];
    let n = write_excl_line(&mut line, seen);
    paint(ROW_EXCL, &line[..n.max(26)]);
    HandlerImpl
}

/// `0xfdf00000` is the base this Swift painted. Translation stays on for the
/// display. The `NVMe`, `xHCI`, and MT7921 windows are excluded so those DMA
/// addresses are used as physical.
const IOMMU_PHYS: u64 = 0xfdf0_0000;

#[expect(
    clippy::cast_ptr_alignment,
    reason = "the IOMMU page is 4 KiB aligned and these offsets are multiples of 8"
)]
fn pass_dma(base: u64) -> Option<(u64, u64, u64)> {
    if base != IOMMU_PHYS {
        return None;
    }
    let regs = memory_region_symbol!(iommu_regs_vaddr: *mut u8).as_ptr();
    // Safety: the template maps the AMD IOMMU MMIO here, uncached. The control
    // register is the aligned 8-byte word at offset 0x18. A read of all ones is
    // not a register, and a clear enable bit means translation is already off.
    unsafe {
        let ctrl_ptr = regs.add(IOMMU_CTRL).cast::<u64>();
        let excl = regs.add(IOMMU_EXCL_BASE).cast::<u64>();
        let limit = regs.add(IOMMU_EXCL_LIMIT).cast::<u64>();
        let ctrl = ctrl_ptr.read_volatile();
        if ctrl == u64::MAX {
            return None;
        }
        if ctrl & 1 != 0 {
            for bit in IOMMU_OFF_BITS {
                let current = ctrl_ptr.read_volatile();
                ctrl_ptr.write_volatile(iommu_with_bit(current, bit, false));
            }
            excl.write_volatile(exclusion_base(SWIFT_DMA_START));
            limit.write_volatile(exclusion_limit(SWIFT_DMA_END));
            let current = ctrl_ptr.read_volatile();
            ctrl_ptr.write_volatile(iommu_with_bit(current, 0, true));
            let _ = ctrl_ptr.read_volatile();
        }
        Some((
            excl.read_volatile(),
            limit.read_volatile(),
            ctrl_ptr.read_volatile(),
        ))
    }
}

/// Bus 0 function `00:00.2` is the Cezanne IOMMU. The scan covers the rest of
/// bus 0 when that function is not class `080600`.
fn find_iommu(cap: u64, port: u16) -> u64 {
    for dev in 0..32u8 {
        for func in 0..8u8 {
            if let Some(base) = read_iommu(cap, port, dev, func) {
                return base;
            }
        }
    }
    0
}

fn read_iommu(cap: u64, port: u16, dev: u8, func: u8) -> Option<u64> {
    let data = port.wrapping_add(4);
    out32(cap, port, config_address(0, dev, func, 0));
    let id = in32(cap, data);
    if id & 0xffff != 0x1022 {
        return None;
    }
    out32(cap, port, config_address(0, dev, func, 0x08));
    if in32(cap, data) >> 8 != 0x0008_0600 {
        return None;
    }
    let mut config = [0u8; 256];
    for reg in (0u8..=252).step_by(4) {
        out32(cap, port, config_address(0, dev, func, reg));
        let word = in32(cap, data);
        let offset = usize::from(reg);
        config[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
    }
    iommu_base(&config)
}

fn enable_claimed(cap: u64, port: u16, bus: u8, dev: u8, func: u8, header: &[u8]) {
    if vendor_device(header).is_none() {
        return;
    }
    let bdf = (bus, dev, func);
    let nvme = bdf == NVME_BDF && class_code(header) == Some(NVME_CLASS);
    let xhci = XHCI_BDFS.contains(&bdf) && class_code(header) == Some(XHCI_CLASS);
    let wifi =
        bdf == MT7921_BDF && vendor_device(header).is_some_and(|id| MT7921_IDS.contains(&id));
    if nvme || xhci || wifi {
        enable_mem(cap, port, bus, dev, func);
    }
}

/// Low 16 bits at offset `0x04`, read after [`enable_mem`] on the MT7921.
/// `None` is `cmd miss`: another function, or a read of all ones.
fn command_word(cap: u64, port: u16, bus: u8, dev: u8, func: u8, header: &[u8]) -> Option<u16> {
    let wifi = (bus, dev, func) == MT7921_BDF
        && vendor_device(header).is_some_and(|id| MT7921_IDS.contains(&id));
    if !wifi {
        return None;
    }
    let data = port.wrapping_add(4);
    out32(cap, port, config_address(bus, dev, func, 0x04));
    let word = in32(cap, data);
    if word == u32::MAX {
        return None;
    }
    Some(u16::try_from(word & 0xffff).expect("low half"))
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
    let bytes = usize::try_from(FRAMEBUFFER_BYTES).expect("framebuffer fits");
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, bytes) };
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
