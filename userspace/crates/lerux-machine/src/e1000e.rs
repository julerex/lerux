//! 82574 facts for the standalone QEMU q35 smoke.
//!
//! Register offsets live in the driver engine. This module is the window,
//! the identity, and the ready line.

use crate::{table::SpelledAddr, PciAddress};

/// QEMU `e1000e`. Milestone 22's I217 (`8086:153b`) is not accepted here.
pub const QEMU_82574: (u16, u16) = (0x8086, 0x10d3);

pub const READY_LINE: &str = "e1000e: ready";

/// The QEMU device line sets `disable_vnet_hdr=on` when this is true.
pub const DISABLE_VNET_HDR: bool = true;

/// PCI BAR0, memory, 128 KiB. Config-space dword at `0x10`.
pub const REGISTER_BAR: u8 = 0;
pub const REGISTER_BAR_SIZE: u64 = 0x2_0000;

const REGS_PADDR: SpelledAddr = SpelledAddr::new("0xfed2_0000");
const DMA_PADDR: SpelledAddr = SpelledAddr::new("0x6000_0000");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct E1000eMap {
    pub regs_vaddr: u64,
    pub regs_paddr: SpelledAddr,
    pub regs_size: u64,
    pub regs_region: &'static str,
    pub dma_vaddr: u64,
    pub dma_paddr: SpelledAddr,
    pub dma_size: u64,
    pub dma_region: &'static str,
}

pub const fn e1000e_map() -> E1000eMap {
    E1000eMap {
        regs_vaddr: REGS_PADDR.value(),
        regs_paddr: REGS_PADDR,
        regs_size: REGISTER_BAR_SIZE,
        regs_region: "e1000e_regs",
        dma_vaddr: 0x8_000_000_000,
        dma_paddr: DMA_PADDR,
        dma_size: 0x200_000,
        dma_region: "e1000e_dma",
    }
}

/// Identity not yet matched. The compiled BDF is `00:04.0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnclaimedNic(PciAddress);

/// `claim` succeeded. The engine's programmer takes this token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClaimedNic(PciAddress);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RejectedId {
    pub vendor: u16,
    pub device: u16,
}

impl UnclaimedNic {
    pub const fn compiled() -> Self {
        Self(PciAddress::new(0, 4, 0))
    }

    pub const fn address(self) -> PciAddress {
        self.0
    }

    /// Vendor and device only. There is no class argument.
    pub const fn claim(self, vendor: u16, device: u16) -> Result<ClaimedNic, RejectedId> {
        if vendor == QEMU_82574.0 && device == QEMU_82574.1 {
            Ok(ClaimedNic(self.0))
        } else {
            Err(RejectedId { vendor, device })
        }
    }
}

impl ClaimedNic {
    pub const fn address(self) -> PciAddress {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ahci_map;

    #[test]
    fn claim_accepts_82574() {
        let claimed = UnclaimedNic::compiled()
            .claim(0x8086, 0x10d3)
            .expect("82574");
        assert_eq!(claimed.address().bus(), 0);
        assert_eq!(claimed.address().device(), 4);
        assert_eq!(claimed.address().function(), 0);
        assert_eq!(claimed.address(), UnclaimedNic::compiled().address());
    }

    #[test]
    fn claim_rejects_other_intel_nics() {
        assert_eq!(
            UnclaimedNic::compiled().claim(0x8086, 0x153b),
            Err(RejectedId {
                vendor: 0x8086,
                device: 0x153b,
            })
        );
        assert_eq!(
            UnclaimedNic::compiled().claim(0x8086, 0x100e),
            Err(RejectedId {
                vendor: 0x8086,
                device: 0x100e,
            })
        );
    }

    #[test]
    fn e1000e_map_windows() {
        let map = e1000e_map();
        assert_eq!(map.regs_vaddr, 0xfed2_0000);
        assert_eq!(map.regs_paddr.text(), "0xfed2_0000");
        assert_eq!(map.regs_paddr.value(), 0xfed2_0000);
        assert_eq!(map.regs_size, 0x2_0000);
        assert_eq!(map.regs_region, "e1000e_regs");
        assert_eq!(map.dma_vaddr, 0x8_000_000_000);
        assert_eq!(map.dma_paddr.text(), "0x6000_0000");
        assert_eq!(map.dma_paddr.value(), 0x6000_0000);
        assert_eq!(map.dma_size, 0x200_000);
        assert_eq!(map.dma_region, "e1000e_dma");
    }

    #[test]
    fn e1000e_windows_stay_off_ahci() {
        let map = e1000e_map();
        let ahci = ahci_map();
        assert_ne!(map.dma_paddr.value(), ahci.dma_paddr.value());
        assert_ne!(map.regs_paddr.value(), ahci.abar_paddr.value());
        assert_eq!(map.regs_paddr.value() + map.regs_size, 0xfed4_0000);
        assert!(map.regs_paddr.value() + map.regs_size < ahci.abar_paddr.value());
    }

    #[test]
    fn ready_line_and_bar_index() {
        assert_eq!(READY_LINE, "e1000e: ready");
        assert_eq!(REGISTER_BAR, 0);
        assert_eq!(QEMU_82574, (0x8086, 0x10d3));
        const { assert!(DISABLE_VNET_HDR) };
    }
}
