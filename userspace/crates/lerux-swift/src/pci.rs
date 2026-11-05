//! Windows recorded these bus numbers. A metal probe replaces them if they differ.

/// `01:00.0` NVIDIA, `02:00.0` `NVMe`, `03:00.0` MT7921, `04:00.0` Radeon,
/// `04:00.3` and `04:00.4` AMD xHCI.
pub const LOCATIONS: [(u8, u8, u8); 6] = [
    (1, 0, 0),
    (2, 0, 0),
    (3, 0, 0),
    (4, 0, 0),
    (4, 0, 3),
    (4, 0, 4),
];

pub const MT7921_BDF: (u8, u8, u8) = (3, 0, 0);
/// Panel line `mt7921: bar 000000fc82000000`. The low dword is `8200000c`.
pub const MT7921_BAR: u64 = 0x00FC_8200_0000;
pub const NVME_BDF: (u8, u8, u8) = (2, 0, 0);
pub const XHCI_BDFS: [(u8, u8, u8); 2] = [(4, 0, 3), (4, 0, 4)];

pub const MT7921_IDS: [(u16, u16); 2] = [(0x14c3, 0x7961), (0x14c3, 0x0608)];
pub const NVME_CLASS: u32 = 0x0001_0802;
pub const XHCI_CLASS: u32 = 0x000c_0330;

/// One PCI configuration header. Six of these fill the shared page.
pub const HEADER_BYTES: usize = 64;

/// Eight bytes in the shared header page, past the six headers.
pub const IOMMU_RECORD: usize = 0x800;

/// AMD IOMMU control register, and the exclusion base and limit beside it.
pub const IOMMU_CTRL: usize = 0x18;
pub const IOMMU_EXCL_BASE: usize = 0x20;
pub const IOMMU_EXCL_LIMIT: usize = 0x28;

/// Bit 0 of the exclusion base. Linux `iommu_set_exclusion_range` sets this and
/// leaves the allow bit clear. Addresses in the range are not translated.
pub const IOMMU_EXCL_ENABLE: u64 = 1;

/// Control bits Linux `iommu_disable` clears, in that order.
pub const IOMMU_OFF_BITS: [u32; 8] = [12, 3, 2, 28, 29, 13, 14, 0];

/// `NVMe` through the MT7921 DMA windows, including the `xHCI` window between them.
pub const SWIFT_DMA_START: u64 = 0x01f0_0000;
pub const SWIFT_DMA_END: u64 = 0x01fa_0000;

/// Set or clear one control bit. The other bits stay.
#[must_use]
pub fn iommu_with_bit(ctrl: u64, bit: u32, on: bool) -> u64 {
    let mask = 1u64 << bit;
    if on {
        ctrl | mask
    } else {
        ctrl & !mask
    }
}

/// Exclusion base for a page-aligned start, with translation bypassed.
#[must_use]
pub fn exclusion_base(start: u64) -> u64 {
    (start & !0xfff) | IOMMU_EXCL_ENABLE
}

/// Last page of an exclusive end, the value Linux writes to the limit register.
#[must_use]
pub fn exclusion_limit(end: u64) -> u64 {
    (end - 1) & !0xfff
}

/// The AMD IOMMU lives at PCI class `080600` and capability id `0x0f`.
/// The base address registers sit at offsets 4 and 8 in that capability.
#[must_use]
pub fn iommu_base(config: &[u8]) -> Option<u64> {
    if config.len() < 0x40 || config[0x0b] != 0x08 || config[0x0a] != 0x06 || config[0x09] != 0x00 {
        return None;
    }
    let status = u16::from_le_bytes([config[0x06], config[0x07]]);
    if status & 0x10 == 0 {
        return None;
    }
    let mut ptr = usize::from(config[0x34] & !0x3);
    for _ in 0..48 {
        if ptr < 0x40 || ptr + 12 > config.len() {
            return None;
        }
        if config[ptr] == 0x0f {
            let lo = u32::from_le_bytes([
                config[ptr + 4],
                config[ptr + 5],
                config[ptr + 6],
                config[ptr + 7],
            ]);
            let hi = u32::from_le_bytes([
                config[ptr + 8],
                config[ptr + 9],
                config[ptr + 10],
                config[ptr + 11],
            ]);
            return Some((u64::from(hi) << 32) | u64::from(lo & 0xffff_c000));
        }
        let next = usize::from(config[ptr + 1] & !0x3);
        if next == 0 || next == ptr {
            return None;
        }
        ptr = next;
    }
    None
}

/// Write the IOMMU MMIO base. A short page is left unchanged.
pub fn store_iommu(page: &mut [u8], addr: u64) {
    let Some(end) = IOMMU_RECORD.checked_add(8) else {
        return;
    };
    if page.len() < end {
        return;
    }
    page[IOMMU_RECORD..end].copy_from_slice(&addr.to_le_bytes());
}

