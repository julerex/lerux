//! Identify a Realtek RTL8852BE from a PCI config header, decode the type-1
//! bridge window in front of it, and stop the PCI HCI.
//!
//! Register access goes through [`Regs`]. This crate holds no raw pointer.
//! The protection domain is the volatile user of BAR 2. The probe still reads
//! configuration space through I/O ports `0xcf8` and `0xcfc`. Linux `rtw89_pci`
//! (v6.8) iomaps BAR 2.

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
/// BAR 2 on this desk. The ISO maps this address and no other MMIO window.
pub const MAPPED_MMIO_PHYS: u64 = 0xf7c0_0000;

/// PCI command bits for I/O space, memory space, and bus master.
const PCI_COMMAND_ENABLE: u16 = 0x0007;

/// `R_AX_SYS_CFG1` in rtw89 v6.8 `reg.h`. Chip version is bits 15:12.
const R_AX_SYS_CFG1: usize = 0x00F0;
const B_AX_CHIP_VER_MASK: u32 = 0x0000_F000;
const CHIP_VER_SHIFT: u32 = 12;
/// `R_AX_HIMR0`. Idle stores 0.
const R_AX_HIMR0: usize = 0x01A0;
/// `R_AX_PCIE_HIMR00`. Idle stores 0.
const R_AX_PCIE_HIMR00: usize = 0x10B0;
/// `R_AX_PCIE_HIMR10`. Idle stores 0.
const R_AX_PCIE_HIMR10: usize = 0x13B0;
/// `R_AX_PCIE_INIT_CFG1`. Idle clears the HCI enable bits.
const R_AX_PCIE_INIT_CFG1: usize = 0x1000;
const B_AX_TXHCI_EN: u32 = 1 << 11;
const B_AX_RXHCI_EN: u32 = 1 << 13;
/// `R_AX_PCIE_DMA_STOP1`. Idle sets the stop bit.
const R_AX_PCIE_DMA_STOP1: usize = 0x1010;
const B_AX_STOP_PCIEIO: u32 = 1 << 20;

/// Status page kind. The header was not an 8852BE the bridge can reach.
pub const STATUS_MISSING: u8 = 0;
/// Status page kind. The id matched and BAR 2 is not the mapped window.
pub const STATUS_BAR_MISMATCH: u8 = 1;
/// Status page kind. BAR 2 is mapped and the PCI HCI read back idle.
pub const STATUS_PROGRAMMED: u8 = 2;

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

/// What the desk headers are, relative to the mapped BAR 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardState {
    /// Bridge or endpoint is not an 8852BE this code can program.
    Missing,
    /// The id matches and BAR 2 is not the mapped, covered memory window.
    BarMismatch(BarMismatch),
    /// BAR 2 is the mapped window and the bridge covers it.
    Programmed(Programmed),
}

/// An 8852BE whose BAR 2 is the address the ISO maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Programmed {
    pub card: Card,
    /// Endpoint command half after [`enable_command`].
    pub command: u16,
}

/// One status word on the page the shell reads.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    /// [`STATUS_MISSING`], [`STATUS_BAR_MISMATCH`], or [`STATUS_PROGRAMMED`].
    pub kind: u8,
    /// Bits 15:12 of `R_AX_SYS_CFG1` when [`kind`](Status::kind) is programmed.
    pub chip_ver: u8,
    /// Endpoint device id when the header is an 8852BE, otherwise 0.
    pub device: u16,
    /// [`MAPPED_MMIO_PHYS`] when the HCI is idle, otherwise 0.
    pub mmio_phys: u64,
}

const _: () = assert!(size_of::<Status>() == 16);

