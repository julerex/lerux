//! Identify a Realtek RTL8852BE from a PCI config header, and decode the
//! type-1 bridge window in front of it.
//!
//! This crate does not touch the card. A later probe reads these bytes through
//! I/O ports `0xcf8`/`0xcfc`. Linux `rtw89_pci` (v6.8) iomaps BAR 2.

#![no_std]

#[cfg(test)]
extern crate std;

/// Realtek PCI vendor id.
pub const REALTEK: VendorId = VendorId(0x10ec);
/// RTL8852BE, the id on this Z97 (`10ec:b852`).
pub const RTL8852BE: DeviceId = DeviceId(0xb852);
/// Second id in the Linux `rtw89_8852be` table (`rtw8852be.c`).
pub const RTL8852BE_B85B: DeviceId = DeviceId(0xb85b);
/// BAR 2 length. The 64-byte header does not carry it. Sysfs `resource` on
/// this desk reports a 1 MiB memory window, and that is what rtw89 iomaps.
pub const MMIO_LEN: u64 = 0x10_0000;
/// BAR 0 length from the same resource file. rtw89 does not iomap this window.
pub const IO_LEN: u32 = 0x100;

/// PCI vendor id. Distinct from [`DeviceId`] so the two cannot be swapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorId(pub u16);

/// PCI device id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceId(pub u16);

/// A config dump shorter than the 64-byte header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortConfig {
    pub len: usize,
}

/// `10ec:b852` or `10ec:b85b` whose BAR 0 and BAR 2 are not the 8852BE map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarMismatch {
    pub device: DeviceId,
}

/// What a 64-byte type-0 header is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Identity {
    Other { vendor: VendorId, device: DeviceId },
    Rtl8852be(Card),
}

/// Assigned windows for an 8852BE. Addresses come from the header, not from a
/// desk constant. Lengths are [`MMIO_LEN`] and [`IO_LEN`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Card {
    pub device: DeviceId,
    pub revision: u8,
    pub subsystem_vendor: u16,
    pub subsystem_device: u16,
    pub io_port: u32,
    pub mmio_phys: u64,
}

/// Type-1 bridge header, including the non-prefetchable memory window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bridge {
    pub vendor: VendorId,
    pub device: DeviceId,
    pub secondary_bus: u8,
    pub subordinate_bus: u8,
    pub io_base: u32,
    pub io_limit_inclusive: u32,
    pub mem_base: u64,
    pub mem_limit_inclusive: u64,
}

/// Header is not a type-1 bridge. The low 7 bits of the header-type byte are kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotBridge {
    pub header_type: u8,
}

/// Parse a PCI config header.
///
/// Matching device ids still return [`BarMismatch`] unless BAR 0 is I/O and
/// BAR 2 is a 64-bit, non-prefetchable memory window. That is the map
/// `rtw89_pci_probe` iomaps (`bar_id = 2`).
pub fn identify(config: &[u8]) -> Result<Identity, CardError> {
    let header = header64(config).map_err(CardError::Short)?;
    let vendor = VendorId(u16::from_le_bytes([header[0], header[1]]));
    let device = DeviceId(u16::from_le_bytes([header[2], header[3]]));
    if vendor != REALTEK || (device != RTL8852BE && device != RTL8852BE_B85B) {
        return Ok(Identity::Other { vendor, device });
    }
    let io_port = io_port(u32_at(header, 0x10));
    let mmio_phys = mmio_phys(u32_at(header, 0x18), u32_at(header, 0x1c));
    let (Some(io_port), Some(mmio_phys)) = (io_port, mmio_phys) else {
        return Err(CardError::BarMismatch(BarMismatch { device }));
    };
    Ok(Identity::Rtl8852be(Card {
        device,
        revision: header[0x08],
        subsystem_vendor: u16::from_le_bytes([header[0x2c], header[0x2d]]),
        subsystem_device: u16::from_le_bytes([header[0x2e], header[0x2f]]),
        io_port,
        mmio_phys,
    }))
}

/// Why [`identify`] rejected a buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardError {
    Short(ShortConfig),
    BarMismatch(BarMismatch),
}