/// IOMMU MMIO base from [`store_iommu`]. A short page reads as 0.
#[must_use]
pub fn load_iommu(page: &[u8]) -> u64 {
    let Some(end) = IOMMU_RECORD.checked_add(8) else {
        return 0;
    };
    if page.len() < end {
        return 0;
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&page[IOMMU_RECORD..end]);
    u64::from_le_bytes(buf)
}

/// Byte offset of this location in the shared header page.
fn header_offset(bus: u8, dev: u8, func: u8) -> Option<usize> {
    LOCATIONS
        .iter()
        .position(|bdf| *bdf == (bus, dev, func))
        .map(|index| index * HEADER_BYTES)
}

/// Copy one header into the shared page. A short buffer is left unchanged.
pub fn store_header(page: &mut [u8], index: usize, header: &[u8]) {
    let Some(start) = index.checked_mul(HEADER_BYTES) else {
        return;
    };
    let Some(end) = start.checked_add(HEADER_BYTES) else {
        return;
    };
    if header.len() < HEADER_BYTES || page.len() < end {
        return;
    }
    page[start..end].copy_from_slice(&header[..HEADER_BYTES]);
}

/// Header the probe stored for this location. Missing slots read as zeros.
#[must_use]
pub fn load_header(page: &[u8], bus: u8, dev: u8, func: u8) -> [u8; HEADER_BYTES] {
    let mut header = [0u8; HEADER_BYTES];
    let Some(start) = header_offset(bus, dev, func) else {
        return header;
    };
    let end = start + HEADER_BYTES;
    if page.len() < end {
        return header;
    }
    header.copy_from_slice(&page[start..end]);
    header
}

#[must_use]
pub fn config_address(bus: u8, dev: u8, func: u8, reg: u8) -> u32 {
    0x8000_0000
        | (u32::from(bus) << 16)
        | (u32::from(dev) << 11)
        | (u32::from(func) << 8)
        | u32::from(reg & 0xfc)
}

/// `mt7921: bar` and sixteen hex digits. `None` is an I/O BAR.
pub fn bar_line(addr: Option<u64>, out: &mut [u8]) -> usize {
    let prefix = b"mt7921: bar ";
    if out.len() < prefix.len() {
        return 0;
    }
    out[..prefix.len()].copy_from_slice(prefix);
    let Some(addr) = addr else {
        let io = b"io";
        if out.len() < prefix.len() + io.len() {
            return prefix.len();
        }
        out[prefix.len()..prefix.len() + io.len()].copy_from_slice(io);
        return prefix.len() + io.len();
    };
    if out.len() < prefix.len() + 16 {
        return prefix.len();
    }
    for (i, byte) in out[prefix.len()..prefix.len() + 16].iter_mut().enumerate() {
        let shift = (15 - i) * 4;
        *byte = hex(((addr >> shift) & 0xf) as u8);
    }
    prefix.len() + 16
}

/// First memory BAR. A 64-bit BAR includes the following dword. Type bits are cleared.
#[must_use]
pub fn memory_bar(header: &[u8]) -> Option<u64> {
    if header.len() < 0x18 {
        return None;
    }
    let lo = u32::from_le_bytes([header[0x10], header[0x11], header[0x12], header[0x13]]);
    if lo & 1 != 0 {
        return None;
    }
    let mut addr = u64::from(lo & 0xffff_fff0);
    if lo & 0x4 != 0 {
        let hi = u32::from_le_bytes([header[0x14], header[0x15], header[0x16], header[0x17]]);
        addr |= u64::from(hi) << 32;
    }
    Some(addr)
}

#[must_use]
pub fn vendor_device(header: &[u8]) -> Option<(u16, u16)> {
    if header.len() < 4 || header[0] == 0xff {
        return None;
    }
    let vendor = u16::from_le_bytes([header[0], header[1]]);
    let device = u16::from_le_bytes([header[2], header[3]]);
    if vendor == 0xffff || vendor == 0 {
        return None;
    }
    Some((vendor, device))
}

#[must_use]
pub fn class_code(header: &[u8]) -> Option<u32> {
    if header.len() < 12 {
        return None;
    }
    Some((u32::from(header[11]) << 16) | (u32::from(header[10]) << 8) | u32::from(header[9]))
}

/// `01:00.0 10de:1f99 bar 00000000 irq 00`
#[must_use]
pub fn describe(bus: u8, dev: u8, func: u8, header: &[u8]) -> Option<[u8; 40]> {
    let (vendor, device) = vendor_device(header)?;
    if header.len() < 0x3d {
        return None;
    }
    let bar = u32::from_le_bytes([header[0x10], header[0x11], header[0x12], header[0x13]]);
    let irq = header[0x3c];
    let mut line = [b' '; 40];
    write_bdf(&mut line, bus, dev, func);
    write_hex16(&mut line[8..12], vendor);
    line[12] = b':';
    write_hex16(&mut line[13..17], device);
    line[18..22].copy_from_slice(b"bar ");
    write_hex32(&mut line[22..30], bar);
    line[31..35].copy_from_slice(b"irq ");
    write_hex8(&mut line[35..37], irq);
    Some(line)
}