/// Little-endian register window. The protection domain supplies the volatile map.
pub trait Regs {
    /// Read one word. An offset that does not hold a full word returns `0xffff_ffff`.
    fn read32(&self, offset: usize) -> u32;
    /// Write one word. An offset that does not hold a full word does nothing.
    fn write32(&mut self, offset: usize, value: u32);
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
///
/// # Errors
///
/// Returns [`CardError::Short`] when `config` is shorter than 64 bytes.
/// Returns [`CardError::BarMismatch`] when the device id matches and BAR 0 is
/// not I/O or BAR 2 is not a 64-bit non-prefetchable memory window.
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
///
/// # Errors
///
/// Returns [`BridgeError::Short`] when `config` is shorter than 64 bytes.
/// Returns [`BridgeError::NotBridge`] when the header type is not 1.
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
#[must_use]
pub fn mmio_covered(bridge: &Bridge, card: &Card) -> bool {
    range_inside(
        card.mmio_phys,
        MMIO_LEN,
        bridge.mem_base,
        bridge.mem_limit_inclusive,
    )
}

/// True when BAR 0's [`IO_LEN`] bytes sit inside the bridge I/O window.
#[must_use]
pub fn io_covered(bridge: &Bridge, card: &Card) -> bool {
    range_inside(
        u64::from(card.io_port),
        u64::from(IO_LEN),
        u64::from(bridge.io_base),
        u64::from(bridge.io_limit_inclusive),
    )
}

/// Classify the desk headers against the BAR 2 address the ISO maps.
///
/// `Missing` means the bridge is short or not type 1, or the endpoint is short
/// or not an 8852BE. `BarMismatch` means the id matches and BAR 2 is not
/// `mapped`, or the bridge does not cover [`MMIO_LEN`]. `Programmed` means
/// BAR 2 matches [`identify`]'s rules, the address equals `mapped`, and the
/// bridge covers that window. `command` is [`enable_command`] of the endpoint
/// command half at offset `0x04`.
#[must_use]
pub fn card_state(bridge: &[u8], endpoint: &[u8], mapped: u64) -> CardState {
    let Ok(bridge) = decode_bridge(bridge) else {
        return CardState::Missing;
    };
    match identify(endpoint) {
        Ok(Identity::Rtl8852be(card)) => programmed_or_mismatch(&bridge, endpoint, card, mapped),
        Ok(Identity::Other { .. }) | Err(CardError::Short(_)) => CardState::Missing,
        Err(CardError::BarMismatch(mismatch)) => CardState::BarMismatch(mismatch),
    }
}

/// Set I/O space, memory space, and bus master. Every other bit stays.
///
/// A second call returns the same word.
#[must_use]
pub const fn enable_command(current: u16) -> u16 {
    current | PCI_COMMAND_ENABLE
}

/// Stop the PCI HCI. `SYS_CFG1` of `0` or `0xffff_ffff` writes nothing.
///
/// # Errors
///
/// Returns `Err` when `SYS_CFG1` is `0` or `0xffff_ffff`, or when a read-back
/// is not idle. Idle means the three HIMR words are 0, both HCI enable bits
/// are clear, and the stop bit is set.
#[expect(
    clippy::result_unit_err,
    reason = "idle has one failure and the caller does not branch on a payload"
)]
pub fn idle(regs: &mut (impl Regs + ?Sized)) -> Result<u8, ()> {
    let cfg = regs.read32(R_AX_SYS_CFG1);
    if cfg == 0 || cfg == 0xffff_ffff {
        return Err(());
    }
    stop_hci(regs);
    if hci_idle(regs) {
        Ok(chip_ver(cfg))
    } else {
        Err(())
    }
}

/// `wifi ok` when the HCI is idle, and `wifi err` for every other kind.
#[must_use]
pub const fn report(status: &Status) -> &'static [u8] {
    if status.kind == STATUS_PROGRAMMED {
        b"wifi ok\n"
    } else {
        b"wifi err\n"
    }
}

impl Regs for [u8] {
    fn read32(&self, offset: usize) -> u32 {
        let Some(end) = offset.checked_add(4) else {
            return 0xffff_ffff;
        };
        let Some(bytes) = self.get(offset..end) else {
            return 0xffff_ffff;
        };
        let mut word = [0u8; 4];
        word.copy_from_slice(bytes);
        u32::from_le_bytes(word)
    }