/// Decode a type-1 bridge. I/O upper bits apply only when the I/O base
/// register's low nibble is 1 (32-bit I/O). A low nibble of 0 is a 16-bit window.
pub fn decode_bridge(config: &[u8]) -> Result<Bridge, BridgeError> {
    let header = header64(config).map_err(BridgeError::Short)?;
    let header_type = header[0x0e] & 0x7f;
    if header_type != 0x01 {
        return Err(BridgeError::NotBridge(NotBridge { header_type }));
    }
    let (io_base, io_limit_inclusive) = io_window(header[0x1c], header[0x1d], header);
    let (mem_base, mem_limit_inclusive) = mem_window(
        u16::from_le_bytes([header[0x20], header[0x21]]),
        u16::from_le_bytes([header[0x22], header[0x23]]),
    );
    Ok(Bridge {
        vendor: VendorId(u16::from_le_bytes([header[0], header[1]])),
        device: DeviceId(u16::from_le_bytes([header[2], header[3]])),
        secondary_bus: header[0x19],
        subordinate_bus: header[0x1a],
        io_base,
        io_limit_inclusive,
        mem_base,
        mem_limit_inclusive,
    })
}

/// Why [`decode_bridge`] rejected a buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeError {
    Short(ShortConfig),
    NotBridge(NotBridge),
}

/// True when BAR 2's [`MMIO_LEN`] bytes sit inside the bridge memory window.
/// rtw89 iomaps this range and does not use the I/O port.
pub fn mmio_covered(bridge: &Bridge, card: &Card) -> bool {
    range_inside(
        card.mmio_phys,
        MMIO_LEN,
        bridge.mem_base,
        bridge.mem_limit_inclusive,
    )
}

/// True when BAR 0's [`IO_LEN`] bytes sit inside the bridge I/O window.
pub fn io_covered(bridge: &Bridge, card: &Card) -> bool {
    range_inside(
        u64::from(card.io_port),
        u64::from(IO_LEN),
        u64::from(bridge.io_base),
        u64::from(bridge.io_limit_inclusive),
    )
}

fn header64(config: &[u8]) -> Result<&[u8; 64], ShortConfig> {
    let header: &[u8; 64] = config
        .get(..64)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(ShortConfig { len: config.len() })?;
    Ok(header)
}

fn u32_at(header: &[u8; 64], offset: usize) -> u32 {
    u32::from_le_bytes([
        header[offset],
        header[offset + 1],
        header[offset + 2],
        header[offset + 3],
    ])
}

fn io_port(bar: u32) -> Option<u32> {
    if bar & 1 == 0 {
        return None;
    }
    Some(bar & 0xffff_fffc)
}

fn mmio_phys(low: u32, high: u32) -> Option<u64> {
    let memory = low & 1 == 0;
    let wide = ((low >> 1) & 0b11) == 0b10;
    let prefetch = (low & 0b1000) != 0;
    if !memory || !wide || prefetch {
        return None;
    }
    Some(u64::from(low & 0xffff_fff0) | (u64::from(high) << 32))
}

fn io_window(base_reg: u8, limit_reg: u8, header: &[u8; 64]) -> (u32, u32) {
    let mut base = u32::from(base_reg & 0xf0) << 8;
    let mut limit = (u32::from(limit_reg & 0xf0) << 8) | 0x0fff;
    if base_reg & 0x0f == 1 {
        let upper_base = u32::from(u16::from_le_bytes([header[0x30], header[0x31]]));
        let upper_limit = u32::from(u16::from_le_bytes([header[0x32], header[0x33]]));
        base |= upper_base << 16;
        limit |= upper_limit << 16;
    }
    (base, limit)
}

fn mem_window(base_reg: u16, limit_reg: u16) -> (u64, u64) {
    let base = u64::from(base_reg & 0xfff0) << 16;
    let limit = (u64::from(limit_reg & 0xfff0) << 16) | 0x000f_ffff;
    (base, limit)
}

