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

/// `E1000_CTRL_SLU` in `drivers/net/ethernet/intel/e1000e/defines.h`.
pub const CTRL_SLU: u32 = 1 << 6;
/// `E1000_CTRL_FRCSPD`.
pub const CTRL_FRCSPD: u32 = 1 << 11;
/// `E1000_CTRL_FRCDPX`.
pub const CTRL_FRCDPX: u32 = 1 << 12;
/// `E1000_STATUS_LU`.
pub const STATUS_LU: u32 = 1 << 1;
/// Receive-address valid bit in RAH.
pub const RAH_AV: u32 = 1 << 31;

pub const REG_CTRL: u32 = 0x0000;
pub const REG_STATUS: u32 = 0x0008;
pub const REG_RDH: u32 = 0x2810;
pub const REG_RDT: u32 = 0x2818;
pub const REG_TDH: u32 = 0x3810;
pub const REG_TDT: u32 = 0x3818;
pub const REG_RAL0: u32 = 0x5400;
pub const REG_RAH0: u32 = 0x5404;

/// Byte offset of [`PingDiag`] in the I217 DMA window, past the descriptor rings.
pub const PING_DIAG_OFF: usize = 0x10_0000;
/// `LXD1`. Zero means the driver did not finish `open`.
pub const PING_DIAG_MAGIC: u32 = 0x4c58_4431;
/// One console row is 80 columns. A trace line stays on one row.
pub const PING_LINE_MAX: usize = 79;

/// How to read a `ping` trace row. Printed again above a failing result.
pub const PING_READ: &[u8] = b"ping read tx>sub queue-drop sub>tdh nic-held lu=0 no-link";
/// `lu` is STATUS bit 1. `slu` is CTRL.SLU. `frc` is force-speed or force-duplex.
pub const PING_LEGEND_BITS: &[u8] =
    b"ping legend lu=link slu=set-link-up frc=forced-speed-or-duplex";
/// `td` is TDT/TDH. `rd` is RDT/RDH. `sub` rang the NIC. `dl`/`df` are driver drops.
pub const PING_LEGEND_Q: &[u8] =
    b"ping legend td=TDT/TDH rd=RDT/RDH sub=nic-tx dl=oversize df=ring-full";

/// Driver counters at [`PING_DIAG_OFF`]. The shell reads them while `ping` waits.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PingDiag {
    pub magic: u32,
    pub submit: u32,
    pub drop_len: u32,
    pub drop_full: u32,
    pub rx_ok: u32,
    pub rx_drop: u32,
    pub last_icr: u32,
    pub last_tx_len: u32,
    pub last_rx_len: u32,
    pub last_rx_status: u32,
    pub last_rx_errors: u32,
}

const _: () = {
    assert!(core::mem::size_of::<PingDiag>().is_multiple_of(4));
    assert!(PING_DIAG_OFF + core::mem::size_of::<PingDiag>() <= 0x200_000);
    assert!(PING_READ.len() <= PING_LINE_MAX);
    assert!(PING_LEGEND_BITS.len() <= PING_LINE_MAX);
    assert!(PING_LEGEND_Q.len() <= PING_LINE_MAX);
};

/// One `ping` sample. `td` is transmit tail/head. `rd` is receive tail/head.
#[derive(Clone, Copy)]
pub struct PingPath {
    pub tag: &'static [u8],
    pub ms: u32,
    pub tx: u32,
    pub rx: u32,
    pub tdt: u32,
    pub tdh: u32,
    pub rdt: u32,
    pub rdh: u32,
    pub status: u32,
    pub ctrl: u32,
}

/// Register and driver-counter row. `diag_live` is false when the magic is absent.
#[derive(Clone, Copy)]
pub struct PingRegs {
    pub status: u32,
    pub ctrl: u32,
    pub submit: u32,
    pub drop_len: u32,
    pub drop_full: u32,
    pub icr: u32,
    pub diag_live: bool,
}

/// Last frame the driver copied or rejected. `diag_live` false prints `diag0`.
#[derive(Clone, Copy)]
pub struct PingFrame {
    pub tx_len: u32,
    pub rx_len: u32,
    pub rx_ok: u32,
    pub rx_drop: u32,
    pub rx_status: u32,
    pub rx_errors: u32,
    pub diag_live: bool,
}

