//! Advanced Host Controller Interface facts for QEMU q35 and the Z97 desk.

use crate::table::SpelledAddr;

pub const QEMU_ICH9_AHCI: (u16, u16) = (0x8086, 0x2922);
/// 9-series AHCI on the Gigabyte GA-Z97-D3H, function `00:1f.2`.
///
/// Published probes of this board carry the same I217 subsystem `1458:e000`
/// recorded for this desk, and they show `8086:8c82`, class [`AHCI_CLASS`],
/// subsystem `1458:b005`. This desk's captured root port `8086:8c9c` is the
/// 9-series port on that chipset. The claim does not read the subsystem.
pub const Z97_AHCI: (u16, u16) = (0x8086, 0x8c82);
pub const AHCI_CLASS: u32 = 0x01_06_01;
/// Byte offset of the IDENTIFY model, words 27..46, still in ATA word order.
pub const ATA_MODEL_OFFSET: usize = 54;
pub const ATA_MODEL_BYTES: usize = 40;
/// Room for `ahci: VVVV:DDDD` plus one space and a 40-byte model.
pub const IDENTITY_CAP: usize = 64;
pub const AHCI_PORT: u8 = 0;
pub const READY_LINE: &str = "ahci: port 0";
/// Guest vector chosen beside serial 48 and virtio block 49; not a measured pin.
pub const AHCI_IOAPIC_VECTOR: u8 = 51;

/// Local APIC address written into an MSI capability.
pub const MSI_ADDRESS: u32 = 0xfee0_0000;