fn range_inside(start: u64, len: u64, window_base: u64, window_limit: u64) -> bool {
    let Some(last) = start.checked_add(len.saturating_sub(1)) else {
        return false;
    };
    len > 0 && start >= window_base && last <= window_limit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z97_card() -> [u8; 64] {
        *include_bytes!("../fixtures/z97-05-00.0.config")
    }

    fn z97_bridge() -> [u8; 64] {
        *include_bytes!("../fixtures/z97-00-1c.6.config")
    }

    fn card_from(bytes: &[u8; 64]) -> Card {
        match identify(bytes) {
            Ok(Identity::Rtl8852be(card)) => card,
            other => panic!("expected 8852BE, got {other:?}"),
        }
    }

    #[test]
    fn z97_header_is_8852be_at_the_assigned_bars() {
        let card = card_from(&z97_card());
        assert_eq!(
            card,
            Card {
                device: RTL8852BE,
                revision: 0,
                subsystem_vendor: 0x1a3b,
                subsystem_device: 0x5470,
                io_port: 0xd000,
                mmio_phys: 0xf7c0_0000,
            }
        );
    }

    #[test]
    fn alternate_id_keeps_the_parsed_address() {
        let mut bytes = z97_card();
        bytes[2] = 0x5b;
        bytes[3] = 0xb8;
        let card = card_from(&bytes);
        assert_eq!(card.device, RTL8852BE_B85B);
        assert_eq!(card.mmio_phys, 0xf7c0_0000);
    }

    #[test]
    fn io_port_above_64k_keeps_the_high_bits() {
        let mut bytes = z97_card();
        bytes[0x10..0x14].copy_from_slice(&0x0002_d001u32.to_le_bytes());
        let card = card_from(&bytes);
        assert_eq!(card.io_port, 0x0002_d000);
    }

    #[test]
    fn moved_mmio_bar_is_reported() {
        let mut bytes = z97_card();
        bytes[0x18..0x1c].copy_from_slice(&0xf800_0004u32.to_le_bytes());
        let card = card_from(&bytes);
        assert_eq!(card.mmio_phys, 0xf800_0000);
    }

    #[test]
    fn mmio_high_dword_is_kept() {
        let mut bytes = z97_card();
        bytes[0x1c..0x20].copy_from_slice(&0x0000_0001u32.to_le_bytes());
        let card = card_from(&bytes);
        assert_eq!(card.mmio_phys, 0x1_f7c0_0000);
    }

    #[test]
    fn other_device_id_is_not_a_card() {
        let mut bytes = z97_card();
        bytes[2] = 0x34;
        bytes[3] = 0x12;
        let id = identify(&bytes).expect("header is 64 bytes");
        assert_eq!(
            id,
            Identity::Other {
                vendor: REALTEK,
                device: DeviceId(0x1234),
            }
        );
    }

    #[test]
    fn short_header_is_rejected() {
        let err = identify(&z97_card()[..32]).expect_err("short");
        assert_eq!(err, CardError::Short(ShortConfig { len: 32 }));
    }

    #[test]
    fn io_bar_in_the_memory_slot_is_a_mismatch() {
        let mut bytes = z97_card();
        bytes[0x18] = 0x01;
        let err = identify(&bytes).expect_err("bar");
        assert_eq!(
            err,
            CardError::BarMismatch(BarMismatch { device: RTL8852BE })
        );
    }

    #[test]
    fn z97_root_port_window_covers_the_card() {
        let bridge = decode_bridge(&z97_bridge()).expect("type-1 bridge");
        assert_eq!(bridge.vendor, VendorId(0x8086));
        assert_eq!(bridge.device, DeviceId(0x8c9c));
        assert_eq!(bridge.secondary_bus, 5);
        assert_eq!(bridge.subordinate_bus, 5);
        assert_eq!(bridge.mem_base, 0xf7c0_0000);
        assert_eq!(bridge.mem_limit_inclusive, 0xf7cf_ffff);
        assert_eq!(bridge.io_base, 0xd000);
        assert_eq!(bridge.io_limit_inclusive, 0xdfff);
        let card = card_from(&z97_card());
        assert_eq!(card.mmio_phys, bridge.mem_base);
        assert_eq!(card.mmio_phys + MMIO_LEN - 1, bridge.mem_limit_inclusive);
        assert!(mmio_covered(&bridge, &card));
        assert!(io_covered(&bridge, &card));
    }

    #[test]
    fn io_outside_the_window_does_not_hide_a_covered_mmio_bar() {
        let bridge = decode_bridge(&z97_bridge()).expect("type-1 bridge");
        let mut card = card_from(&z97_card());
        card.io_port = 0x1_0000;
        assert!(mmio_covered(&bridge, &card));
        assert!(!io_covered(&bridge, &card));
    }

    #[test]
    fn thirty_two_bit_io_window_uses_the_upper_half() {
        let mut bytes = z97_bridge();
        bytes[0x1c] = 0xd1;
        bytes[0x1d] = 0xd1;
        bytes[0x30..0x34].copy_from_slice(&0x0001_0001u32.to_le_bytes());
        let bridge = decode_bridge(&bytes).expect("type-1 bridge");
        assert_eq!(bridge.io_base, 0x1_d000);
        assert_eq!(bridge.io_limit_inclusive, 0x1_dfff);
    }

    #[test]
    fn one_extra_mmio_byte_falls_outside_the_window() {
        let bridge = decode_bridge(&z97_bridge()).expect("type-1 bridge");
        let mut card = card_from(&z97_card());
        card.mmio_phys = bridge.mem_limit_inclusive - (MMIO_LEN - 2);
        assert!(!mmio_covered(&bridge, &card));
    }

    #[test]
    fn endpoint_header_is_not_a_bridge() {
        let err = decode_bridge(&z97_card()).expect_err("type 0");
        assert_eq!(err, BridgeError::NotBridge(NotBridge { header_type: 0 }));
    }
}
