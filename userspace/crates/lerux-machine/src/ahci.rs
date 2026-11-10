//! Advanced Host Controller Interface facts for QEMU q35.
// The verifier's expansion names `vstd` and `int`. A normal build
// erases that expansion, so the imports are unused unless `verus`
// is verifying. Protection-domain builds leave the feature off.
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use verus_builtin::*;
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::prelude::*;

use crate::table::SpelledAddr;

verus! {

pub const QEMU_ICH9_AHCI: (u16, u16) = (0x8086, 0x2922);

pub const AHCI_CLASS: u32 = 0x01_06_01;

/// Guest vector chosen beside serial 48 and virtio block 49; not a measured pin.
pub const AHCI_IOAPIC_VECTOR: u8 = 51;

/// seL4 delivers SDF vector V as V + 0x30. The kernel does not rewrite the message.
#[must_use]
pub const fn msi_data(sdf_vector: u8) -> (data: u32)
    ensures
        data == (sdf_vector as u32) + 0x30,
        sdf_vector == 51 ==> data == 0x63,
        sdf_vector == AHCI_IOAPIC_VECTOR ==> data == 0x63,
{
    proof {
        assert(AHCI_IOAPIC_VECTOR == 51);
        assert(sdf_vector == 51 ==> (sdf_vector as u32) + 0x30 == 0x63) by (bit_vector);
    }
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
        Self { bus, device, function }
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
pub struct Claimed(PciAddress);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RejectedClass {
    pub found: u32,
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

    /// Accept the compiled address when `class` is [`AHCI_CLASS`].
    ///
    /// # Errors
    ///
    /// Returns [`RejectedClass`] when `class` is not [`AHCI_CLASS`].
    pub const fn claim(self, class: u32) -> (result: Result<Claimed, RejectedClass>)
        ensures
            (result is Ok) <==> (class == AHCI_CLASS),
            result matches Err(RejectedClass { found }) ==> found == class,
    {
        if class == AHCI_CLASS {
            Ok(Claimed(self.0))
        } else {
            Err(RejectedClass { found: class })
        }
    }
}

/// QEMU's ICH9 AHCI controller stays vendor `0x8086`, device `0x2922`.
pub proof fn lemma_qemu_ich9_ahci()
    ensures
        QEMU_ICH9_AHCI == (0x8086u16, 0x2922u16),
{
}

/// The class code accepted by [`Unclaimed::claim`] is `0x01_06_01`.
pub proof fn lemma_ahci_class()
    ensures
        AHCI_CLASS == 0x01_06_01,
{
}

/// SDF vector 51 is the recorded AHCI vector, and its MSI data word is `0x63`.
pub proof fn lemma_msi_data_recorded_vector()
    ensures
        AHCI_IOAPIC_VECTOR == 51,
        (AHCI_IOAPIC_VECTOR as u32) + 0x30 == 0x63,
{
    assert((51u8 as u32) + 0x30 == 0x63) by (bit_vector);
}

} // verus!
pub const AHCI_PORT: u8 = 0;
pub const READY_LINE: &str = "ahci: port 0";

/// Local APIC address written into an MSI capability.
pub const MSI_ADDRESS: u32 = 0xfee0_0000;

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
    }

    #[test]
    fn claim_accepts_ahci_class() {
        assert!(Unclaimed::compiled().claim(AHCI_CLASS).is_ok());
    }

    #[test]
    fn claim_rejects_other_class() {
        assert_eq!(
            Unclaimed::compiled().claim(0x01_06_00),
            Err(RejectedClass { found: 0x01_06_00 })
        );
    }

    #[test]
    fn msi_data_matches_sel4_irq_offset() {
        const RECORDED: u32 = msi_data(51);
        assert_eq!(msi_data(50), 98);
        assert_eq!(msi_data(AHCI_IOAPIC_VECTOR), 99);
        assert_eq!(RECORDED, 0x63);
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
