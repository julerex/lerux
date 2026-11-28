//! `mt792x_regs.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT_MCU_WFDMA1_BASE: u64 = 0x3000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MCU_WFDMA1(ofs: u64) -> u64 { (MT_MCU_WFDMA1_BASE).wrapping_add(ofs) }

pub const MT_MCU_INT_EVENT: u64 = 0x3108;
pub const MT_MCU_INT_EVENT_DMA_STOPPED: u64 = 1;
pub const MT_MCU_INT_EVENT_DMA_INIT: u64 = 2;
pub const MT_MCU_INT_EVENT_SER_TRIGGER: u64 = 4;
pub const MT_MCU_INT_EVENT_RESET_DONE: u64 = 8;
pub const MT_PLE_BASE: u64 = 0x820c_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_PLE(ofs: u64) -> u64 { (MT_PLE_BASE).wrapping_add(ofs) }

pub const MT_PLE_FL_Q0_CTRL: u64 = 0x820c_03e0;
pub const MT_PLE_FL_Q1_CTRL: u64 = 0x820c_03e4;
pub const MT_PLE_FL_Q2_CTRL: u64 = 0x820c_03e8;
pub const MT_PLE_FL_Q3_CTRL: u64 = 0x820c_03ec;
#[must_use]
#[rustfmt::skip]
pub const fn MT_PLE_AC_QEMPTY(n: u64) -> u64 { MT_PLE((0x500u64).wrapping_add((64u64).wrapping_mul(n))) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_PLE_AMSDU_PACK_MSDU_CNT(n: u64) -> u64 { MT_PLE((0x10e0u64).wrapping_add(shl(n, 2u64))) }

pub const MT_PSE_BASE: u64 = 0x820c_8000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_TMAC_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_4000u64 } else { 0x820e_4000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_TMAC(band: u64, ofs: u64) -> u64 { (MT_WF_TMAC_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_TMAC_TCR0(band: u64) -> u64 { MT_WF_TMAC(band, 0u64) }

pub const MT_TMAC_TCR0_TBTT_STOP_CTRL: u64 = 0x200_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TMAC_CDTR(band: u64) -> u64 { MT_WF_TMAC(band, 144u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_TMAC_ODTR(band: u64) -> u64 { MT_WF_TMAC(band, 148u64) }

pub const MT_TIMEOUT_VAL_PLCP: u64 = 0xffff;
pub const MT_TIMEOUT_VAL_CCA: u64 = 0xffff_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TMAC_ICR0(band: u64) -> u64 { MT_WF_TMAC(band, 164u64) }

pub const MT_IFS_EIFS: u64 = 511;
pub const MT_IFS_RIFS: u64 = 0x7c00;
pub const MT_IFS_SIFS: u64 = 0x7f_0000;
pub const MT_IFS_SLOT: u64 = 0x7f00_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TMAC_CTCR0(band: u64) -> u64 { MT_WF_TMAC(band, 244u64) }

pub const MT_TMAC_CTCR0_INS_DDLMT_REFTIME: u64 = 63;
pub const MT_TMAC_CTCR0_INS_DDLMT_EN: u64 = 0x2_0000;
pub const MT_TMAC_CTCR0_INS_DDLMT_VHT_SMPDU_EN: u64 = 0x4_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TMAC_TRCR0(band: u64) -> u64 { MT_WF_TMAC(band, 156u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_TMAC_TFCR0(band: u64) -> u64 { MT_WF_TMAC(band, 480u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_DMA_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_7000u64 } else { 0x820e_7000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_DMA(band: u64, ofs: u64) -> u64 { (MT_WF_DMA_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_DMA_DCR0(band: u64) -> u64 { MT_WF_DMA(band, 0u64) }

pub const MT_DMA_DCR0_MAX_RX_LEN: u64 = 0xfff8;
pub const MT_DMA_DCR0_RXD_G5_EN: u64 = 0x80_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WTBLOFF_TOP_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_9000u64 } else { 0x820e_9000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WTBLOFF_TOP(band: u64, ofs: u64) -> u64 { (MT_WTBLOFF_TOP_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WTBLOFF_TOP_RSCR(band: u64) -> u64 { MT_WTBLOFF_TOP(band, 8u64) }

pub const MT_WTBLOFF_TOP_RSCR_RCPI_MODE: u64 = 0xc000_0000;
pub const MT_WTBLOFF_TOP_RSCR_RCPI_PARAM: u64 = 0x300_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_LPON_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_b000u64 } else { 0x820e_b000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_LPON(band: u64, ofs: u64) -> u64 { (MT_WF_LPON_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_LPON_UTTR0(band: u64) -> u64 { MT_WF_LPON(band, 128u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_LPON_UTTR1(band: u64) -> u64 { MT_WF_LPON(band, 132u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_LPON_TCR(band: u64, n: u64) -> u64 { MT_WF_LPON(band, (168u64).wrapping_add((n).wrapping_mul(4u64))) }

pub const MT_LPON_TCR_SW_MODE: u64 = 3;
pub const MT_LPON_TCR_SW_WRITE: u64 = 1;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_ETBF_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_a000u64 } else { 0x820e_a000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_ETBF(band: u64, ofs: u64) -> u64 { (MT_WF_ETBF_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_ETBF_TX_APP_CNT(band: u64) -> u64 { MT_WF_ETBF(band, 336u64) }

pub const MT_ETBF_TX_IBF_CNT: u64 = 0xffff_0000;
pub const MT_ETBF_TX_EBF_CNT: u64 = 0xffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_ETBF_RX_FB_CNT(band: u64) -> u64 { MT_WF_ETBF(band, 344u64) }

pub const MT_ETBF_RX_FB_ALL: u64 = 0xff00_0000;
pub const MT_ETBF_RX_FB_HE: u64 = 0xff_0000;
pub const MT_ETBF_RX_FB_VHT: u64 = 0xff00;
pub const MT_ETBF_RX_FB_HT: u64 = 255;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_MIB_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_d000u64 } else { 0x820e_d000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_MIB(band: u64, ofs: u64) -> u64 { (MT_WF_MIB_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SCR1(band: u64) -> u64 { MT_WF_MIB(band, 4u64) }

pub const MT_MIB_TXDUR_EN: u64 = 256;
pub const MT_MIB_RXDUR_EN: u64 = 512;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR3(band: u64) -> u64 { MT_WF_MIB(band, 0x698u64) }

pub const MT_MIB_SDR3_FCS_ERR_MASK: u64 = 0xffff_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR5(band: u64) -> u64 { MT_WF_MIB(band, 0x780u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR9(band: u64) -> u64 { MT_WF_MIB(band, 44u64) }

pub const MT_MIB_SDR9_BUSY_MASK: u64 = 0xff_ffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR12(band: u64) -> u64 { MT_WF_MIB(band, 0x558u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR14(band: u64) -> u64 { MT_WF_MIB(band, 0x564u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR15(band: u64) -> u64 { MT_WF_MIB(band, 0x568u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR16(band: u64) -> u64 { MT_WF_MIB(band, 72u64) }

pub const MT_MIB_SDR16_BUSY_MASK: u64 = 0xff_ffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR22(band: u64) -> u64 { MT_WF_MIB(band, 0x770u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR23(band: u64) -> u64 { MT_WF_MIB(band, 0x774u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR31(band: u64) -> u64 { MT_WF_MIB(band, 0x55cu64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR32(band: u64) -> u64 { MT_WF_MIB(band, 0x7a8u64) }

pub const MT_MIB_SDR9_IBF_CNT_MASK: u64 = 0xffff_0000;
pub const MT_MIB_SDR9_EBF_CNT_MASK: u64 = 0xffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR34(band: u64) -> u64 { MT_WF_MIB(band, 144u64) }

pub const MT_MIB_MU_BF_TX_CNT: u64 = 0xffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR36(band: u64) -> u64 { MT_WF_MIB(band, 84u64) }

pub const MT_MIB_SDR36_TXTIME_MASK: u64 = 0xff_ffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR37(band: u64) -> u64 { MT_WF_MIB(band, 88u64) }

pub const MT_MIB_SDR37_RXTIME_MASK: u64 = 0xff_ffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_DR8(band: u64) -> u64 { MT_WF_MIB(band, 192u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_DR9(band: u64) -> u64 { MT_WF_MIB(band, 196u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_DR11(band: u64) -> u64 { MT_WF_MIB(band, 204u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_MB_SDR0(band: u64, n: u64) -> u64 { MT_WF_MIB(band, (256u64).wrapping_add(shl(n, 4u64))) }

pub const MT_MIB_RTS_RETRIES_COUNT_MASK: u64 = 0xffff_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_MB_BSDR0(band: u64) -> u64 { MT_WF_MIB(band, 0x688u64) }

pub const MT_MIB_RTS_COUNT_MASK: u64 = 0xffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_MB_BSDR1(band: u64) -> u64 { MT_WF_MIB(band, 0x690u64) }

pub const MT_MIB_RTS_FAIL_COUNT_MASK: u64 = 0xffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_MB_BSDR2(band: u64) -> u64 { MT_WF_MIB(band, 0x518u64) }

pub const MT_MIB_BA_FAIL_COUNT_MASK: u64 = 0xffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_MB_BSDR3(band: u64) -> u64 { MT_WF_MIB(band, 0x520u64) }

pub const MT_MIB_ACK_FAIL_COUNT_MASK: u64 = 0xffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_MB_SDR2(band: u64, n: u64) -> u64 { MT_WF_MIB(band, (264u64).wrapping_add(shl(n, 4u64))) }

pub const MT_MIB_FRAME_RETRIES_COUNT_MASK: u64 = 0xffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TX_AGG_CNT(band: u64, n: u64) -> u64 { MT_WF_MIB(band, (0x7dcu64).wrapping_add(shl(n, 2u64))) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_TX_AGG_CNT2(band: u64, n: u64) -> u64 { MT_WF_MIB(band, (0x7ecu64).wrapping_add(shl(n, 2u64))) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_ARNG(band: u64, n: u64) -> u64 { MT_WF_MIB(band, (176u64).wrapping_add(shl(n, 2u64))) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_ARNCR_RANGE(val: u64, n: u64) -> u64 { (shr(val, shl(n, 3u64))) & (genmask(7u64, 0u64)) }

pub const MT_WTBLON_TOP_BASE: u64 = 0x820d_4000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WTBLON_TOP(ofs: u64) -> u64 { (MT_WTBLON_TOP_BASE).wrapping_add(ofs) }

pub const MT_WTBL_UPDATE_BUSY: u64 = 0x8000_0000;
pub const MT_WTBL_ITCR: u64 = 0x820d_43b0;
pub const MT_WTBL_ITCR_WR: u64 = 0x1_0000;
pub const MT_WTBL_ITCR_EXEC: u64 = 0x8000_0000;
pub const MT_WTBL_ITDR0: u64 = 0x820d_43b8;
pub const MT_WTBL_ITDR1: u64 = 0x820d_43bc;
pub const MT_WTBL_SPE_IDX_SEL: u64 = 64;
pub const MT_WTBL_BASE: u64 = 0x820d_8000;
pub const MT_WTBL_LMAC_ID: u64 = 0x7f00;
pub const MT_WTBL_LMAC_DW: u64 = 252;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_AGG_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_2000u64 } else { 0x820e_2000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_AGG(band: u64, ofs: u64) -> u64 { (MT_WF_AGG_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_AGG_AWSCR0(band: u64, n: u64) -> u64 { MT_WF_AGG(band, (92u64).wrapping_add((n).wrapping_mul(4u64))) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_AGG_PCR0(band: u64, n: u64) -> u64 { MT_WF_AGG(band, (108u64).wrapping_add((n).wrapping_mul(4u64))) }

pub const MT_AGG_PCR0_MM_PROT: u64 = 1;
pub const MT_AGG_PCR0_GF_PROT: u64 = 2;
pub const MT_AGG_PCR0_BW20_PROT: u64 = 4;
pub const MT_AGG_PCR0_BW40_PROT: u64 = 16;
pub const MT_AGG_PCR0_BW80_PROT: u64 = 64;
pub const MT_AGG_PCR0_ERP_PROT: u64 = 0x1f00;
pub const MT_AGG_PCR0_VHT_PROT: u64 = 0x2000;
pub const MT_AGG_PCR0_PTA_WIN_DIS: u64 = 0x8000;
pub const MT_AGG_PCR1_RTS0_NUM_THRES: u64 = 0xff80_0000;
pub const MT_AGG_PCR1_RTS0_LEN_THRES: u64 = 0xf_ffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_AGG_ACR0(band: u64) -> u64 { MT_WF_AGG(band, 132u64) }

pub const MT_AGG_ACR_CFEND_RATE: u64 = 0x3fff;
pub const MT_AGG_ACR_BAR_RATE: u64 = 0x3fff_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_AGG_MRCR(band: u64) -> u64 { MT_WF_AGG(band, 152u64) }

pub const MT_AGG_MRCR_BAR_CNT_LIMIT: u64 = 0xf000;
pub const MT_AGG_MRCR_LAST_RTS_CTS_RN: u64 = 64;
pub const MT_AGG_MRCR_RTS_FAIL_LIMIT: u64 = 0xf80;
pub const MT_AGG_MRCR_TXCMD_RTS_FAIL_LIMIT: u64 = 0x1f00_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_AGG_ATCR1(band: u64) -> u64 { MT_WF_AGG(band, 240u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_AGG_ATCR3(band: u64) -> u64 { MT_WF_AGG(band, 244u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_ARB_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_3000u64 } else { 0x820e_3000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_ARB(band: u64, ofs: u64) -> u64 { (MT_WF_ARB_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_ARB_SCR(band: u64) -> u64 { MT_WF_ARB(band, 128u64) }

pub const MT_ARB_SCR_TX_DISABLE: u64 = 256;
pub const MT_ARB_SCR_RX_DISABLE: u64 = 512;
#[must_use]
#[rustfmt::skip]
pub const fn MT_ARB_DRNGR0(band: u64, n: u64) -> u64 { MT_WF_ARB(band, (404u64).wrapping_add((n).wrapping_mul(4u64))) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_RMAC_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_5000u64 } else { 0x820e_5000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_RMAC(band: u64, ofs: u64) -> u64 { (MT_WF_RMAC_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_RFCR(band: u64) -> u64 { MT_WF_RMAC(band, 0u64) }

pub const MT_WF_RFCR_DROP_STBC_MULTI: u64 = 1;
pub const MT_WF_RFCR_DROP_FCSFAIL: u64 = 2;
pub const MT_WF_RFCR_DROP_VERSION: u64 = 8;
pub const MT_WF_RFCR_DROP_PROBEREQ: u64 = 16;
pub const MT_WF_RFCR_DROP_MCAST: u64 = 32;
pub const MT_WF_RFCR_DROP_BCAST: u64 = 64;
pub const MT_WF_RFCR_DROP_MCAST_FILTERED: u64 = 128;
pub const MT_WF_RFCR_DROP_A3_MAC: u64 = 256;
pub const MT_WF_RFCR_DROP_A3_BSSID: u64 = 512;
pub const MT_WF_RFCR_DROP_A2_BSSID: u64 = 0x400;
pub const MT_WF_RFCR_DROP_OTHER_BEACON: u64 = 0x800;
pub const MT_WF_RFCR_DROP_FRAME_REPORT: u64 = 0x1000;
pub const MT_WF_RFCR_DROP_CTL_RSV: u64 = 0x2000;
pub const MT_WF_RFCR_DROP_CTS: u64 = 0x4000;
pub const MT_WF_RFCR_DROP_RTS: u64 = 0x8000;
pub const MT_WF_RFCR_DROP_DUPLICATE: u64 = 0x1_0000;
pub const MT_WF_RFCR_DROP_OTHER_BSS: u64 = 0x2_0000;
pub const MT_WF_RFCR_DROP_OTHER_UC: u64 = 0x4_0000;
pub const MT_WF_RFCR_DROP_OTHER_TIM: u64 = 0x8_0000;
pub const MT_WF_RFCR_DROP_NDPA: u64 = 0x10_0000;
pub const MT_WF_RFCR_DROP_UNWANTED_CTL: u64 = 0x20_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_RFCR1(band: u64) -> u64 { MT_WF_RMAC(band, 4u64) }

pub const MT_WF_RFCR1_DROP_ACK: u64 = 16;
pub const MT_WF_RFCR1_DROP_BF_POLL: u64 = 32;
pub const MT_WF_RFCR1_DROP_BA: u64 = 64;
pub const MT_WF_RFCR1_DROP_CFEND: u64 = 128;
pub const MT_WF_RFCR1_DROP_CFACK: u64 = 256;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_RMAC_MIB_TIME0(band: u64) -> u64 { MT_WF_RMAC(band, 964u64) }

pub const MT_WF_RMAC_MIB_RXTIME_CLR: u64 = 0x8000_0000;
pub const MT_WF_RMAC_MIB_RXTIME_EN: u64 = 0x4000_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_RMAC_MIB_AIRTIME14(band: u64) -> u64 { MT_WF_RMAC(band, 952u64) }

pub const MT_MIB_OBSSTIME_MASK: u64 = 0xff_ffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_RMAC_MIB_AIRTIME0(band: u64) -> u64 { MT_WF_RMAC(band, 896u64) }

pub const MT_WFDMA0_BASE: u64 = 0xd_4000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WFDMA0(ofs: u64) -> u64 { (MT_WFDMA0_BASE).wrapping_add(ofs) }

pub const MT_WFDMA0_RST: u64 = 0xd_4100;
pub const MT_WFDMA0_RST_LOGIC_RST: u64 = 16;
pub const MT_WFDMA0_RST_DMASHDL_ALL_RST: u64 = 32;
pub const MT_WFDMA0_BUSY_ENA: u64 = 0xd_413c;
pub const MT_WFDMA0_BUSY_ENA_TX_FIFO0: u64 = 1;
pub const MT_WFDMA0_BUSY_ENA_TX_FIFO1: u64 = 2;
pub const MT_WFDMA0_BUSY_ENA_RX_FIFO: u64 = 4;
pub const MT_MCU_CMD: u64 = 0xd_41f0;
pub const MT_MCU_CMD_WAKE_RX_PCIE: u64 = 1;
pub const MT_MCU_CMD_STOP_DMA_FW_RELOAD: u64 = 2;
pub const MT_MCU_CMD_STOP_DMA: u64 = 4;
pub const MT_MCU_CMD_RESET_DONE: u64 = 8;
pub const MT_MCU_CMD_RECOVERY_DONE: u64 = 16;
pub const MT_MCU_CMD_NORMAL_STATE: u64 = 32;
pub const MT_MCU_CMD_ERROR_MASK: u64 = 62;
pub const MT_MCU2HOST_SW_INT_ENA: u64 = 0xd_41f4;
pub const MT_WFDMA0_HOST_INT_STA: u64 = 0xd_4200;
pub const HOST_RX_DONE_INT_STS0: u64 = 1;
pub const HOST_RX_DONE_INT_STS2: u64 = 4;
pub const HOST_RX_DONE_INT_STS4: u64 = 0x40_0000;
pub const HOST_TX_DONE_INT_STS16: u64 = 0x400_0000;
pub const HOST_TX_DONE_INT_STS17: u64 = 0x800_0000;
pub const MT_WFDMA0_GLO_CFG: u64 = 0xd_4208;
pub const MT_WFDMA0_GLO_CFG_TX_DMA_EN: u64 = 1;
pub const MT_WFDMA0_GLO_CFG_TX_DMA_BUSY: u64 = 2;
pub const MT_WFDMA0_GLO_CFG_RX_DMA_EN: u64 = 4;
pub const MT_WFDMA0_GLO_CFG_RX_DMA_BUSY: u64 = 8;
pub const MT_WFDMA0_GLO_CFG_DMA_SIZE: u64 = 48;
pub const MT_WFDMA0_GLO_CFG_TX_WB_DDONE: u64 = 64;
pub const MT_WFDMA0_GLO_CFG_FW_DWLD_BYPASS_DMASHDL: u64 = 512;
pub const MT_WFDMA0_GLO_CFG_FIFO_DIS_CHECK: u64 = 0x800;
pub const MT_WFDMA0_GLO_CFG_FIFO_LITTLE_ENDIAN: u64 = 0x1000;
pub const MT_WFDMA0_GLO_CFG_RX_WB_DDONE: u64 = 0x2000;
pub const MT_WFDMA0_GLO_CFG_CSR_DISP_BASE_PTR_CHAIN_EN: u64 = 0x8000;
pub const MT_WFDMA0_GLO_CFG_CSR_LBK_RX_Q_SEL_EN: u64 = 0x10_0000;
pub const MT_WFDMA0_GLO_CFG_OMIT_RX_INFO_PFET2: u64 = 0x20_0000;
pub const MT_WFDMA0_GLO_CFG_ADDR_EXT_EN: u64 = 0x400_0000;
pub const MT_WFDMA0_GLO_CFG_OMIT_RX_INFO: u64 = 0x800_0000;
pub const MT_WFDMA0_GLO_CFG_OMIT_TX_INFO: u64 = 0x1000_0000;
pub const MT_WFDMA0_GLO_CFG_CLK_GAT_DIS: u64 = 0x4000_0000;
pub const HOST_RX_DONE_INT_ENA0: u64 = 1;
pub const HOST_RX_DONE_INT_ENA1: u64 = 2;
pub const HOST_RX_DONE_INT_ENA2: u64 = 4;
pub const HOST_RX_DONE_INT_ENA3: u64 = 8;
pub const MT7927_RX_DONE_INT_ENA4: u64 = 0x1000;
pub const MT7927_RX_DONE_INT_ENA6: u64 = 0x4000;
pub const MT7927_RX_DONE_INT_ENA7: u64 = 0x8000;
pub const HOST_TX_DONE_INT_ENA0: u64 = 16;
pub const HOST_TX_DONE_INT_ENA1: u64 = 32;
pub const HOST_TX_DONE_INT_ENA2: u64 = 64;
pub const HOST_TX_DONE_INT_ENA3: u64 = 128;
pub const HOST_TX_DONE_INT_ENA4: u64 = 256;
pub const HOST_TX_DONE_INT_ENA5: u64 = 512;
pub const HOST_TX_DONE_INT_ENA6: u64 = 0x400;
pub const HOST_TX_DONE_INT_ENA7: u64 = 0x800;
pub const HOST_RX_COHERENT_EN: u64 = 0x10_0000;
pub const HOST_TX_COHERENT_EN: u64 = 0x20_0000;
pub const MCU2HOST_SW_INT_ENA: u64 = 0x2000_0000;
pub const HOST_TX_DONE_INT_ENA18: u64 = 0x4000_0000;
pub const MT_INT_MCU_CMD: u64 = 0x2000_0000;
pub const MT_WFDMA0_RST_DTX_PTR: u64 = 0xd_420c;
pub const MT_WFDMA0_WPDMA_PAUSE_RXQ_TH10: u64 = 0xd_4260;
pub const MT_WFDMA0_WPDMA_PAUSE_RXQ_TH32: u64 = 0xd_4264;
pub const MT_WFDMA0_WPDMA_PAUSE_RXQ_TH54: u64 = 0xd_4268;
pub const MT_WFDMA0_WPDMA_PAUSE_RXQ_TH76: u64 = 0xd_426c;
pub const MT_WFDMA0_WPDMA_PAUSE_RXQ_THXX_L_TH_MASK: u64 = 0xfff;
pub const MT_WFDMA0_WPDMA_PAUSE_RXQ_THXX_H_TH_MASK: u64 = 0xfff_0000;
pub const MT_WFDMA0_RST_DRX_PTR: u64 = 0xd_4280;
pub const MT_WFDMA0_INT_RX_PRI: u64 = 0xd_4298;
pub const MT_WFDMA0_INT_TX_PRI: u64 = 0xd_429c;
pub const MT_WFDMA0_GLO_CFG_EXT0: u64 = 0xd_42b0;
pub const MT_WFDMA0_GLO_CFG_EXT0_CSR_MEM_ARB_LOCK_EN: u64 = 16;
pub const MT_WFDMA0_GLO_CFG_EXT0_CSR_TX_DMASHDL_EN: u64 = 64;
pub const MT_WFDMA0_GLO_CFG_EXT0_CSR_TX_DMASHDL_LITE_EN: u64 = 128;
pub const MT_WFDMA0_GLO_CFG_EXT0_CSR_RX_WB_KEEP_RSVD: u64 = 0x400;
pub const MT_WFDMA0_GLO_CFG_EXT0_CSR_BID_CHECK_BYPASS_EN: u64 = 0x40_0000;
pub const MT_WFDMA0_GLO_CFG_EXT0_CSR_RX_INFO_WB_EN: u64 = 0x80_0000;
pub const MT_WFDMA0_GLO_CFG_EXT0_CSR_AXI_AWUSER_LOCK_EN: u64 = 0x2000_0000;
pub const MT_WFDMA0_GLO_CFG_EXT0_CSR_MAX_PREFETCH_CNT_MASK: u64 = 3;
pub const MT_WFDMA0_GLO_CFG_EXT0_CSR_MEM_BST_SIZE_MASK: u64 = 12;
pub const MT_WFDMA0_GLO_CFG_EXT0_CSR_AXI_AW_OUTSTANDING_NUM_MASK: u64 = 0xf00_0000;
pub const MT_WFDMA0_GLO_CFG_EXT1: u64 = 0xd_42b4;
pub const MT_WFDMA0_PRI_DLY_INT_CFG0: u64 = 0xd_42f0;
pub const MT_WFDMA0_TX_RING0_EXT_CTRL: u64 = 0xd_4600;
pub const MT_WFDMA0_TX_RING1_EXT_CTRL: u64 = 0xd_4604;
pub const MT_WFDMA0_TX_RING2_EXT_CTRL: u64 = 0xd_4608;
pub const MT_WFDMA0_TX_RING3_EXT_CTRL: u64 = 0xd_460c;
pub const MT_WFDMA0_TX_RING4_EXT_CTRL: u64 = 0xd_4610;
pub const MT_WFDMA0_TX_RING5_EXT_CTRL: u64 = 0xd_4614;
pub const MT_WFDMA0_TX_RING6_EXT_CTRL: u64 = 0xd_4618;
pub const MT_WFDMA0_TX_RING15_EXT_CTRL: u64 = 0xd_463c;
pub const MT_WFDMA0_TX_RING16_EXT_CTRL: u64 = 0xd_4640;
pub const MT_WFDMA0_TX_RING17_EXT_CTRL: u64 = 0xd_4644;
pub const MT_WPDMA0_MAX_CNT_MASK: u64 = 255;
pub const MT_WPDMA0_BASE_PTR_MASK: u64 = 0xffff_0000;
pub const MT_WFDMA0_RX_RING0_EXT_CTRL: u64 = 0xd_4680;
pub const MT_WFDMA0_RX_RING1_EXT_CTRL: u64 = 0xd_4684;
pub const MT_WFDMA0_RX_RING2_EXT_CTRL: u64 = 0xd_4688;
pub const MT_WFDMA0_RX_RING3_EXT_CTRL: u64 = 0xd_468c;
pub const MT_WFDMA0_RX_RING4_EXT_CTRL: u64 = 0xd_4690;
pub const MT_WFDMA0_RX_RING5_EXT_CTRL: u64 = 0xd_4694;
pub const MT_WFDMA0_RX_RING6_EXT_CTRL: u64 = 0xd_4698;
pub const MT_WFDMA0_RX_RING7_EXT_CTRL: u64 = 0xd_469c;
pub const MT_TX_RING_BASE: u64 = 0xd_4300;
pub const MT_RX_EVENT_RING_BASE: u64 = 0xd_4500;
pub const MT_WFDMA_EXT_CSR_BASE: u64 = 0xd_7000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WFDMA_EXT_CSR(ofs: u64) -> u64 { (MT_WFDMA_EXT_CSR_BASE).wrapping_add(ofs) }

pub const MT_WFDMA_EXT_CSR_HIF_MISC: u64 = 0xd_7044;
pub const MT_WFDMA_EXT_CSR_HIF_MISC_BUSY: u64 = 1;
pub const MT_WFDMA_PREFETCH_CTRL: u64 = 0xd_7030;
pub const MT_WFDMA_PREFETCH_CFG0: u64 = 0xd_70f0;
pub const MT_WFDMA_PREFETCH_CFG1: u64 = 0xd_70f4;
pub const MT_WFDMA_PREFETCH_CFG2: u64 = 0xd_70f8;
pub const MT_WFDMA_PREFETCH_CFG3: u64 = 0xd_70fc;
pub const MT_SWDEF_BASE: u64 = 0x41_f200;
#[must_use]
#[rustfmt::skip]
pub const fn MT_SWDEF(ofs: u64) -> u64 { (MT_SWDEF_BASE).wrapping_add(ofs) }

pub const MT_SWDEF_MODE: u64 = 0x41_f23c;
pub const MT_SWDEF_NORMAL_MODE: u64 = 0;
pub const MT_SWDEF_ICAP_MODE: u64 = 1;
pub const MT_SWDEF_SPECTRUM_MODE: u64 = 2;
pub const MT_TOP_BASE: u64 = 0x1806_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP(ofs: u64) -> u64 { (MT_TOP_BASE).wrapping_add(ofs) }

pub const MT_TOP_LPCR_HOST_BAND0: u64 = 0x1806_0010;
pub const MT_TOP_LPCR_HOST_FW_OWN: u64 = 1;
pub const MT_TOP_LPCR_HOST_DRV_OWN: u64 = 2;
pub const MT_TOP_MISC: u64 = 0x1806_00f0;
pub const MT_TOP_MISC_FW_STATE: u64 = 7;
pub const MT_MCU_WPDMA0_BASE: u64 = 0x5400_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MCU_WPDMA0(ofs: u64) -> u64 { (MT_MCU_WPDMA0_BASE).wrapping_add(ofs) }

pub const MT_WFDMA_DUMMY_CR: u64 = 0x5400_0120;
pub const MT_WFDMA_NEED_REINIT: u64 = 2;
#[must_use]
#[rustfmt::skip]
pub const fn MT_CBTOP_RGU(ofs: u64) -> u64 { (0x7000_2000u64).wrapping_add(ofs) }

pub const MT_CBTOP_RGU_WF_SUBSYS_RST: u64 = 0x7000_2600;
pub const MT_CBTOP_RGU_WF_SUBSYS_RST_WF_WHOLE_PATH: u64 = 1;
pub const MT7925_CBTOP_RGU_WF_SUBSYS_RST: u64 = 0x7002_8600;
pub const MT7925_WFSYS_INIT_DONE_ADDR: u64 = 0x184c_1604;
pub const MT7925_WFSYS_INIT_DONE: u64 = 0x1d1e;
pub const MT_HW_BOUND: u64 = 0x7001_0020;
pub const MT_HW_CHIPID: u64 = 0x7001_0200;
pub const MT_HW_REV: u64 = 0x7001_0204;
pub const MT_HW_EMI_CTL: u64 = 0x1801_1100;
pub const MT_HW_EMI_CTL_SLPPROT_EN: u64 = 2;
pub const MT_PCIE_MAC_PM_L0S_DIS: u64 = 256;
#[must_use]
#[rustfmt::skip]
pub const fn MT_DMA_SHDL(ofs: u64) -> u64 { (0x7c02_6000u64).wrapping_add(ofs) }

pub const MT_DMASHDL_SW_CONTROL: u64 = 0x7c02_6004;
pub const MT_DMASHDL_DMASHDL_BYPASS: u64 = 0x1000_0000;
pub const MT_DMASHDL_OPTIONAL: u64 = 0x7c02_6008;
pub const MT_DMASHDL_PAGE: u64 = 0x7c02_600c;
pub const MT_DMASHDL_GROUP_SEQ_ORDER: u64 = 0x1_0000;
pub const MT_DMASHDL_REFILL: u64 = 0x7c02_6010;
pub const MT_DMASHDL_REFILL_MASK: u64 = 0xffff_0000;
pub const MT_DMASHDL_PKT_MAX_SIZE: u64 = 0x7c02_601c;
pub const MT_DMASHDL_PKT_MAX_SIZE_PLE: u64 = 0xfff;
pub const MT_DMASHDL_PKT_MAX_SIZE_PSE: u64 = 0xfff_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_DMASHDL_GROUP_QUOTA(n: u64) -> u64 { MT_DMA_SHDL((32u64).wrapping_add(shl(n, 2u64))) }

pub const MT_DMASHDL_GROUP_QUOTA_MIN: u64 = 0xfff;
pub const MT_DMASHDL_GROUP_QUOTA_MAX: u64 = 0xfff_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_DMASHDL_Q_MAP(n: u64) -> u64 { MT_DMA_SHDL((96u64).wrapping_add(shl(n, 2u64))) }

pub const MT_DMASHDL_Q_MAP_MASK: u64 = 15;
#[must_use]
#[rustfmt::skip]
pub const fn MT_DMASHDL_Q_MAP_SHIFT(n: u64) -> u64 { (4u64).wrapping_mul(rem(n, 8u64)) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_DMASHDL_SCHED_SET(n: u64) -> u64 { MT_DMA_SHDL((112u64).wrapping_add(shl(n, 2u64))) }

pub const MT_WFDMA_HOST_CONFIG: u64 = 0x7c02_7030;
pub const MT_WFDMA_HOST_CONFIG_USB_RXEVT_EP4_EN: u64 = 64;
#[must_use]
#[rustfmt::skip]
pub const fn MT_UMAC(ofs: u64) -> u64 { (0x7400_0000u64).wrapping_add(ofs) }

pub const MT_UDMA_TX_QSEL: u64 = 0x7400_0008;
pub const MT_FW_DL_EN: u64 = 8;
pub const MT_UDMA_WLCFG_1: u64 = 0x7400_000c;
pub const MT_WL_RX_AGG_PKT_LMT: u64 = 255;
pub const MT_WL_TX_TMOUT_LMT: u64 = 0xfff_ff00;
pub const MT_UDMA_WLCFG_0: u64 = 0x7400_0018;
pub const MT_WL_RX_AGG_TO: u64 = 255;
pub const MT_WL_RX_AGG_LMT: u64 = 0xff00;
pub const MT_WL_TX_TMOUT_FUNC_EN: u64 = 0x1_0000;
pub const MT_WL_TX_DPH_CHK_EN: u64 = 0x2_0000;
pub const MT_WL_RX_MPSZ_PAD0: u64 = 0x4_0000;
pub const MT_WL_RX_FLUSH: u64 = 0x8_0000;
pub const MT_TICK_1US_EN: u64 = 0x10_0000;
pub const MT_WL_RX_AGG_EN: u64 = 0x20_0000;
pub const MT_WL_RX_EN: u64 = 0x40_0000;
pub const MT_WL_TX_EN: u64 = 0x80_0000;
pub const MT_WL_RX_BUSY: u64 = 0x4000_0000;
pub const MT_WL_TX_BUSY: u64 = 0x8000_0000;
pub const MT_UDMA_CONN_INFRA_STATUS: u64 = 0x7400_0a20;
pub const MT_UDMA_CONN_WFSYS_INIT_DONE: u64 = 0x40_0000;
pub const MT_UDMA_CONN_INFRA_STATUS_SEL: u64 = 0x7400_0a24;
#[must_use]
#[rustfmt::skip]
pub const fn MT_SSUSB_EPCTL_CSR(ofs: u64) -> u64 { (0x7401_1800u64).wrapping_add(ofs) }

pub const MT_SSUSB_EPCTL_CSR_EP_RST_OPT: u64 = 0x7401_1890;
#[must_use]
#[rustfmt::skip]
pub const fn MT_UWFDMA0(ofs: u64) -> u64 { (0x7c02_4000u64).wrapping_add(ofs) }

pub const MT_UWFDMA0_GLO_CFG: u64 = 0x7c02_4208;
pub const MT_UWFDMA0_GLO_CFG_EXT0: u64 = 0x7c02_42b0;
pub const MT_UWFDMA0_GLO_CFG_EXT1: u64 = 0x7c02_42b4;
#[must_use]
#[rustfmt::skip]
pub const fn MT_UWFDMA0_TX_RING_EXT_CTRL(n: u64) -> u64 { MT_UWFDMA0((0x600u64).wrapping_add(shl(n, 2u64))) }

pub const MT_CONN_STATUS: u64 = 0x7c05_3c10;
pub const MT_WIFI_PATCH_DL_STATE: u64 = 1;
pub const MT_CONN_ON_LPCTL: u64 = 0x7c06_0010;
pub const PCIE_LPCR_HOST_SET_OWN: u64 = 1;
pub const PCIE_LPCR_HOST_CLR_OWN: u64 = 2;
pub const PCIE_LPCR_HOST_OWN_SYNC: u64 = 4;
pub const MT_CONN_ON_MISC: u64 = 0x7c06_00f0;
pub const MT_TOP_MISC2_FW_PWR_ON: u64 = 1;
pub const MT_TOP_MISC2_FW_N9_ON: u64 = 2;
pub const MT_TOP_MISC2_FW_N9_RDY: u64 = 3;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_SW_DEF_CR(ofs: u64) -> u64 { (0x40_1a00u64).wrapping_add(ofs) }

pub const MT_WF_SW_DEF_CR_USB_MCU_EVENT: u64 = 0x40_1a28;
pub const MT_WF_SW_SER_TRIGGER_SUSPEND: u64 = 64;
pub const MT_WF_SW_SER_DONE_SUSPEND: u64 = 128;
pub const WFSYS_SW_RST_B: u64 = 1;
pub const WFSYS_SW_INIT_DONE: u64 = 16;
pub const MT7927_CBINFRA_RGU_WF_RST: u64 = 0x1f_8600;
pub const MT7927_CBINFRA_RGU_WF_RST_WF_SUBSYS: u64 = 16;
pub const MT7927_CBINFRA_MCU_OWN_SET: u64 = 0x1f_5034;
pub const MT7927_ROMCODE_INDEX: u64 = 0xc_1604;
pub const MT7927_MCU_IDLE_VALUE: u64 = 0x1d1e;
pub const MT7927_PCIE2AP_REMAP_WF_0_54: u64 = 0x2_1008;
pub const MT7927_PCIE2AP_REMAP_WF_0_54_MASK: u64 = 0xffff;
pub const MT7927_PCIE2AP_REMAP_WF_0_54_VAL: u64 = 0x1807;
pub const MT7927_SEMA_OWN_STA: u64 = 0x4_0000;
pub const MT7927_SEMA_OWN_STA_REP: u64 = 0x4_0400;
