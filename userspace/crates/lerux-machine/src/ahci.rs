//! Advanced Host Controller Interface facts for QEMU q35.

use crate::table::SpelledAddr;

pub const QEMU_ICH9_AHCI: (u16, u16) = (0x8086, 0x2922);
pub const AHCI_CLASS: u32 = 0x01_06_01;
pub const AHCI_PORT: u8 = 0;
pub const READY_LINE: &str = "ahci: port 0";
/// Guest vector chosen beside serial 48 and virtio block 49; not a measured pin.
pub const AHCI_IOAPIC_VECTOR: u8 = 51;

/// Local APIC address written into an MSI capability.
pub const MSI_ADDRESS: u32 = 0xfee0_0000;

/// seL4 delivers SDF vector V as V + 0x30. The kernel does not rewrite the message.
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

    pub const fn bus(self) -> u8 {
        self.bus
    }

    pub const fn device(self) -> u8 {
        self.device
    }

    pub const fn function(self) -> u8 {
        self.function
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unclaimed(PciAddress);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Claimed(PciAddress);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RejectedClass {
    pub found: u32,
}

impl Unclaimed {
    pub const fn compiled() -> Self {
        Self(PciAddress::new(0, 0x1f, 2))
    }

    pub const fn address(self) -> PciAddress {
        self.0
    }

    pub const fn claim(self, class: u32) -> Result<Claimed, RejectedClass> {
        if class == AHCI_CLASS {
            Ok(Claimed(self.0))
        } else {
            Err(RejectedClass { found: class })
        }
    }
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
    }

    #[test]
    fn claim_accepts_ahci_class() {
        assert!(Unclaimed::compiled().claim(AHCI_CLASS).is_ok());
    }

    #[test]
    fn claim_rejects_other_class() {
        assert_eq!(
            Unclaimed::compiled().claim(0x010600),
            Err(RejectedClass { found: 0x010600 })
        );
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
