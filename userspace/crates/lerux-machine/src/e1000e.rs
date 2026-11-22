//! 82574 and I217-V facts.
//!
//! Register offsets live in the driver engine. This module is the window,
//! the identity, and the ready line. The QEMU image programs `8086:10d3` at
//! `00:04.0`. The desk image programs `8086:153b` at `00:19.0`.

use crate::{table::SpelledAddr, PciAddress};

/// QEMU `e1000e` at `00:04.0`.
pub const QEMU_82574: (u16, u16) = (0x8086, 0x10d3);

/// I217-V recorded on the Z97 at `00:19.0`.
pub const I217_V: (u16, u16) = (0x8086, 0x153b);

/// Revision byte from the 2026-10-22 configuration-space read.
pub const I217_REVISION: u8 = 0;

/// Subsystem vendor and device from that read (`1458:e000`).
pub const I217_SUBSYSTEM: (u16, u16) = (0x1458, 0xe000);

pub const READY_LINE: &str = "e1000e: ready";

/// The QEMU device line sets `disable_vnet_hdr=1` when this is true.
pub const DISABLE_VNET_HDR: bool = true;

/// PCI BAR0, memory, 128 KiB. Config-space dword at `0x10`.
pub const REGISTER_BAR: u8 = 0;
pub const REGISTER_BAR_SIZE: u64 = 0x2_0000;

/// After `X86_NET_BAR1`. A 128 KiB window at `X86_NET_BAR0` would contain that constant.
const REGS_PADDR: SpelledAddr = SpelledAddr::new("0xfed4_0000");
const DMA_PADDR: SpelledAddr = SpelledAddr::new("0x6000_0000");

/// Firmware BAR0 on the desk. 128 KiB, ending at the console image's xHCI window.
const I217_REGS_PADDR: SpelledAddr = SpelledAddr::new("0xf7e0_0000");

/// Firmware flash BAR. The rings stay in BAR0, so the image leaves this unmapped.
pub const I217_FLASH_PADDR: SpelledAddr = SpelledAddr::new("0xf7e3_c000");
pub const I217_FLASH_SIZE: u64 = 0x1000;

/// Firmware I/O BAR. The driver uses the memory register window, as on QEMU.
pub const I217_IO_PORT: u16 = 0xf080;
pub const I217_IO_SIZE: u16 = 32;

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

#[must_use]
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

/// Desk register window at the recorded BAR0. The DMA window matches the QEMU
/// image, so the rings are programmed at the same physical addresses.
#[must_use]
pub const fn i217_map() -> E1000eMap {
    E1000eMap {
        regs_vaddr: I217_REGS_PADDR.value(),
        regs_paddr: I217_REGS_PADDR,
        regs_size: REGISTER_BAR_SIZE,
        regs_region: "e1000e_regs",
        dma_vaddr: 0x8_000_000_000,
        dma_paddr: DMA_PADDR,
        dma_size: 0x200_000,
        dma_region: "e1000e_dma",
    }
}

/// Identity not yet matched. [`UnclaimedNic::compiled`] is the QEMU BDF `00:04.0`.
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

/// I217 claim failed. The slot, revision, or subsystem did not match the record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RejectedI217 {
    pub vendor: u16,
    pub device: u16,
    pub revision: u8,
    pub subsystem_vendor: u16,
    pub subsystem_device: u16,
}

impl UnclaimedNic {
    /// QEMU slot `00:04.0`.
    #[must_use]
    pub const fn qemu() -> Self {
        Self(PciAddress::new(0, 4, 0))
    }

    /// Desk slot `00:19.0`.
    #[must_use]
    pub const fn i217() -> Self {
        Self(PciAddress::new(0, 0x19, 0))
    }

    /// The QEMU BDF. The I217 image uses [`UnclaimedNic::i217`].
    #[must_use]
    pub const fn compiled() -> Self {
        Self::qemu()
    }

    #[must_use]
    pub const fn address(self) -> PciAddress {
        self.0
    }

    const fn is_qemu_slot(self) -> bool {
        self.0.bus() == 0 && self.0.device() == 4 && self.0.function() == 0
    }

    const fn is_i217_slot(self) -> bool {
        self.0.bus() == 0 && self.0.device() == 0x19 && self.0.function() == 0
    }

    /// Vendor and device at the QEMU slot. There is no class argument.
    ///
    /// # Errors
    ///
    /// Returns [`RejectedId`] when the slot is not `00:04.0` or the id is not
    /// the QEMU 82574.
    pub const fn claim(self, vendor: u16, device: u16) -> Result<ClaimedNic, RejectedId> {
        if self.is_qemu_slot() && vendor == QEMU_82574.0 && device == QEMU_82574.1 {
            Ok(ClaimedNic(self.0))
        } else {
            Err(RejectedId { vendor, device })
        }
    }