/// Write one path row. Counts above 9999 print as 9999 so the row stays on one line.
pub fn format_ping_path(path: &PingPath, out: &mut [u8]) -> usize {
    let mut line = Line { buf: out, n: 0 };
    line.bytes(b"ping ");
    line.bytes(path.tag);
    line.bytes(b" ");
    line.u32(cap(path.ms, 99_999));
    line.bytes(b" tx");
    line.u32(cap(path.tx, 9_999));
    line.bytes(b" rx");
    line.u32(cap(path.rx, 9_999));
    line.bytes(b" td ");
    line.hex_trim(path.tdt);
    line.bytes(b"/");
    line.hex_trim(path.tdh);
    line.bytes(b" rd ");
    line.hex_trim(path.rdt);
    line.bytes(b"/");
    line.hex_trim(path.rdh);
    line.bytes(b" lu");
    line.u32(u32::from(path.status & STATUS_LU != 0));
    line.bytes(b" slu");
    line.u32(u32::from(path.ctrl & CTRL_SLU != 0));
    line.bytes(b" frc");
    line.u32(u32::from(path.ctrl & (CTRL_FRCSPD | CTRL_FRCDPX) != 0));
    line.n
}

/// Write the register row. `diag0` means the driver did not publish [`PingDiag`].
pub fn format_ping_regs(regs: &PingRegs, out: &mut [u8]) -> usize {
    let mut line = Line { buf: out, n: 0 };
    line.bytes(b"ping regs sta ");
    line.hex_width(regs.status, 8);
    line.bytes(b" ctrl ");
    line.hex_width(regs.ctrl, 8);
    if regs.diag_live {
        line.bytes(b" sub");
        line.u32(cap(regs.submit, 9_999));
        line.bytes(b" dl");
        line.u32(cap(regs.drop_len, 9_999));
        line.bytes(b" df");
        line.u32(cap(regs.drop_full, 9_999));
        line.bytes(b" icr ");
        line.hex_width(regs.icr, 8);
    } else {
        line.bytes(b" diag0");
    }
    line.n
}

/// Write the last-frame row.
pub fn format_ping_frame(frame: &PingFrame, out: &mut [u8]) -> usize {
    let mut line = Line { buf: out, n: 0 };
    if !frame.diag_live {
        line.bytes(b"ping frame diag0");
        return line.n;
    }
    line.bytes(b"ping frame txlen ");
    line.u32(cap(frame.tx_len, 9_999));
    line.bytes(b" rxlen ");
    line.u32(cap(frame.rx_len, 9_999));
    line.bytes(b" rxok ");
    line.u32(cap(frame.rx_ok, 9_999));
    line.bytes(b" rxdrop ");
    line.u32(cap(frame.rx_drop, 9_999));
    line.bytes(b" sta ");
    line.hex_width(frame.rx_status, 2);
    line.bytes(b" err ");
    line.hex_width(frame.rx_errors, 2);
    line.n
}

/// `mac` is the address the stack sends. `ral`/`rah` are the receive-address registers.
pub fn format_ping_mac(mac: [u8; 6], ral: u32, rah: u32, out: &mut [u8]) -> usize {
    let mut line = Line { buf: out, n: 0 };
    line.bytes(b"ping mac ");
    line.mac(mac);
    line.bytes(b" ral ");
    line.mac(station(ral, rah));
    line.bytes(b" av ");
    line.u32(u32::from(rah & RAH_AV != 0));
    line.n
}

fn station(ral: u32, rah: u32) -> [u8; 6] {
    let low = ral.to_le_bytes();
    let high = rah.to_le_bytes();
    [low[0], low[1], low[2], low[3], high[0], high[1]]
}

fn cap(value: u32, max: u32) -> u32 {
    if value > max {
        max
    } else {
        value
    }
}

struct Line<'a> {
    buf: &'a mut [u8],
    n: usize,
}

impl Line<'_> {
    fn bytes(&mut self, data: &[u8]) {
        for &byte in data {
            if self.n >= self.buf.len() {
                return;
            }
            self.buf[self.n] = byte;
            self.n += 1;
        }
    }

    fn u32(&mut self, mut value: u32) {
        let mut tmp = [0u8; 10];
        let mut index = tmp.len();
        loop {
            index -= 1;
            let digit = u8::try_from(value % 10).expect("decimal digit");
            tmp[index] = b'0' + digit;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        self.bytes(&tmp[index..]);
    }

    fn hex_trim(&mut self, mut value: u32) {
        let mut tmp = [0u8; 8];
        let mut index = tmp.len();
        loop {
            index -= 1;
            tmp[index] = hex_digit(value & 0xf);
            value >>= 4;
            if value == 0 {
                break;
            }
        }
        self.bytes(&tmp[index..]);
    }

    fn hex_width(&mut self, mut value: u32, width: usize) {
        let mut tmp = [0u8; 8];
        for slot in (0..width).rev() {
            tmp[slot] = hex_digit(value & 0xf);
            value >>= 4;
        }
        self.bytes(&tmp[..width]);
    }

    fn mac(&mut self, mac: [u8; 6]) {
        for byte in mac {
            self.hex_width(u32::from(byte), 2);
        }
    }
}