/// seL4 delivers SDF vector V as V + 0x30. The kernel does not rewrite the message.
#[must_use]
pub const fn msi_data(sdf_vector: u8) -> u32 {
    (sdf_vector as u32) + 0x30
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PciAddress {
    bus: u8,
    device: u8,
    function: u8,
}

impl PciAddress {
    pub(crate) const fn new(bus: u8, device: u8, function: u8) -> Self {
        Self {
            bus,
            device,
            function,
        }
    }

    #[must_use]
    pub const fn bus(self) -> u8 {
        self.bus
    }

    #[must_use]
    pub const fn device(self) -> u8 {
        self.device
    }

    #[must_use]
    pub const fn function(self) -> u8 {
        self.function
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unclaimed(PciAddress);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Claimed {
    address: PciAddress,
    vendor: u16,
    device: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RejectedId {
    pub class: u32,
    pub vendor: u16,
    pub device: u16,
}

/// IDENTIFY model with the ATA word swap undone and the trailing pad removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtaModel {
    bytes: [u8; ATA_MODEL_BYTES],
    len: usize,
}

impl Unclaimed {
    #[must_use]
    pub const fn compiled() -> Self {
        Self(PciAddress::new(0, 0x1f, 2))
    }

    #[must_use]
    pub const fn address(self) -> PciAddress {
        self.0
    }

    /// Accept [`QEMU_ICH9_AHCI`] or [`Z97_AHCI`] when `class` is [`AHCI_CLASS`].
    ///
    /// # Errors
    ///
    /// Returns [`RejectedId`] for any other class or vendor/device pair.
    pub const fn claim(self, class: u32, vendor: u16, device: u16) -> Result<Claimed, RejectedId> {
        if class == AHCI_CLASS && accepted_id(vendor, device) {
            Ok(Claimed {
                address: self.0,
                vendor,
                device,
            })
        } else {
            Err(RejectedId {
                class,
                vendor,
                device,
            })
        }
    }
}

const fn accepted_id(vendor: u16, device: u16) -> bool {
    (vendor == QEMU_ICH9_AHCI.0 && device == QEMU_ICH9_AHCI.1)
        || (vendor == Z97_AHCI.0 && device == Z97_AHCI.1)
}

impl Claimed {
    #[must_use]
    pub const fn address(self) -> PciAddress {
        self.address
    }

    #[must_use]
    pub const fn vendor(self) -> u16 {
        self.vendor
    }

    #[must_use]
    pub const fn device(self) -> u16 {
        self.device
    }

    /// The QEMU smoke volume is writable. The desk volume is identified and not written.
    #[must_use]
    pub const fn allows_write(self) -> bool {
        self.vendor == QEMU_ICH9_AHCI.0 && self.device == QEMU_ICH9_AHCI.1
    }
}

impl AtaModel {
    /// `raw` is the 40 model bytes as IDENTIFY stored them, each word swapped.
    #[must_use]
    pub const fn from_swapped(raw: &[u8; ATA_MODEL_BYTES]) -> Self {
        let mut bytes = [0u8; ATA_MODEL_BYTES];
        let mut i = 0;
        while i < ATA_MODEL_BYTES {
            bytes[i] = raw[i ^ 1];
            i += 1;
        }
        Self {
            bytes,
            len: model_len(&bytes),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

const fn model_len(bytes: &[u8; ATA_MODEL_BYTES]) -> usize {
    let mut end = ATA_MODEL_BYTES;
    while end > 0 {
        let b = bytes[end - 1];
        if b != b' ' && b != 0 {
            break;
        }
        end -= 1;
    }
    let mut n = 0;
    while n < end {
        let b = bytes[n];
        if b < 0x20 || b > 0x7e {
            break;
        }
        n += 1;
    }
    n
}

/// Write `ahci: VVVV:DDDD` and, when `model` is non-empty, a space and the model.
///
/// Returns `None` when `out` is shorter than the line or `model` is not printable ASCII.
#[must_use]
pub fn format_identity(vendor: u16, device: u16, model: &str, out: &mut [u8]) -> Option<usize> {
    let model = model.as_bytes();
    if model.iter().any(|b| !(0x20..=0x7e).contains(b)) {
        return None;
    }
    let extra = if model.is_empty() { 0 } else { 1 + model.len() };
    let len = 15 + extra;
    if out.len() < len {
        return None;
    }
    out[..6].copy_from_slice(b"ahci: ");
    write_hex4(out, 6, vendor);
    out[10] = b':';
    write_hex4(out, 11, device);
    if !model.is_empty() {
        out[15] = b' ';
        out[16..len].copy_from_slice(model);
    }
    Some(len)
}

fn write_hex4(out: &mut [u8], at: usize, value: u16) {
    const HEX: [u8; 16] = *b"0123456789abcdef";
    out[at] = HEX[usize::from((value >> 12) & 0xf)];
    out[at + 1] = HEX[usize::from((value >> 8) & 0xf)];
    out[at + 2] = HEX[usize::from((value >> 4) & 0xf)];
    out[at + 3] = HEX[usize::from(value & 0xf)];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AhciMap {
    pub abar_vaddr: u64,
    pub abar_paddr: SpelledAddr,
    pub abar_size: u64,
    pub abar_vaddr_setvar: &'static str,
    pub abar_paddr_setvar: &'static str,
    pub dma_vaddr: u64,
    pub dma_paddr: SpelledAddr,
    pub dma_size: u64,
    pub dma_vaddr_setvar: &'static str,
    pub dma_paddr_setvar: &'static str,
}

const ABAR_PADDR: SpelledAddr = SpelledAddr::new("0xfed8_0000");
const DMA_PADDR: SpelledAddr = SpelledAddr::new("0x6400_0000");

#[must_use]
pub const fn ahci_map() -> AhciMap {
    AhciMap {
        abar_vaddr: ABAR_PADDR.value(),
        abar_paddr: ABAR_PADDR,
        abar_size: 0x1000,
        abar_vaddr_setvar: "ahci_abar_vaddr",
        abar_paddr_setvar: "ahci_abar_paddr",
        dma_vaddr: 0x8_000_000_000,
        dma_paddr: DMA_PADDR,
        dma_size: 0x2_0000,
        dma_vaddr_setvar: "ahci_dma_vaddr",
        dma_paddr_setvar: "ahci_dma_paddr",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ich9_vendor_device_id() {
        assert_eq!(QEMU_ICH9_AHCI, (0x8086, 0x2922));
        assert_eq!(Z97_AHCI, (0x8086, 0x8c82));
    }

    #[test]
    fn claim_accepts_qemu_and_z97() {
        let qemu = Unclaimed::compiled()
            .claim(AHCI_CLASS, 0x8086, 0x2922)
            .expect("qemu");
        assert_eq!(qemu.address(), Unclaimed::compiled().address());
        assert!(qemu.allows_write());
        let z97 = Unclaimed::compiled()
            .claim(AHCI_CLASS, 0x8086, 0x8c82)
            .expect("z97");
        assert_eq!((z97.vendor(), z97.device()), Z97_AHCI);
        assert!(!z97.allows_write());
    }

    #[test]
    fn claim_rejects_other_controllers() {
        assert_eq!(
            Unclaimed::compiled().claim(0x01_06_00, 0x8086, 0x2922),
            Err(RejectedId {
                class: 0x01_06_00,
                vendor: 0x8086,
                device: 0x2922,
            })
        );
        for device in [0x3b2f_u16, 0x8c02, 0x8d02, 0x2923] {
            assert_eq!(
                Unclaimed::compiled().claim(AHCI_CLASS, 0x8086, device),
                Err(RejectedId {
                    class: AHCI_CLASS,
                    vendor: 0x8086,
                    device,
                })
            );
        }
    }

    #[test]
    fn identity_line_names_the_controller_and_the_drive() {
        let swapped = swapped_model(b"QEMU HARDDISK");
        let model = AtaModel::from_swapped(&swapped);
        assert_eq!(model.as_str(), "QEMU HARDDISK");
        let mut line = [0u8; IDENTITY_CAP];
        let n = format_identity(0x8086, 0x2922, model.as_str(), &mut line).expect("line");
        assert_eq!(&line[..n], b"ahci: 8086:2922 QEMU HARDDISK");
        let n = format_identity(0x8086, 0x8c82, "", &mut line).expect("id only");
        assert_eq!(&line[..n], b"ahci: 8086:8c82");
        assert!(format_identity(0x8086, 0x8c82, "bad\nname", &mut line).is_none());
    }

    fn swapped_model(text: &[u8]) -> [u8; ATA_MODEL_BYTES] {
        let mut raw = [b' '; ATA_MODEL_BYTES];
        for (i, byte) in text.iter().enumerate() {
            raw[i ^ 1] = *byte;
        }
        raw
    }

    #[test]
    fn msi_data_matches_sel4_irq_offset() {
        assert_eq!(msi_data(50), 98);
        assert_eq!(msi_data(AHCI_IOAPIC_VECTOR), 99);
    }

    #[test]
    fn ahci_map_windows() {
        let map = ahci_map();
        assert_eq!(map.abar_vaddr, 0xfed8_0000);
        assert_eq!(map.abar_paddr.value(), 0xfed8_0000);
        assert_eq!(map.abar_size, 0x1000);
        assert_eq!(map.abar_vaddr_setvar, "ahci_abar_vaddr");
        assert_eq!(map.abar_paddr_setvar, "ahci_abar_paddr");
        assert_eq!(map.dma_vaddr, 0x8_000_000_000);
        assert_eq!(map.dma_paddr.value(), 0x6400_0000);
        assert_eq!(map.dma_size, 0x2_0000);
        assert_eq!(map.dma_vaddr_setvar, "ahci_dma_vaddr");
        assert_eq!(map.dma_paddr_setvar, "ahci_dma_paddr");
    }
}