fn write_bdf(line: &mut [u8], bus: u8, dev: u8, func: u8) {
    write_hex8(&mut line[0..2], bus);
    line[2] = b':';
    write_hex8(&mut line[3..5], dev);
    line[5] = b'.';
    line[6] = b'0' + (func & 0xf);
}

fn write_hex8(slot: &mut [u8], value: u8) {
    slot[0] = hex(value >> 4);
    slot[1] = hex(value);
}

fn write_hex16(slot: &mut [u8], value: u16) {
    for (i, byte) in slot.iter_mut().enumerate() {
        let shift = (3 - i) * 4;
        *byte = hex(hex_byte(u32::from(value >> shift)));
    }
}

fn write_hex32(slot: &mut [u8], value: u32) {
    for (i, byte) in slot.iter_mut().enumerate() {
        let shift = (7 - i) * 4;
        *byte = hex(hex_byte(value >> shift));
    }
}

fn hex_byte(value: u32) -> u8 {
    u8::try_from(value & 0xff).expect("hex byte")
}

fn hex(nibble: u8) -> u8 {
    let nibble = nibble & 0xf;
    if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + (nibble - 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_a_present_device() {
        let mut header = [0u8; 64];
        header[0..2].copy_from_slice(&0x10deu16.to_le_bytes());
        header[2..4].copy_from_slice(&0x1f99u16.to_le_bytes());
        header[0x10] = 0x04;
        header[0x3c] = 10;
        let line = describe(1, 0, 0, &header).unwrap();
        assert_eq!(&line[..37], b"01:00.0 10de:1f99 bar 00000004 irq 0a");
    }

    #[test]
    fn mt7921_ids_are_the_recorded_chip() {
        assert!(MT7921_IDS.contains(&(0x14c3, 0x7961)));
    }

    #[test]
    fn mt7921_bar_drops_the_type_bits() {
        let mut header = [0u8; 64];
        header[0x10..0x14].copy_from_slice(&0x8200_000cu32.to_le_bytes());
        assert_eq!(memory_bar(&header), Some(0x8200_0000));
        header[0x14..0x18].copy_from_slice(&0xfcu32.to_le_bytes());
        assert_eq!(memory_bar(&header), Some(MT7921_BAR));
        header[0x14] = 1;
        header[0x15] = 0;
        header[0x16] = 0;
        header[0x17] = 0;
        assert_eq!(memory_bar(&header), Some(0x1_8200_0000));
        header[0x10] = 1;
        assert_eq!(memory_bar(&header), None);
    }

    #[test]
    fn bar_line_prints_the_upper_dword() {
        let mut line = [0u8; 40];
        let n = bar_line(Some(0x00FC_8200_0000), &mut line);
        assert_eq!(&line[..n], b"mt7921: bar 000000fc82000000");
        let n = bar_line(None, &mut line);
        assert_eq!(&line[..n], b"mt7921: bar io");
        assert_eq!(bar_line(Some(0), &mut [0u8; 4]), 0);
    }

    #[test]
    fn iommu_base_drops_the_lock_bit() {
        let mut config = [0u8; 256];
        config[0x09] = 0x00;
        config[0x0a] = 0x06;
        config[0x0b] = 0x08;
        config[0x06] = 0x10;
        config[0x34] = 0x40;
        config[0x40] = 0x0f;
        config[0x44..0x48].copy_from_slice(&0xfed8_0001u32.to_le_bytes());
        assert_eq!(iommu_base(&config), Some(0xfed8_0000));
        let mut page = [0u8; 0x1000];
        store_iommu(&mut page, 0xfed8_0000);
        assert_eq!(load_iommu(&page), 0xfed8_0000);
    }

    #[test]
    fn exclusion_covers_the_swift_dma_windows() {
        assert_eq!(exclusion_base(SWIFT_DMA_START), 0x01f0_0001);
        assert_eq!(exclusion_limit(SWIFT_DMA_END), 0x01f9_f000);
        let ctrl = 1u64 | (1 << 10);
        let off = IOMMU_OFF_BITS
            .iter()
            .fold(ctrl, |value, bit| iommu_with_bit(value, *bit, false));
        assert_eq!(off, 1 << 10);
        assert_eq!(iommu_with_bit(off, 0, true) & 1, 1);
    }

    #[test]
    fn header_page_round_trip() {
        let mut page = [0u8; 0x1000];
        let mut header = [0u8; HEADER_BYTES];
        header[0..2].copy_from_slice(&0x14c3u16.to_le_bytes());
        header[2..4].copy_from_slice(&0x7961u16.to_le_bytes());
        let (bus, dev, func) = MT7921_BDF;
        let index = header_offset(bus, dev, func).unwrap() / HEADER_BYTES;
        store_header(&mut page, index, &header);
        assert_eq!(load_header(&page, bus, dev, func), header);
        assert_eq!(load_header(&page, 9, 9, 9), [0u8; HEADER_BYTES]);
    }
}