fn hex_digit(nibble: u32) -> u8 {
    let nibble = u8::try_from(nibble & 0xf).expect("hex digit");
    if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + (nibble - 10)
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

    #[test]
    fn ping_trace_rows_name_the_link_and_the_rings() {
        assert_eq!(CTRL_SLU, 0x40);
        assert_eq!(CTRL_FRCSPD, 0x800);
        assert_eq!(CTRL_FRCDPX, 0x1000);
        assert_eq!(STATUS_LU, 2);
        let mut buf = [0u8; 96];
        let n = format_ping_path(
            &PingPath {
                tag: b"wait",
                ms: 4000,
                tx: 1,
                rx: 0,
                tdt: 1,
                tdh: 0,
                rdt: 0x1f,
                rdh: 0,
                status: 0x8008_0083,
                ctrl: CTRL_SLU,
            },
            &mut buf,
        );
        assert_eq!(
            &buf[..n],
            b"ping wait 4000 tx1 rx0 td 1/0 rd 1f/0 lu1 slu1 frc0"
        );
        assert!(n <= PING_LINE_MAX);
        let n = format_ping_path(
            &PingPath {
                tag: b"timeout",
                ms: 99_999,
                tx: 9_999,
                rx: 9_999,
                tdt: 0xffff,
                tdh: 0xffff,
                rdt: 0xffff,
                rdh: 0xffff,
                status: 0xffff_ffff,
                ctrl: CTRL_SLU | CTRL_FRCSPD | CTRL_FRCDPX,
            },
            &mut buf,
        );
        assert!(n <= PING_LINE_MAX);
    }

    #[test]
    fn ping_trace_rows_name_the_driver_counters() {
        let mut buf = [0u8; 96];
        let regs = PingRegs {
            status: 0x8008_0083,
            ctrl: CTRL_SLU,
            submit: 1,
            drop_len: 0,
            drop_full: 0,
            icr: 0,
            diag_live: true,
        };
        let n = format_ping_regs(&regs, &mut buf);
        assert_eq!(
            &buf[..n],
            b"ping regs sta 80080083 ctrl 00000040 sub1 dl0 df0 icr 00000000"
        );
        assert!(n <= PING_LINE_MAX);
        let n = format_ping_regs(
            &PingRegs {
                diag_live: false,
                ..regs
            },
            &mut buf,
        );
        assert_eq!(&buf[..n], b"ping regs sta 80080083 ctrl 00000040 diag0");
    }

    #[test]
    fn ping_trace_frame_row_names_the_last_copy() {
        let mut buf = [0u8; 96];
        let n = format_ping_frame(
            &PingFrame {
                tx_len: 342,
                rx_len: 0,
                rx_ok: 0,
                rx_drop: 0,
                rx_status: 0,
                rx_errors: 0,
                diag_live: true,
            },
            &mut buf,
        );
        assert_eq!(
            &buf[..n],
            b"ping frame txlen 342 rxlen 0 rxok 0 rxdrop 0 sta 00 err 00"
        );
        assert!(n <= PING_LINE_MAX);
        let n = format_ping_frame(
            &PingFrame {
                tx_len: 0,
                rx_len: 0,
                rx_ok: 0,
                rx_drop: 0,
                rx_status: 0,
                rx_errors: 0,
                diag_live: false,
            },
            &mut buf,
        );
        assert_eq!(&buf[..n], b"ping frame diag0");
    }

    #[test]
    fn ping_trace_mac_is_the_wire_order() {
        let mut buf = [0u8; 96];
        let n = format_ping_mac(
            [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
            0xc814_aafc,
            RAH_AV | 0x9e00,
            &mut buf,
        );
        assert_eq!(&buf[..n], b"ping mac 525400123456 ral fcaa14c8009e av 1");
        assert!(n <= PING_LINE_MAX);
    }
}
