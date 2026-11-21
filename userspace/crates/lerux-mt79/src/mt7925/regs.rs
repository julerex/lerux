//! `mt7925/regs.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT_MDP_BASE: u64 = 0x820c_c800;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MDP(ofs: u64) -> u64 { (MT_MDP_BASE).wrapping_add(ofs) }

pub const MT_MDP_DCR0: u64 = 0x820c_c800;
pub const MT_MDP_DCR0_DAMSDU_EN: u64 = 0x8000;
pub const MT_MDP_DCR0_RX_HDR_TRANS_EN: u64 = 0x8_0000;
pub const MT_MDP_DCR1: u64 = 0x820c_c804;
pub const MT_MDP_DCR1_MAX_RX_LEN: u64 = 0xfff8;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MDP_BNRCFR0(band: u64) -> u64 { MT_MDP((144u64).wrapping_add(shl(band, 8u64))) }

pub const MT_MDP_RCFR0_MCU_RX_MGMT: u64 = 48;
pub const MT_MDP_RCFR0_MCU_RX_CTL_NON_BAR: u64 = 192;
pub const MT_MDP_RCFR0_MCU_RX_CTL_BAR: u64 = 768;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MDP_BNRCFR1(band: u64) -> u64 { MT_MDP((148u64).wrapping_add(shl(band, 8u64))) }

pub const MT_MDP_RCFR1_MCU_RX_BYPASS: u64 = 0xc0_0000;
pub const MT_MDP_RCFR1_RX_DROPPED_UCAST: u64 = 0x1800_0000;
pub const MT_MDP_RCFR1_RX_DROPPED_MCAST: u64 = 0x6000_0000;
pub const MT_MDP_TO_HIF: u64 = 0;
pub const MT_MDP_TO_WM: u64 = 1;
pub const HOST_RX_DONE_INT_ENA4: u64 = 0x1000;
pub const HOST_RX_DONE_INT_ENA5: u64 = 0x2000;
pub const HOST_RX_DONE_INT_ENA6: u64 = 0x4000;
pub const HOST_RX_DONE_INT_ENA7: u64 = 0x8000;
pub const HOST_RX_DONE_INT_ENA8: u64 = 0x1_0000;
pub const HOST_RX_DONE_INT_ENA9: u64 = 0x2_0000;
pub const HOST_RX_DONE_INT_ENA10: u64 = 0x4_0000;
pub const HOST_RX_DONE_INT_ENA11: u64 = 0x8_0000;
pub const HOST_TX_DONE_INT_ENA15: u64 = 0x200_0000;
pub const HOST_TX_DONE_INT_ENA16: u64 = 0x400_0000;
pub const HOST_TX_DONE_INT_ENA17: u64 = 0x800_0000;
pub const MT_INT_TX_DONE_MCU_WM: u64 = 0xa00_0000;
pub const MT_INT_TX_DONE_FWDL: u64 = 0x400_0000;
pub const MT_INT_TX_DONE_MCU: u64 = 0xe00_0000;
pub const MT_INFRA_CFG_BASE: u64 = 0xd_1000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_INFRA(ofs: u64) -> u64 { (MT_INFRA_CFG_BASE).wrapping_add(ofs) }

pub const MT_HIF_REMAP_L1: u64 = 0x15_5024;
pub const MT_HIF_REMAP_L1_MASK: u64 = 0xffff_0000;
pub const MT_HIF_REMAP_L1_OFFSET: u64 = 0xffff;
pub const MT_HIF_REMAP_L1_BASE: u64 = 0xffff_0000;
pub const MT_HIF_REMAP_BASE_L1: u64 = 0x13_0000;
pub const MT_HIF_REMAP_L2: u64 = 288;
pub const MT_HIF_REMAP_BASE_L2: u64 = 0x1850_0000;
pub const MT_WFSYS_SW_RST_B: u64 = 0x7c00_0140;
pub const MT_WTBLON_TOP_WDUCR_GROUP: u64 = 31;
pub const MT_WTBL_UPDATE_WLAN_IDX: u64 = 0xfff;
pub const MT_WTBL_UPDATE_ADM_COUNT_CLEAR: u64 = 0x4000;
pub const MT7925_PCIE_MAC_BASE: u64 = 0x1_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT7925_PCIE_MAC(ofs: u64) -> u64 { (MT7925_PCIE_MAC_BASE).wrapping_add(ofs) }

pub const MT7925_PCIE_MAC_INT_ENABLE: u64 = 0x1_0188;
pub const MT7925_PCIE_MAC_PM: u64 = 0x1_0194;
pub const MT7928_PCIE_MAC_BASE: u64 = 0x7404_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT7928_PCIE_MAC(ofs: u64) -> u64 { (MT7928_PCIE_MAC_BASE).wrapping_add(ofs) }

pub const MT7928_PCIE_MAC_INT_ENABLE: u64 = 0x7404_0188;
pub const MT7928_PCIE_MAC_PM: u64 = 0x7404_0194;
pub const MT_DMASHDL_LITE_BASE: u64 = 0x2002_6200;
#[must_use]
#[rustfmt::skip]
pub const fn MT_DMASHDL_LITE(ofs: u64) -> u64 { (MT_DMASHDL_LITE_BASE).wrapping_add(ofs) }

pub const MT_DMASHDL_LITE_MAIN_CONTROL: u64 = 0x2002_6204;
pub const MT_DMASHDL_LITE_MAIN_CONTROL_SW_RST: u64 = 0x4_0000;
pub const MT_DMASHDL_LITE_PAGE_SIZE: u64 = 0x2002_6208;
pub const MT_DMASHDL_LITE_PLE_PAGE_SIZE_MASK: u64 = 0x1fff;
pub const MT_DMASHDL_LITE_PSE_PAGE_SIZE_MASK: u64 = 0x1fff_0000;
pub const MT_DMASHDL_LITE_PKT_MAX_SIZE: u64 = 0x2002_620c;
pub const MT_DMASHDL_LITE_PLE_PKT_MAX_SIZE_MASK: u64 = 0x1fff;
pub const MT_DMASHDL_LITE_PSE_PKT_MAX_SIZE_MASK: u64 = 0x1fff_0000;
pub const MT_DMASHDL_LITE_GROUP_DISABLE0: u64 = 0x2002_6210;
pub const MT_DMASHDL_LITE_GROUP_DISABLE1: u64 = 0x2002_6214;
pub const MT_DMASHDL_LITE_GROUP_SN_CHK0: u64 = 0x2002_6218;
pub const MT_DMASHDL_LITE_GROUP_SN_CHK1: u64 = 0x2002_621c;
pub const MT_DMASHDL_LITE_GROUP_UDF_CHK0: u64 = 0x2002_6220;
pub const MT_DMASHDL_LITE_GROUP_UDF_CHK1: u64 = 0x2002_6224;
pub const MT_DMASHDL_LITE_Q_MAPPING0: u64 = 0x2002_6228;
pub const MT_DMASHDL_LITE_GROUP0_QUOTA: u64 = 0x2002_6300;
pub const MT_DMASHDL_LITE_GROUP_MIN_QUOTA_MASK: u64 = 0x1fff;
pub const MT_DMASHDL_LITE_GROUP_MAX_QUOTA_MASK: u64 = 0x1fff_0000;
