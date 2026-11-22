//! `mt7921/regs.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT_MDP_BASE: u64 = 0x820c_d000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MDP(ofs: u64) -> u64 { (MT_MDP_BASE).wrapping_add(ofs) }

pub const MT_MDP_DCR0: u64 = 0x820c_d000;
pub const MT_MDP_DCR0_DAMSDU_EN: u64 = 0x8000;
pub const MT_MDP_DCR0_RX_HDR_TRANS_EN: u64 = 0x8_0000;
pub const MT_MDP_DCR1: u64 = 0x820c_d004;
pub const MT_MDP_DCR1_MAX_RX_LEN: u64 = 0xfff8;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MDP_BNRCFR0(band: u64) -> u64 { MT_MDP((112u64).wrapping_add(shl(band, 8u64))) }

pub const MT_MDP_RCFR0_MCU_RX_MGMT: u64 = 48;
pub const MT_MDP_RCFR0_MCU_RX_CTL_NON_BAR: u64 = 192;
pub const MT_MDP_RCFR0_MCU_RX_CTL_BAR: u64 = 768;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MDP_BNRCFR1(band: u64) -> u64 { MT_MDP((116u64).wrapping_add(shl(band, 8u64))) }

pub const MT_MDP_RCFR1_MCU_RX_BYPASS: u64 = 0xc0_0000;
pub const MT_MDP_RCFR1_RX_DROPPED_UCAST: u64 = 0x1800_0000;
pub const MT_MDP_RCFR1_RX_DROPPED_MCAST: u64 = 0x6000_0000;
pub const MT_MDP_TO_HIF: u64 = 0;
pub const MT_MDP_TO_WM: u64 = 1;
pub const MT_WFDMA0_HOST_INT_ENA: u64 = 0xd_4204;
pub const HOST_TX_DONE_INT_ENA8: u64 = 0x1000;
pub const HOST_TX_DONE_INT_ENA9: u64 = 0x2000;
pub const HOST_TX_DONE_INT_ENA10: u64 = 0x4000;
pub const HOST_TX_DONE_INT_ENA11: u64 = 0x8000;
pub const HOST_TX_DONE_INT_ENA12: u64 = 0x1_0000;
pub const HOST_TX_DONE_INT_ENA13: u64 = 0x2_0000;
pub const HOST_TX_DONE_INT_ENA14: u64 = 0x4_0000;
pub const HOST_RX_DONE_INT_ENA4: u64 = 0x40_0000;
pub const HOST_RX_DONE_INT_ENA5: u64 = 0x80_0000;
pub const HOST_TX_DONE_INT_ENA16: u64 = 0x400_0000;
pub const HOST_TX_DONE_INT_ENA17: u64 = 0x800_0000;
pub const MT_INT_RX_DONE_DATA: u64 = 4;
pub const MT_INT_RX_DONE_WM: u64 = 1;
pub const MT_INT_RX_DONE_WM2: u64 = 0x40_0000;
pub const MT_INT_RX_DONE_ALL: u64 = 0x40_0005;
pub const MT_INT_TX_DONE_MCU_WM: u64 = 0x800_0000;
pub const MT_INT_TX_DONE_FWDL: u64 = 0x400_0000;
pub const MT_INT_TX_DONE_BAND0: u64 = 16;
pub const MT_INT_TX_DONE_MCU: u64 = 0xc00_0000;
pub const MT_INT_TX_DONE_ALL: u64 = 0x807_fff0;
pub const MT_RX_DATA_RING_BASE: u64 = 0xd_4520;
pub const MT_INFRA_CFG_BASE: u64 = 0xf_e000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_INFRA(ofs: u64) -> u64 { (MT_INFRA_CFG_BASE).wrapping_add(ofs) }

pub const MT_HIF_REMAP_L1: u64 = 0xf_e24c;
pub const MT_HIF_REMAP_L1_MASK: u64 = 0xffff;
pub const MT_HIF_REMAP_L1_OFFSET: u64 = 0xffff;
pub const MT_HIF_REMAP_L1_BASE: u64 = 0xffff_0000;
pub const MT_HIF_REMAP_BASE_L1: u64 = 0x4_0000;
pub const MT_WFSYS_SW_RST_B: u64 = 0x1800_0140;
pub const MT_WTBLON_TOP_WDUCR: u64 = 0x820d_4200;
pub const MT_WTBLON_TOP_WDUCR_GROUP: u64 = 7;
pub const MT_WTBL_UPDATE: u64 = 0x820d_4230;
pub const MT_WTBL_UPDATE_WLAN_IDX: u64 = 0x3ff;
pub const MT_WTBL_UPDATE_ADM_COUNT_CLEAR: u64 = 0x1000;
pub const MT_PCIE_MAC_BASE: u64 = 0x1_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_PCIE_MAC(ofs: u64) -> u64 { (MT_PCIE_MAC_BASE).wrapping_add(ofs) }

pub const MT_PCIE_MAC_INT_ENABLE: u64 = 0x1_0188;
pub const MT_PCIE_MAC_PM: u64 = 0x1_0194;