    /// Vendor, device, revision, and subsystem at `00:19.0`.
    ///
    /// # Errors
    ///
    /// Returns [`RejectedI217`] when the slot or any recorded field differs.
    pub const fn claim_i217(
        self,
        vendor: u16,
        device: u16,
        revision: u8,
        subsystem_vendor: u16,
        subsystem_device: u16,
    ) -> Result<ClaimedNic, RejectedI217> {
        if self.is_i217_slot()
            && vendor == I217_V.0
            && device == I217_V.1
            && revision == I217_REVISION
            && subsystem_vendor == I217_SUBSYSTEM.0
            && subsystem_device == I217_SUBSYSTEM.1
        {
            Ok(ClaimedNic(self.0))
        } else {
            Err(RejectedI217 {
                vendor,
                device,
                revision,
                subsystem_vendor,
                subsystem_device,
            })
        }
    }
}

impl ClaimedNic {
    #[must_use]
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
        assert_eq!(
            UnclaimedNic::i217().claim(0x8086, 0x10d3),
            Err(RejectedId {
                vendor: 0x8086,
                device: 0x10d3,
            })
        );
    }

    #[test]
    fn claim_i217_accepts_the_recorded_desk_nic() {
        let claimed = UnclaimedNic::i217()
            .claim_i217(0x8086, 0x153b, 0, 0x1458, 0xe000)
            .expect("I217-V");
        assert_eq!(claimed.address().bus(), 0);
        assert_eq!(claimed.address().device(), 0x19);
        assert_eq!(claimed.address().function(), 0);
        assert_eq!(I217_V, (0x8086, 0x153b));
        assert_eq!(I217_REVISION, 0);
        assert_eq!(I217_SUBSYSTEM, (0x1458, 0xe000));
    }

    #[test]
    fn claim_i217_rejects_wrong_slot_revision_and_subsystem() {
        assert!(UnclaimedNic::compiled()
            .claim_i217(0x8086, 0x153b, 0, 0x1458, 0xe000)
            .is_err());
        assert!(UnclaimedNic::i217()
            .claim_i217(0x8086, 0x153b, 1, 0x1458, 0xe000)
            .is_err());
        assert!(UnclaimedNic::i217()
            .claim_i217(0x8086, 0x153b, 0, 0x1458, 0)
            .is_err());
        assert!(UnclaimedNic::i217()
            .claim_i217(0x8086, 0x10d3, 0, 0x1458, 0xe000)
            .is_err());
    }

    #[test]
    fn e1000e_map_windows() {
        let map = e1000e_map();
        assert_eq!(map.regs_vaddr, 0xfed4_0000);
        assert_eq!(map.regs_paddr.text(), "0xfed4_0000");
        assert_eq!(map.regs_paddr.value(), 0xfed4_0000);
        assert_device_windows_exclude_foreign_constants();
        assert_eq!(map.regs_size, 0x2_0000);
        assert_eq!(map.regs_region, "e1000e_regs");
        assert_eq!(map.dma_vaddr, 0x8_000_000_000);
        assert_eq!(map.dma_paddr.text(), "0x6000_0000");
        assert_eq!(map.dma_paddr.value(), 0x6000_0000);
        assert_eq!(map.dma_size, 0x200_000);
        assert_eq!(map.dma_region, "e1000e_dma");
    }

    #[test]
    fn i217_map_uses_the_recorded_register_window() {
        let map = i217_map();
        let qemu = e1000e_map();
        assert_eq!(map.regs_paddr.text(), "0xf7e0_0000");
        assert_eq!(map.regs_paddr.value(), 0xf7e0_0000);
        assert_eq!(map.regs_vaddr, 0xf7e0_0000);
        assert_eq!(map.regs_size, REGISTER_BAR_SIZE);
        assert_eq!(map.regs_paddr.value() + map.regs_size, 0xf7e2_0000);
        assert!(map.regs_paddr.value() + map.regs_size <= I217_FLASH_PADDR.value());
        assert_eq!(I217_FLASH_SIZE, 0x1000);
        assert_eq!(I217_IO_PORT, 0xf080);
        assert_eq!(I217_IO_SIZE, 32);
        assert_eq!(map.dma_paddr, qemu.dma_paddr);
        assert_eq!(map.dma_vaddr, qemu.dma_vaddr);
        assert_eq!(map.dma_size, qemu.dma_size);
        assert_ne!(map.regs_paddr.value(), qemu.regs_paddr.value());
    }

    #[test]
    fn e1000e_windows_stay_off_ahci() {
        let map = e1000e_map();
        let ahci = ahci_map();
        assert_ne!(map.dma_paddr.value(), ahci.dma_paddr.value());
        assert_ne!(map.regs_paddr.value(), ahci.abar_paddr.value());
        assert_eq!(map.regs_paddr.value() + map.regs_size, 0xfed6_0000);
        assert!(map.regs_paddr.value() + map.regs_size < ahci.abar_paddr.value());
    }

    #[test]
    fn ready_line_and_bar_index() {
        assert_eq!(READY_LINE, "e1000e: ready");
        assert_eq!(REGISTER_BAR, 0);
        assert_eq!(QEMU_82574, (0x8086, 0x10d3));
        const { assert!(DISABLE_VNET_HDR) };
    }

    fn always_address_constants() -> [(&'static str, u64); 12] {
        use crate::table::{
            X86_BLK_BAR0, X86_BLK_BAR1, X86_NET_BAR0, X86_NET_BAR1, X86_PCI_ECAM_PHYS,
            X86_VIRTIO_BLK_DMA_PHYS, X86_VIRTIO_NET_DMA_PHYS,
        };
        let nic = e1000e_map();
        let desk = i217_map();
        let ahci = ahci_map();
        [
            ("X86_PCI_ECAM_PHYS", X86_PCI_ECAM_PHYS.value()),
            ("X86_BLK_BAR0", X86_BLK_BAR0.value()),
            ("X86_BLK_BAR1", X86_BLK_BAR1.value()),
            ("X86_NET_BAR0", X86_NET_BAR0.value()),
            ("X86_NET_BAR1", X86_NET_BAR1.value()),
            ("X86_VIRTIO_NET_DMA_PHYS", X86_VIRTIO_NET_DMA_PHYS.value()),
            ("X86_VIRTIO_BLK_DMA_PHYS", X86_VIRTIO_BLK_DMA_PHYS.value()),
            ("e1000e regs", nic.regs_paddr.value()),
            ("e1000e dma", nic.dma_paddr.value()),
            ("i217 regs", desk.regs_paddr.value()),
            ("ahci abar", ahci.abar_paddr.value()),
            ("ahci dma", ahci.dma_paddr.value()),
        ]
    }

    #[cfg(feature = "system-description")]
    fn description_address_constants() -> [(&'static str, u64); 8] {
        use crate::table::{
            AARCH64_UART_PHYS, AARCH64_VIRTIO_WINDOW_PHYS, RISCV64_BLK_PHYS, RISCV64_GPU_PHYS,
            RISCV64_NET_PHYS, RISCV64_UART_PHYS, X86_PCI_CONFIG_PORT, X86_UART_PORT,
        };
        [
            ("AARCH64_UART_PHYS", AARCH64_UART_PHYS.value()),
            ("RISCV64_UART_PHYS", RISCV64_UART_PHYS.value()),
            ("X86_UART_PORT", X86_UART_PORT.value()),
            (
                "AARCH64_VIRTIO_WINDOW_PHYS",
                AARCH64_VIRTIO_WINDOW_PHYS.value(),
            ),
            ("RISCV64_BLK_PHYS", RISCV64_BLK_PHYS.value()),
            ("RISCV64_NET_PHYS", RISCV64_NET_PHYS.value()),
            ("RISCV64_GPU_PHYS", RISCV64_GPU_PHYS.value()),
            ("X86_PCI_CONFIG_PORT", X86_PCI_CONFIG_PORT.value()),
        ]
    }

    fn hit_in(base: u64, end: u64, constants: &[(&'static str, u64)]) -> Option<&'static str> {
        constants
            .iter()
            .find_map(|&(label, value)| (value > base && value < end).then_some(label))
    }

    fn foreign_inside(base: u64, size: u64) -> Option<&'static str> {
        let end = base.checked_add(size).expect("window end");
        let hit = hit_in(base, end, &always_address_constants());
        #[cfg(feature = "system-description")]
        if hit.is_none() {
            return hit_in(base, end, &description_address_constants());
        }
        hit
    }

    fn assert_device_windows_exclude_foreign_constants() {
        let nic = e1000e_map();
        let desk = i217_map();
        let ahci = ahci_map();
        let windows = [
            ("e1000e regs", nic.regs_paddr.value(), nic.regs_size),
            ("i217 regs", desk.regs_paddr.value(), desk.regs_size),
            ("e1000e dma", nic.dma_paddr.value(), nic.dma_size),
            ("ahci abar", ahci.abar_paddr.value(), ahci.abar_size),
            ("ahci dma", ahci.dma_paddr.value(), ahci.dma_size),
            (
                "ecam",
                crate::X86_PCI_ECAM_PHYS.value(),
                crate::X86_PCI_ECAM_SIZE,
            ),
        ];
        for (name, base, size) in windows {
            let hit = foreign_inside(base, size);
            assert!(hit.is_none(), "{name} contains {hit:?}");
        }
        assert_eq!(foreign_inside(0xfed2_0000, 0x2_0000), Some("X86_NET_BAR1"));
    }
}
