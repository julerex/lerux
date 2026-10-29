//! Windows recorded these bus numbers. A metal probe replaces them if they differ.

/// `01:00.0` NVIDIA, `02:00.0` NVMe, `03:00.0` MT7921, `04:00.0` Radeon,
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
pub const NVME_BDF: (u8, u8, u8) = (2, 0, 0);
pub const XHCI_BDFS: [(u8, u8, u8); 2] = [(4, 0, 3), (4, 0, 4)];

pub const MT7921_IDS: [(u16, u16); 2] = [(0x14c3, 0x7961), (0x14c3, 0x0608)];
pub const NVME_CLASS: u32 = 0x0001_0802;
pub const XHCI_CLASS: u32 = 0x000c_0330;

/// One PCI configuration header. Six of these fill the shared page.
pub const HEADER_BYTES: usize = 64;

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

pub fn config_address(bus: u8, dev: u8, func: u8, reg: u8) -> u32 {
    0x8000_0000
        | (u32::from(bus) << 16)
        | (u32::from(dev) << 11)
        | (u32::from(func) << 8)
        | u32::from(reg & 0xfc)
}

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

pub fn class_code(header: &[u8]) -> Option<u32> {
    if header.len() < 12 {
        return None;
    }
    Some((u32::from(header[11]) << 16) | (u32::from(header[10]) << 8) | u32::from(header[9]))
}

/// `01:00.0 10de:1f99 bar 00000000 irq 00`
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
        *byte = hex((value >> shift) as u8);
    }
}

fn write_hex32(slot: &mut [u8], value: u32) {
    for (i, byte) in slot.iter_mut().enumerate() {
        let shift = (7 - i) * 4;
        *byte = hex((value >> shift) as u8);
    }
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