    fn write32(&mut self, offset: usize, value: u32) {
        let Some(end) = offset.checked_add(4) else {
            return;
        };
        let Some(slot) = self.get_mut(offset..end) else {
            return;
        };
        slot.copy_from_slice(&value.to_le_bytes());
    }
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

fn programmed_or_mismatch(bridge: &Bridge, endpoint: &[u8], card: Card, mapped: u64) -> CardState {
    if card.mmio_phys != mapped || !mmio_covered(bridge, &card) {
        return CardState::BarMismatch(BarMismatch {
            device: card.device,
        });
    }
    let command = enable_command(u16::from_le_bytes([endpoint[0x04], endpoint[0x05]]));
    CardState::Programmed(Programmed { card, command })
}

fn stop_hci(regs: &mut (impl Regs + ?Sized)) {
    regs.write32(R_AX_HIMR0, 0);
    regs.write32(R_AX_PCIE_HIMR00, 0);
    regs.write32(R_AX_PCIE_HIMR10, 0);
    let init = regs.read32(R_AX_PCIE_INIT_CFG1);
    regs.write32(R_AX_PCIE_INIT_CFG1, init & !B_AX_TXHCI_EN & !B_AX_RXHCI_EN);
    let stop = regs.read32(R_AX_PCIE_DMA_STOP1);
    regs.write32(R_AX_PCIE_DMA_STOP1, stop | B_AX_STOP_PCIEIO);
}

fn hci_idle(regs: &(impl Regs + ?Sized)) -> bool {
    let init = regs.read32(R_AX_PCIE_INIT_CFG1);
    let stop = regs.read32(R_AX_PCIE_DMA_STOP1);
    regs.read32(R_AX_HIMR0) == 0
        && regs.read32(R_AX_PCIE_HIMR00) == 0
        && regs.read32(R_AX_PCIE_HIMR10) == 0
        && (init & (B_AX_TXHCI_EN | B_AX_RXHCI_EN)) == 0
        && (stop & B_AX_STOP_PCIEIO) != 0
}

fn chip_ver(cfg: u32) -> u8 {
    let bits = (cfg & B_AX_CHIP_VER_MASK) >> CHIP_VER_SHIFT;
    u8::try_from(bits).unwrap_or(0)
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

    #[test]
    fn desk_headers_program_the_mapped_bar() {
        let state = card_state(&z97_bridge(), &z97_card(), MAPPED_MMIO_PHYS);
        let CardState::Programmed(programmed) = state else {
            panic!("expected programmed, got {state:?}");
        };
        assert_eq!(programmed.card.mmio_phys, 0xf7c0_0000);
        assert_eq!(programmed.command, 0x0407);
        assert_eq!(enable_command(0x0407), 0x0407);
        assert_eq!(enable_command(0x0400), 0x0407);
    }

    #[test]
    fn moved_bar_against_the_desk_map_is_a_mismatch() {
        let mut endpoint = z97_card();
        endpoint[0x18..0x1c].copy_from_slice(&0xf800_0004u32.to_le_bytes());
        let state = card_state(&z97_bridge(), &endpoint, MAPPED_MMIO_PHYS);
        assert!(matches!(state, CardState::BarMismatch(_)));
    }

    #[test]
    fn io_slot_against_the_desk_map_is_a_mismatch() {
        let mut endpoint = z97_card();
        endpoint[0x18] = 0x01;
        let state = card_state(&z97_bridge(), &endpoint, MAPPED_MMIO_PHYS);
        assert!(matches!(state, CardState::BarMismatch(_)));
    }

    #[test]
    fn other_vendor_and_short_bridge_are_missing() {
        let mut endpoint = z97_card();
        endpoint[0] = 0x86;
        endpoint[1] = 0x80;
        let state = card_state(&z97_bridge(), &endpoint, MAPPED_MMIO_PHYS);
        assert!(matches!(state, CardState::Missing));
        let state = card_state(&z97_bridge()[..32], &z97_card(), MAPPED_MMIO_PHYS);
        assert!(matches!(state, CardState::Missing));
    }

    fn armed_window(cfg1: u32) -> [u8; 0x1400] {
        let mut regs = [0u8; 0x1400];
        regs.write32(R_AX_SYS_CFG1, cfg1);
        regs.write32(R_AX_PCIE_INIT_CFG1, B_AX_TXHCI_EN | B_AX_RXHCI_EN | 1);
        regs.write32(R_AX_PCIE_DMA_STOP1, 2);
        regs.write32(R_AX_HIMR0, 0x11);
        regs.write32(R_AX_PCIE_HIMR00, 0x22);
        regs.write32(R_AX_PCIE_HIMR10, 0x33);
        regs
    }

    #[test]
    fn idle_stops_hci_for_the_desk_headers() {
        let state = card_state(&z97_bridge(), &z97_card(), MAPPED_MMIO_PHYS);
        assert!(matches!(state, CardState::Programmed(_)));
        let mut regs = armed_window(0x0000_C001);
        let chip = idle(&mut regs[..]).expect("config word is live");
        assert_eq!(chip, 0xC);
        assert_eq!(regs.read32(R_AX_HIMR0), 0);
        assert_eq!(regs.read32(R_AX_PCIE_HIMR00), 0);
        assert_eq!(regs.read32(R_AX_PCIE_HIMR10), 0);
        assert_eq!(
            regs.read32(R_AX_PCIE_INIT_CFG1) & (B_AX_TXHCI_EN | B_AX_RXHCI_EN),
            0
        );
        assert_eq!(regs.read32(R_AX_PCIE_INIT_CFG1) & 1, 1);
        assert_ne!(regs.read32(R_AX_PCIE_DMA_STOP1) & B_AX_STOP_PCIEIO, 0);
        assert_eq!(regs.read32(R_AX_PCIE_DMA_STOP1) & 2, 2);
    }

    fn dead_cfg_keeps_hci(cfg1: u32) -> bool {
        let mut regs = armed_window(cfg1);
        let before = regs.read32(R_AX_PCIE_INIT_CFG1);
        idle(&mut regs[..]).is_err() && regs.read32(R_AX_PCIE_INIT_CFG1) == before
    }

    #[test]
    fn all_ones_sys_cfg_leaves_the_hci_bit() {
        assert!(dead_cfg_keeps_hci(0xffff_ffff));
        assert!(dead_cfg_keeps_hci(0));
        let seeded = armed_window(0xffff_ffff);
        assert_eq!(
            seeded.read32(R_AX_PCIE_INIT_CFG1) & B_AX_TXHCI_EN,
            B_AX_TXHCI_EN
        );
    }

    #[test]
    fn report_follows_the_status_kind() {
        let programmed = Status {
            kind: STATUS_PROGRAMMED,
            chip_ver: 0xC,
            device: RTL8852BE.0,
            mmio_phys: MAPPED_MMIO_PHYS,
        };
        assert_eq!(report(&programmed), b"wifi ok\n");
        let missing = Status {
            kind: STATUS_MISSING,
            chip_ver: 0,
            device: 0,
            mmio_phys: 0,
        };
        assert_eq!(report(&missing), b"wifi err\n");
        let mismatch = Status {
            kind: STATUS_BAR_MISMATCH,
            chip_ver: 0,
            device: RTL8852BE.0,
            mmio_phys: 0,
        };
        assert_eq!(report(&mismatch), b"wifi err\n");
    }
}
