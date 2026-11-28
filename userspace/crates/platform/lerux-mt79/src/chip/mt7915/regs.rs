//! `mt7915/regs.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT_MCU_WFDMA0_BASE: u64 = 0x2000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MCU_WFDMA0(ofs: u64) -> u64 { (MT_MCU_WFDMA0_BASE).wrapping_add(ofs) }

pub const MT_MCU_WFDMA0_DUMMY_CR: u64 = 0x2120;
pub const MT_MCU_WFDMA1_BASE: u64 = 0x3000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MCU_WFDMA1(ofs: u64) -> u64 { (MT_MCU_WFDMA1_BASE).wrapping_add(ofs) }

pub const MT_MCU_INT_EVENT_DMA_STOPPED: u64 = 1;
pub const MT_MCU_INT_EVENT_DMA_INIT: u64 = 2;
pub const MT_MCU_INT_EVENT_SER_TRIGGER: u64 = 4;
pub const MT_MCU_INT_EVENT_RESET_DONE: u64 = 8;
pub const MT_PLE_BASE: u64 = 0x820c_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_PLE(ofs: u64) -> u64 { (MT_PLE_BASE).wrapping_add(ofs) }

pub const MT_PLE_HOST_RPT0: u64 = 0x820c_0030;
pub const MT_PLE_HOST_RPT0_TX_LATENCY: u64 = 8;
#[must_use]
#[rustfmt::skip]
pub const fn MT_PLE_AMSDU_PACK_MSDU_CNT(n: u64) -> u64 { MT_PLE((0x10e0u64).wrapping_add(shl(n, 2u64))) }

pub const MT_PSE_BASE: u64 = 0x820c_8000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_PSE(ofs: u64) -> u64 { (MT_PSE_BASE).wrapping_add(ofs) }

pub const MT_MDP_BASE: u64 = 0x820c_d000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MDP(ofs: u64) -> u64 { (MT_MDP_BASE).wrapping_add(ofs) }

pub const MT_MDP_DCR0: u64 = 0x820c_d000;
pub const MT_MDP_DCR0_DAMSDU_EN: u64 = 0x8000;
pub const MT_MDP_DCR0_RX_HDR_TRANS_EN: u64 = 0x8_0000;
pub const MT_MDP_DCR1: u64 = 0x820c_d004;
pub const MT_MDP_DCR1_MAX_RX_LEN: u64 = 0xfff8;
pub const MT_MDP_DCR2: u64 = 0x820c_d0e8;
pub const MT_MDP_DCR2_RX_TRANS_SHORT: u64 = 4;
pub const MT_MDP_RCFR0_MCU_RX_MGMT: u64 = 48;
pub const MT_MDP_RCFR0_MCU_RX_CTL_NON_BAR: u64 = 192;
pub const MT_MDP_RCFR0_MCU_RX_CTL_BAR: u64 = 768;
pub const MT_MDP_RCFR1_MCU_RX_BYPASS: u64 = 0xc0_0000;
pub const MT_MDP_RCFR1_RX_DROPPED_UCAST: u64 = 0x1800_0000;
pub const MT_MDP_RCFR1_RX_DROPPED_MCAST: u64 = 0x6000_0000;
pub const MT_MDP_TO_HIF: u64 = 0;
pub const MT_MDP_TO_WM: u64 = 1;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_TRB_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_1000u64 } else { 0x820e_1000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_TRB(band: u64, ofs: u64) -> u64 { (MT_WF_TRB_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_TRB_RXPSR0(band: u64) -> u64 { MT_WF_TRB(band, 60u64) }

pub const MT_TRB_RXPSR0_RX_WTBL_PTR: u64 = 0x3ff_0000;
pub const MT_TRB_RXPSR0_RX_RMAC_PTR: u64 = 0x3ff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_TMAC_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_4000u64 } else { 0x820e_4000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_TMAC(band: u64, ofs: u64) -> u64 { (MT_WF_TMAC_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_TMAC_TCR0(band: u64) -> u64 { MT_WF_TMAC(band, 0u64) }

pub const MT_TMAC_TCR0_TX_BLINK: u64 = 192;
pub const MT_TMAC_TCR0_TBTT_STOP_CTRL: u64 = 0x200_0000;
pub const MT_TIMEOUT_VAL_PLCP: u64 = 0xffff;
pub const MT_TIMEOUT_VAL_CCA: u64 = 0xffff_0000;
pub const MT_TMAC_ATCR_TXV_TOUT: u64 = 255;
pub const MT_TMAC_TRCR0_TR2T_CHK: u64 = 511;
pub const MT_TMAC_TRCR0_I2T_CHK: u64 = 0x1ff_0000;
pub const MT_IFS_EIFS_OFDM: u64 = 511;
pub const MT_IFS_RIFS: u64 = 0x7c00;
pub const MT_IFS_SIFS: u64 = 0x7f_0000;
pub const MT_IFS_SLOT: u64 = 0x7f00_0000;
pub const MT_IFS_EIFS_CCK: u64 = 511;
pub const MT_TMAC_CTCR0_INS_DDLMT_REFTIME: u64 = 63;
pub const MT_TMAC_CTCR0_INS_DDLMT_EN: u64 = 0x2_0000;
pub const MT_TMAC_CTCR0_INS_DDLMT_VHT_SMPDU_EN: u64 = 0x4_0000;
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
pub const fn MT_WTBLOFF_TOP_ACR(band: u64) -> u64 { MT_WTBLOFF_TOP(band, 16u64) }

pub const MT_WTBLOFF_TOP_ADM_BACKOFFTIME: u64 = 0x2000_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_ETBF_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_a000u64 } else { 0x820e_a000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_ETBF(band: u64, ofs: u64) -> u64 { (MT_WF_ETBF_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_ETBF_TX_NDP_BFRP(band: u64) -> u64 { MT_WF_ETBF(band, 64u64) }

pub const MT_ETBF_TX_FB_CPL: u64 = 0xffff_0000;
pub const MT_ETBF_TX_FB_TRI: u64 = 0xffff;
pub const MT_ETBF_PAR_RPT0_FB_BW: u64 = 192;
pub const MT_ETBF_PAR_RPT0_FB_NC: u64 = 56;
pub const MT_ETBF_PAR_RPT0_FB_NR: u64 = 7;
#[must_use]
#[rustfmt::skip]
pub const fn MT_ETBF_TX_APP_CNT(band: u64) -> u64 { MT_WF_ETBF(band, 240u64) }

pub const MT_ETBF_TX_IBF_CNT: u64 = 0xffff_0000;
pub const MT_ETBF_TX_EBF_CNT: u64 = 0xffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_ETBF_RX_FB_CNT(band: u64) -> u64 { MT_WF_ETBF(band, 248u64) }

pub const MT_ETBF_RX_FB_ALL: u64 = 0xff00_0000;
pub const MT_ETBF_RX_FB_HE: u64 = 0xff_0000;
pub const MT_ETBF_RX_FB_VHT: u64 = 0xff00;
pub const MT_ETBF_RX_FB_HT: u64 = 255;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_LPON_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_b000u64 } else { 0x820e_b000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_LPON(band: u64, ofs: u64) -> u64 { (MT_WF_LPON_BASE(band)).wrapping_add(ofs) }

pub const INT_SOURCE_CSR: u64 = 0;
#[must_use]
#[rustfmt::skip]
pub const fn MT_LPON_TCR(band: u64, n: u64) -> u64 { MT_WF_LPON(band, (168u64).wrapping_add(shl((n).wrapping_mul(4u64), 1u64))) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_LPON_TCR_MT7916(band: u64, n: u64) -> u64 { MT_WF_LPON(band, (168u64).wrapping_add(shl((n).wrapping_mul(4u64), 4u64))) }

pub const MT_LPON_TCR_SW_MODE: u64 = 3;
pub const MT_LPON_TCR_SW_WRITE: u64 = 1;
pub const MT_LPON_TCR_SW_ADJUST: u64 = 2;
pub const MT_LPON_TCR_SW_READ: u64 = 3;
pub const INT_MASK_CSR: u64 = 1;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_MIB_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_d000u64 } else { 0x820e_d000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_MIB(band: u64, ofs: u64) -> u64 { (MT_WF_MIB_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR0(band: u64) -> u64 { MT_WF_MIB(band, 16u64) }

pub const MT_MIB_SDR0_BERACON_TX_CNT_MASK: u64 = 0xffff;
pub const MT_MIB_SDR3_FCS_ERR_MASK: u64 = 0xffff;
pub const MT_MIB_SDR3_FCS_ERR_MASK_MT7916: u64 = 0xffff_0000;
pub const MT_MIB_SDR4_RX_FIFO_FULL_MASK: u64 = 0xffff;
pub const INT1_SOURCE_CSR: u64 = 2;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR6(band: u64) -> u64 { MT_WF_MIB(band, 32u64) }

pub const MT_MIB_SDR6_CHANNEL_IDL_CNT_MASK: u64 = 0xffff;
pub const MT_MIB_SDR7_RX_VECTOR_MISMATCH_CNT_MASK: u64 = 0xffff;
pub const MT_MIB_SDR8_RX_DELIMITER_FAIL_CNT_MASK: u64 = 0xffff;
pub const MT_MIB_SDR9_CCA_BUSY_TIME_MASK: u64 = 0xff_ffff;
pub const MT_MIB_SDR10_MRDY_COUNT_MASK: u64 = 0x3ff_ffff;
pub const INT1_MASK_CSR: u64 = 3;
pub const MT_MIB_SDR10_MRDY_COUNT_MASK_MT7916: u64 = 0xffff_ffff;
pub const MT_MIB_SDR11_RX_LEN_MISMATCH_CNT_MASK: u64 = 0xffff;
pub const MT_MIB_SDR13_TX_STOP_Q_EMPTY_CNT_MASK: u64 = 0xffff;
pub const MT_MIB_SDR14_TX_MPDU_ATTEMPTS_CNT_MASK: u64 = 0xff_ffff;
pub const MT_MIB_SDR14_TX_MPDU_ATTEMPTS_CNT_MASK_MT7916: u64 = 0xffff_ffff;
pub const INT_MCU_CMD_SOURCE: u64 = 4;
pub const MT_MIB_SDR15_TX_MPDU_SUCCESS_CNT_MASK: u64 = 0xff_ffff;
pub const MT_MIB_SDR15_TX_MPDU_SUCCESS_CNT_MASK_MT7916: u64 = 0xffff_ffff;
pub const MT_MIB_SDR16_PRIMARY_CCA_BUSY_TIME_MASK: u64 = 0xff_ffff;
pub const MT_MIB_SDR17_SECONDARY_CCA_BUSY_TIME_MASK: u64 = 0xff_ffff;
pub const MT_MIB_SDR18_PRIMARY_ENERGY_DETECT_TIME_MASK: u64 = 0xff_ffff;
pub const MT_MIB_SDR19_CCK_MDRDY_TIME_MASK: u64 = 0xff_ffff;
pub const MT_MIB_SDR20_OFDM_VHT_MDRDY_TIME_MASK: u64 = 0xff_ffff;
pub const INT_MCU_CMD_EVENT: u64 = 5;
pub const MT_MIB_SDR21_GREEN_MDRDY_TIME_MASK: u64 = 0xff_ffff;
pub const MT_MIB_SDR24_RX_AMPDU_SF_CNT_MASK: u64 = 0xff_ffff;
pub const MT_MIB_SDR24_RX_AMPDU_SF_CNT_MASK_MT7916: u64 = 0xffff_ffff;
pub const WFDMA0_ADDR: u64 = 6;
pub const MT_MIB_SDR27_TX_RWP_FAIL_CNT_MASK: u64 = 0xffff;
pub const MT_MIB_SDR28_TX_RWP_NEED_CNT_MASK: u64 = 0xffff;
pub const MT_MIB_SDR29_RX_PFDROP_CNT_MASK: u64 = 255;
pub const MT_MIB_SDR29_RX_PFDROP_CNT_MASK_MT7916: u64 = 0xffff;
pub const MT_MIB_SDR30_RX_VEC_QUEUE_OVERFLOW_DROP_CNT_MASK: u64 = 0xffff;
pub const MT_MIB_SDR30_RX_VEC_QUEUE_OVERFLOW_DROP_CNT_MASK_MT7916: u64 = 0xffff_0000;
pub const WFDMA0_PCIE1_ADDR: u64 = 7;
pub const MT_MIB_SDR32_TX_PKT_EBF_CNT: u64 = 0xffff;
pub const MT_MIB_SDR32_TX_PKT_IBF_CNT: u64 = 0xffff_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_SDR33(band: u64) -> u64 { MT_WF_MIB(band, 136u64) }

pub const MT_MIB_SDR33_TX_PKT_IBF_CNT: u64 = 0xffff;
pub const MT_MIB_MU_BF_TX_CNT: u64 = 0xffff;
pub const MT_MIB_RTS_RETRIES_COUNT_MASK: u64 = 0xffff_0000;
pub const WFDMA_EXT_CSR_ADDR: u64 = 8;
pub const MT_MIB_RTS_COUNT_MASK: u64 = 0xffff;
pub const MT_MIB_BA_MISS_COUNT_MASK: u64 = 0xffff;
pub const MT_MIB_ACK_FAIL_COUNT_MASK: u64 = 0xffff_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_MB_SDR2(band: u64, n: u64) -> u64 { MT_WF_MIB(band, (0x518u64).wrapping_add(n)) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_MB_BFTF(band: u64, n: u64) -> u64 { MT_WF_MIB(band, (0x510u64).wrapping_add(n)) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_ARNCR_RANGE(val: u64, n: u64) -> u64 { (shr(val, shl(n, 3u64))) & (genmask(7u64, 0u64)) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_BFCR0(band: u64) -> u64 { MT_WF_MIB(band, 0x7b0u64) }

pub const MT_MIB_BFCR0_RX_FB_HT: u64 = 0xffff;
pub const MT_MIB_BFCR0_RX_FB_VHT: u64 = 0xffff_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_BFCR1(band: u64) -> u64 { MT_WF_MIB(band, 0x7b4u64) }

pub const MT_MIB_BFCR1_RX_FB_HE: u64 = 0xffff;
pub const CBTOP1_PHY_END: u64 = 9;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_BFCR2(band: u64) -> u64 { MT_WF_MIB(band, 0x7b8u64) }

pub const MT_MIB_BFCR2_BFEE_TX_FB_TRIG: u64 = 0xffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_BFCR7(band: u64) -> u64 { MT_WF_MIB(band, 0x7ccu64) }

pub const MT_MIB_BFCR7_BFEE_TX_FB_CPL: u64 = 0xffff;
pub const MT_WTBLON_TOP_BASE: u64 = 0x820d_4000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WTBLON_TOP(ofs: u64) -> u64 { (MT_WTBLON_TOP_BASE).wrapping_add(ofs) }

pub const MT_WTBLON_TOP_WDUCR_GROUP: u64 = 7;
pub const MT_WTBL_UPDATE_WLAN_IDX: u64 = 0x3ff;
pub const MT_WTBL_UPDATE_ADM_COUNT_CLEAR: u64 = 0x1000;
pub const MT_WTBL_UPDATE_BUSY: u64 = 0x8000_0000;
pub const INFRA_MCU_ADDR_END: u64 = 10;
pub const MT_WTBL_BASE: u64 = 0x820d_8000;
pub const MT_WTBL_LMAC_ID: u64 = 0x7f00;
pub const MT_WTBL_LMAC_DW: u64 = 252;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_AGG_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_2000u64 } else { 0x820e_2000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_AGG(band: u64, ofs: u64) -> u64 { (MT_WF_AGG_BASE(band)).wrapping_add(ofs) }

pub const MT_AGG_PCR0_MM_PROT: u64 = 1;
pub const MT_AGG_PCR0_GF_PROT: u64 = 2;
pub const MT_AGG_PCR0_BW20_PROT: u64 = 4;
pub const MT_AGG_PCR0_BW40_PROT: u64 = 16;
pub const MT_AGG_PCR0_BW80_PROT: u64 = 64;
pub const MT_AGG_PCR0_ERP_PROT: u64 = 0x1f00;
pub const MT_AGG_PCR0_VHT_PROT: u64 = 0x2000;
pub const MT_AGG_PCR0_PTA_WIN_DIS: u64 = 0x8000;
pub const FW_ASSERT_STAT_ADDR: u64 = 11;
pub const MT_AGG_PCR1_RTS0_NUM_THRES: u64 = 0xff80_0000;
pub const MT_AGG_PCR1_RTS0_LEN_THRES: u64 = 0xf_ffff;
pub const MT_AGG_ACR_CFEND_RATE: u64 = 0x3fff;
pub const MT_AGG_ACR_BAR_RATE: u64 = 0x3fff_0000;
pub const MT_AGG_ACR_PPDU_TXS2H: u64 = 2;
pub const MT_AGG_MRCR_BAR_CNT_LIMIT: u64 = 0xf000;
pub const MT_AGG_MRCR_LAST_RTS_CTS_RN: u64 = 64;
pub const MT_AGG_MRCR_RTS_FAIL_LIMIT: u64 = 0xf80;
pub const MT_AGG_MRCR_TXCMD_RTS_FAIL_LIMIT: u64 = 0x1f00_0000;
pub const MT_AGG_ATCR_MAC_BFF_TIME_EN: u64 = 0x4000_0000;
pub const FW_EXCEPT_TYPE_ADDR: u64 = 12;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_ARB_BASE(band: u64) -> u64 { if (band) != 0 { 0x820f_3000u64 } else { 0x820e_3000u64 } }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_ARB(band: u64, ofs: u64) -> u64 { (MT_WF_ARB_BASE(band)).wrapping_add(ofs) }

pub const MT_ARB_SCR_TX_DISABLE: u64 = 256;
pub const MT_ARB_SCR_RX_DISABLE: u64 = 512;
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
pub const FW_EXCEPT_COUNT_ADDR: u64 = 13;
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
pub const fn MT_WF_RMAC_RSVD0(band: u64) -> u64 { MT_WF_RMAC(band, 736u64) }

pub const FW_CIRQ_COUNT_ADDR: u64 = 14;
pub const MT_WF_RMAC_RSVD0_EIFS_CLR: u64 = 0x20_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_RMAC_MIB_AIRTIME0(band: u64) -> u64 { MT_WF_RMAC(band, 896u64) }

pub const MT_WF_RMAC_MIB_RXTIME_CLR: u64 = 0x8000_0000;
pub const MT_WF_RMAC_MIB_OBSS_BACKOFF: u64 = 0xffff;
pub const MT_WF_RMAC_MIB_ED_OFFSET: u64 = 0x1f_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_RMAC_MIB_AIRTIME1(band: u64) -> u64 { MT_WF_RMAC(band, 900u64) }

pub const MT_WF_RMAC_MIB_NONQOSD_BACKOFF: u64 = 0xffff_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_RMAC_MIB_AIRTIME3(band: u64) -> u64 { MT_WF_RMAC(band, 908u64) }

pub const MT_WF_RMAC_MIB_QOS01_BACKOFF: u64 = 0xffff_ffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_RMAC_MIB_AIRTIME4(band: u64) -> u64 { MT_WF_RMAC(band, 912u64) }

pub const MT_WF_RMAC_MIB_QOS23_BACKOFF: u64 = 0xffff_ffff;
pub const FW_CIRQ_IDX_ADDR: u64 = 15;
pub const MT_WFDMA0_RST_LOGIC_RST: u64 = 16;
pub const MT_WFDMA0_RST_DMASHDL_ALL_RST: u64 = 32;
pub const MT_WFDMA0_BUSY_ENA_TX_FIFO0: u64 = 1;
pub const MT_WFDMA0_BUSY_ENA_TX_FIFO1: u64 = 2;
pub const MT_WFDMA0_BUSY_ENA_RX_FIFO: u64 = 4;
pub const MT_WFDMA0_GLO_CFG_TX_DMA_EN: u64 = 1;
pub const MT_WFDMA0_GLO_CFG_RX_DMA_EN: u64 = 4;
pub const MT_WFDMA0_GLO_CFG_OMIT_TX_INFO: u64 = 0x1000_0000;
pub const MT_WFDMA0_GLO_CFG_OMIT_RX_INFO: u64 = 0x800_0000;
pub const MT_WFDMA0_GLO_CFG_OMIT_RX_INFO_PFET2: u64 = 0x20_0000;
pub const FW_CIRQ_LISR_ADDR: u64 = 16;
pub const MT_WFDMA0_EXT0_RXWB_KEEP: u64 = 0x400;
pub const MT_WFDMA1_BASE: u64 = 0xd_5000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WFDMA1(ofs: u64) -> u64 { (MT_WFDMA1_BASE).wrapping_add(ofs) }

pub const MT_WFDMA1_RST: u64 = 0xd_5100;
pub const MT_WFDMA1_RST_LOGIC_RST: u64 = 16;
pub const MT_WFDMA1_RST_DMASHDL_ALL_RST: u64 = 32;
pub const MT_WFDMA1_BUSY_ENA: u64 = 0xd_513c;
pub const MT_WFDMA1_BUSY_ENA_TX_FIFO0: u64 = 1;
pub const MT_WFDMA1_BUSY_ENA_TX_FIFO1: u64 = 2;
pub const MT_WFDMA1_BUSY_ENA_RX_FIFO: u64 = 4;
pub const FW_TASK_ID_ADDR: u64 = 17;
pub const MT_WFDMA1_GLO_CFG: u64 = 0xd_5208;
pub const MT_WFDMA1_GLO_CFG_TX_DMA_EN: u64 = 1;
pub const MT_WFDMA1_GLO_CFG_RX_DMA_EN: u64 = 4;
pub const MT_WFDMA1_GLO_CFG_OMIT_TX_INFO: u64 = 0x1000_0000;
pub const MT_WFDMA1_GLO_CFG_OMIT_RX_INFO: u64 = 0x800_0000;
pub const MT_WFDMA1_GLO_CFG_OMIT_RX_INFO_PFET2: u64 = 0x20_0000;
pub const MT_WFDMA1_RST_DTX_PTR: u64 = 0xd_520c;
pub const MT_WFDMA1_PRI_DLY_INT_CFG0: u64 = 0xd_52f0;
pub const MT_WFDMA_EXT_CSR_PHYS_BASE: u64 = 0x1802_7000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WFDMA_EXT_CSR_PHYS(ofs: u64) -> u64 { (MT_WFDMA_EXT_CSR_PHYS_BASE).wrapping_add(ofs) }

pub const MT_WFDMA_HOST_CONFIG: u64 = 0x1802_7030;
pub const FW_TASK_IDX_ADDR: u64 = 18;
pub const MT_WFDMA_HOST_CONFIG_PDMA_BAND: u64 = 1;
pub const MT_WFDMA_HOST_CONFIG_WED: u64 = 2;
pub const MT_WFDMA_WED_RING_CONTROL: u64 = 0x1802_7034;
pub const MT_WFDMA_WED_RING_CONTROL_TX0: u64 = 31;
pub const MT_WFDMA_WED_RING_CONTROL_TX1: u64 = 0x1f00;
pub const MT_WFDMA_WED_RING_CONTROL_RX1: u64 = 0x1f_0000;
pub const MT_WFDMA_EXT_CSR_HIF_MISC: u64 = 0x1802_7044;
pub const MT_WFDMA_EXT_CSR_HIF_MISC_BUSY: u64 = 1;
pub const MT_PCIE_RECOG_ID: u64 = 0xd_7090;
pub const MT_PCIE_RECOG_ID_MASK: u64 = 0x7fff_ffff;
pub const MT_PCIE_RECOG_ID_SEM: u64 = 0x8000_0000;
pub const FW_TASK_QID1_ADDR: u64 = 19;
pub const MT_WFDMA0_PCIE1_BUSY_ENA_TX_FIFO0: u64 = 1;
pub const MT_WFDMA0_PCIE1_BUSY_ENA_TX_FIFO1: u64 = 2;
pub const MT_WFDMA0_PCIE1_BUSY_ENA_RX_FIFO: u64 = 4;
pub const MT_WFDMA1_PCIE1_BASE: u64 = 0xd_9000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WFDMA1_PCIE1(ofs: u64) -> u64 { (MT_WFDMA1_PCIE1_BASE).wrapping_add(ofs) }

pub const MT_WFDMA1_PCIE1_BUSY_ENA: u64 = 0xd_913c;
pub const MT_WFDMA1_PCIE1_BUSY_ENA_TX_FIFO0: u64 = 1;
pub const MT_WFDMA1_PCIE1_BUSY_ENA_TX_FIFO1: u64 = 2;
pub const MT_WFDMA1_PCIE1_BUSY_ENA_RX_FIFO: u64 = 4;
pub const FW_TASK_QID2_ADDR: u64 = 20;
pub const FW_TASK_START_ADDR: u64 = 21;
pub const MT_INT_RX_DONE_BAND0: u64 = 0x1_0000;
pub const MT_INT_RX_DONE_BAND1: u64 = 0x2_0000;
pub const MT_INT_RX_DONE_WM: u64 = 1;
pub const MT_INT_RX_DONE_WA: u64 = 2;
pub const MT_INT_RX_DONE_WA_MAIN: u64 = 2;
pub const MT_INT_RX_DONE_WA_EXT: u64 = 4;
pub const MT_INT_MCU_CMD: u64 = 0x2000_0000;
pub const MT_INT_RX_DONE_BAND0_MT7916: u64 = 0x40_0000;
pub const MT_INT_RX_DONE_BAND1_MT7916: u64 = 0x80_0000;
pub const MT_INT_RX_DONE_WA_MAIN_MT7916: u64 = 4;
pub const MT_INT_RX_DONE_WA_EXT_MT7916: u64 = 8;
pub const FW_TASK_END_ADDR: u64 = 22;
pub const MT_INT_WED_RX_DONE_BAND0_MT7916: u64 = 0x4_0000;
pub const MT_INT_WED_RX_DONE_BAND1_MT7916: u64 = 0x8_0000;
pub const MT_INT_WED_RX_DONE_WA_MAIN_MT7916: u64 = 2;
pub const MT_INT_WED_RX_DONE_WA_MT7916: u64 = 0x2_0000;
pub const MT_INT_TX_DONE_FWDL: u64 = 0x400_0000;
pub const MT_INT_TX_DONE_MCU_WM: u64 = 0x800_0000;
pub const MT_INT_TX_DONE_MCU_WA: u64 = 0x8000;
pub const FW_TASK_SIZE_ADDR: u64 = 23;
pub const MT_INT_TX_DONE_BAND0: u64 = 0x4000_0000;
pub const MT_INT_TX_DONE_BAND1: u64 = 0x8000_0000;
pub const MT_INT_TX_DONE_MCU_WA_MT7916: u64 = 0x200_0000;
pub const MT_INT_WED_TX_DONE_BAND0: u64 = 16;
pub const MT_INT_WED_TX_DONE_BAND1: u64 = 32;
pub const MT_MCU_CMD_STOP_DMA_FW_RELOAD: u64 = 2;
pub const MT_MCU_CMD_STOP_DMA: u64 = 4;
pub const MT_MCU_CMD_RESET_DONE: u64 = 8;
pub const MT_MCU_CMD_RECOVERY_DONE: u64 = 16;
pub const MT_MCU_CMD_NORMAL_STATE: u64 = 32;
pub const MT_MCU_CMD_ERROR_MASK: u64 = 62;
pub const MT_MCU_CMD_WA_WDT: u64 = 0x8000_0000;
pub const MT_MCU_CMD_WM_WDT: u64 = 0x4000_0000;
pub const MT_MCU_CMD_WDT_MASK: u64 = 0xc000_0000;
pub const FW_LAST_MSG_ID_ADDR: u64 = 24;
pub const MT_TOP_RGU_BASE: u64 = 0x1800_0000;
pub const MT_TOP_PWR_CTRL: u64 = 0x1800_0000;
pub const MT_TOP_PWR_KEY: u64 = 0x5746_0000;
pub const MT_TOP_PWR_SW_RST: u64 = 1;
pub const MT_TOP_PWR_SW_PWR_ON: u64 = 12;
pub const MT_TOP_PWR_HW_CTRL: u64 = 16;
pub const MT_TOP_PWR_PWR_ON: u64 = 128;
pub const MT_TOP_RGU_SYSRAM_PDN: u64 = 0x1800_0050;
pub const MT_TOP_RGU_SYSRAM_SLP: u64 = 0x1800_0054;
pub const MT_TOP_WFSYS_PWR: u64 = 0x1800_0010;
pub const MT_TOP_PWR_EN_MASK: u64 = 128;
pub const MT_TOP_PWR_ACK_MASK: u64 = 64;
pub const MT_TOP_PWR_KEY_MASK: u64 = 0xffff_0000;
pub const MT7986_TOP_WM_RESET: u64 = 0x1800_0120;
pub const MT7986_TOP_WM_RESET_MASK: u64 = 1;
pub const MT_HIF_REMAP_L1: u64 = 0xf_11ac;
pub const MT_HIF_REMAP_L1_MT7916: u64 = 0xf_e260;
pub const FW_EINT_INFO_ADDR: u64 = 25;
pub const MT_HIF_REMAP_L1_MASK: u64 = 0xffff;
pub const MT_HIF_REMAP_L1_OFFSET: u64 = 0xffff;
pub const MT_HIF_REMAP_L1_BASE: u64 = 0xffff_0000;
pub const MT_HIF_REMAP_BASE_L1: u64 = 0xe_0000;
pub const MT_HIF_REMAP_L2: u64 = 0xf_11b0;
pub const MT_HIF_REMAP_L2_MASK: u64 = 0xf_ffff;
pub const MT_HIF_REMAP_L2_OFFSET: u64 = 0xfff;
pub const MT_HIF_REMAP_L2_BASE: u64 = 0xffff_f000;
pub const MT_HIF_REMAP_L2_MT7916: u64 = 440;
pub const MT_HIF_REMAP_L2_MASK_MT7916: u64 = 0xffff_0000;
pub const MT_HIF_REMAP_L2_OFFSET_MT7916: u64 = 0xffff;
pub const MT_HIF_REMAP_L2_BASE_MT7916: u64 = 0xffff_0000;
pub const MT_HIF_REMAP_BASE_L2_MT7916: u64 = 0x4_0000;
pub const MT_INFRA_BASE: u64 = 0x1800_0000;
pub const MT_WFSYS0_PHY_START: u64 = 0x1840_0000;
pub const MT_WFSYS1_PHY_START: u64 = 0x1880_0000;
pub const MT_WFSYS1_PHY_END: u64 = 0x18bf_ffff;
pub const MT_CBTOP1_PHY_START: u64 = 0x7000_0000;
pub const FW_SCHED_INFO_ADDR: u64 = 26;
pub const MT_CBTOP2_PHY_START: u64 = 0xf000_0000;
pub const MT_INFRA_MCU_START: u64 = 0x7c00_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_CONN_INFRA_OFFSET(p: u64) -> u64 { (p).wrapping_sub(MT_INFRA_BASE) }

pub const MT_CONN_INFRA_BASE: u64 = 0x1800_1000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_CONN_INFRA(ofs: u64) -> u64 { (MT_CONN_INFRA_BASE).wrapping_add(ofs) }

pub const MT_CONN_INFRA_EFUSE: u64 = 0x1800_1020;
pub const MT_CONN_INFRA_ADIE_RESET: u64 = 0x1800_1030;
pub const MT_CONN_INFRA_ADIE1_RESET_MASK: u64 = 1;
pub const MT_CONN_INFRA_ADIE2_RESET_MASK: u64 = 4;
pub const MT_CONN_INFRA_OSC_RC_EN: u64 = 0x1800_1380;
pub const MT_CONN_INFRA_OSC_CTRL: u64 = 0x1800_1300;
pub const MT_CONN_INFRA_OSC_RC_EN_MASK: u64 = 128;
pub const MT_CONN_INFRA_OSC_STB_TIME_MASK: u64 = 0xff_ffff;
pub const SWDEF_BASE_ADDR: u64 = 27;
pub const MT_CONN_INFRA_HW_CTRL: u64 = 0x1800_1200;
pub const MT_CONN_INFRA_HW_CTRL_MASK: u64 = 1;
pub const MT_CONN_INFRA_WF_SLP_PROT: u64 = 0x1800_1540;
pub const MT_CONN_INFRA_WF_SLP_PROT_MASK: u64 = 1;
pub const MT_CONN_INFRA_WF_SLP_PROT_RDY: u64 = 0x1800_1544;
pub const MT_CONN_INFRA_CONN_WF_MASK: u64 = 0xa000_0000;
pub const MT_CONN_INFRA_CONN: u64 = 0xa200_0000;
pub const MT_CONN_INFRA_EMI_REQ: u64 = 0x1800_1414;
pub const MT_CONN_INFRA_EMI_REQ_MASK: u64 = 1;
pub const MT_CONN_INFRA_INFRA_REQ_MASK: u64 = 32;
#[must_use]
#[rustfmt::skip]
pub const fn MT_AFE_CTRL_BASE(band: u64) -> u64 { (0x1800_3000u64).wrapping_add(shl(band, 19u64)) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_AFE_CTRL(band: u64, ofs: u64) -> u64 { (MT_AFE_CTRL_BASE(band)).wrapping_add(ofs) }

pub const TXQ_WED_RING_BASE: u64 = 28;
#[must_use]
#[rustfmt::skip]
pub const fn MT_AFE_DIG_EN_01(band: u64) -> u64 { MT_AFE_CTRL(band, 0u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_AFE_DIG_EN_02(band: u64) -> u64 { MT_AFE_CTRL(band, 4u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_AFE_DIG_EN_03(band: u64) -> u64 { MT_AFE_CTRL(band, 8u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_AFE_DIG_TOP_01(band: u64) -> u64 { MT_AFE_CTRL(band, 12u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_AFE_PLL_STB_TIME(band: u64) -> u64 { MT_AFE_CTRL(band, 244u64) }

pub const MT_AFE_PLL_STB_TIME_MASK: u64 = 0x7fff_7fff;
pub const MT_AFE_BPLL_CFG_MASK: u64 = 192;
pub const MT_AFE_WPLL_CFG_MASK: u64 = 3;
pub const MT_AFE_MCU_WPLL_CFG_MASK: u64 = 12;
pub const MT_AFE_MCU_BPLL_CFG_MASK: u64 = 0x3_0000;
pub const MT_AFE_PLL_CFG_MASK: u64 = 0x3_00cf;
pub const MT_AFE_DIG_TOP_01_MASK: u64 = 0x7_8000;
pub const MT_AFE_RG_WBG_EN_RCK_MASK: u64 = 1;
pub const RXQ_WED_RING_BASE: u64 = 29;
pub const MT_AFE_RG_WBG_EN_BPLL_UP_MASK: u64 = 0x20_0000;
pub const MT_AFE_RG_WBG_EN_WPLL_UP_MASK: u64 = 0x10_0000;
pub const MT_AFE_RG_WBG_EN_PLL_UP_MASK: u64 = 0x30_0000;
pub const MT_AFE_RG_WBG_EN_TXCAL_WF4: u64 = 0x2000_0000;
pub const MT_AFE_RG_WBG_EN_TXCAL_BT: u64 = 0x20_0000;
pub const MT_AFE_RG_WBG_EN_TXCAL_WF3: u64 = 0x10_0000;
pub const MT_AFE_RG_WBG_EN_TXCAL_WF2: u64 = 0x8_0000;
pub const MT_AFE_RG_WBG_EN_TXCAL_WF1: u64 = 0x4_0000;
pub const MT_AFE_RG_WBG_EN_TXCAL_WF0: u64 = 0x2_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_ADIE_SLP_CTRL_BASE(band: u64) -> u64 { (0x1800_5000u64).wrapping_add(shl(band, 19u64)) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_ADIE_SLP_CTRL(band: u64, ofs: u64) -> u64 { (MT_ADIE_SLP_CTRL_BASE(band)).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_ADIE_SLP_CTRL_CK0(band: u64) -> u64 { MT_ADIE_SLP_CTRL(band, 288u64) }

pub const MT_ADIE_CHIP_ID: u64 = 44;
pub const MT_ADIE_VERSION_MASK: u64 = 0xffff;
pub const MT_ADIE_CHIP_ID_MASK: u64 = 0xffff_0000;
pub const MT_ADIE_IDX0: u64 = 0xffff;
pub const RXQ_WED_DATA_RING_BASE: u64 = 30;
pub const MT_ADIE_IDX1: u64 = 0xffff_0000;
pub const MT_ADIE_RG_TOP_THADC_BG: u64 = 52;
pub const MT_ADIE_VRPI_SEL_CR_MASK: u64 = 0xf000;
pub const MT_ADIE_VRPI_SEL_EFUSE_MASK: u64 = 120;
pub const MT_ADIE_RG_TOP_THADC: u64 = 56;
pub const MT_ADIE_PGA_GAIN_MASK: u64 = 0x380_0000;
pub const MT_ADIE_PGA_GAIN_EFUSE_MASK: u64 = 7;
pub const MT_ADIE_LDO_CTRL_MASK: u64 = 0xc00_0000;
pub const MT_ADIE_LDO_CTRL_EFUSE_MASK: u64 = 96;
pub const MT_AFE_RG_ENCAL_WBTAC_IF_SW: u64 = 112;
pub const MT_ADIE_EFUSE_RDATA0: u64 = 304;
pub const MT_ADIE_EFUSE2_CTRL: u64 = 328;
pub const MT_ADIE_EFUSE_CTRL_MASK: u64 = 2;
pub const MT_ADIE_EFUSE_CFG: u64 = 324;
pub const MT_ADIE_EFUSE_MODE_MASK: u64 = 192;
pub const MT_ADIE_EFUSE_ADDR_MASK: u64 = 0x3ff_0000;
pub const MT_ADIE_EFUSE_VALID_MASK: u64 = 0x2000_0000;
pub const MT_ADIE_EFUSE_KICK_MASK: u64 = 0x4000_0000;
pub const MT_ADIE_THADC_ANALOG: u64 = 934;
pub const __MT_REG_MAX: u64 = 31;
pub const MT_ADIE_THADC_SLOP: u64 = 935;
pub const MT_ADIE_ANA_EN_MASK: u64 = 128;
pub const MT_ADIE_7975_XTAL_CAL: u64 = 929;
pub const MT_ADIE_TRIM_MASK: u64 = 127;
pub const MT_ADIE_EFUSE_TRIM_MASK: u64 = 63;
pub const MT_ADIE_XO_TRIM_EN_MASK: u64 = 128;
pub const MT_ADIE_XTAL_DECREASE_MASK: u64 = 64;
pub const MT_ADIE_7975_XO_TRIM2: u64 = 930;
pub const MT_ADIE_7975_XO_TRIM3: u64 = 931;
pub const MT_ADIE_7975_XO_TRIM4: u64 = 932;
pub const MT_ADIE_7975_XTAL_EN: u64 = 933;
pub const MT_ADIE_XO_TRIM_FLOW: u64 = 940;
pub const MT_ADIE_XTAL_AXM_80M_OSC: u64 = 912;
pub const MT_ADIE_XTAL_AXM_40M_OSC: u64 = 913;
pub const MT_ADIE_XTAL_TRIM1_80M_OSC: u64 = 920;
pub const MT_ADIE_XTAL_TRIM1_40M_OSC: u64 = 921;
pub const MT_ADIE_WRI_CK_SEL: u64 = 0x4ac;
pub const MT_ADIE_RG_STRAP_PIN_IN: u64 = 0x4fc;
pub const MT_ADIE_XTAL_C1: u64 = 0x654;
pub const MT_ADIE_XTAL_C2: u64 = 0x658;
pub const MT_ADIE_RG_XO_01: u64 = 0x65c;
pub const MT_ADIE_RG_XO_03: u64 = 0x664;
pub const MT_ADIE_CLK_EN: u64 = 0xa00;
pub const MT_ADIE_7975_XTAL: u64 = 0xa18;
pub const MT_ADIE_7975_XTAL_EN_MASK: u64 = 0x2000_0000;
pub const MT_ADIE_7975_COCLK: u64 = 0xa1c;
pub const MT_ADIE_7975_XO_2: u64 = 0xa84;
pub const MT_ADIE_7975_XO_2_FIX_EN: u64 = 0x8000_0000;
pub const TMAC_CDTR: u64 = 0;
pub const MT_ADIE_7975_XO_CTRL2: u64 = 0xa94;
pub const MT_ADIE_7975_XO_CTRL2_C1_MASK: u64 = 0x7f0_0000;
pub const MT_ADIE_7975_XO_CTRL2_C2_MASK: u64 = 0x7_f000;
pub const MT_ADIE_7975_XO_CTRL2_MASK: u64 = 0x7f7_f000;
pub const MT_ADIE_7975_XO_CTRL6: u64 = 0xaa4;
pub const MT_ADIE_7975_XO_CTRL6_MASK: u64 = 0x1_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP_SPI_ADIE_BASE(band: u64) -> u64 { (0x1800_4000u64).wrapping_add(shl(band, 19u64)) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP_SPI_ADIE(band: u64, ofs: u64) -> u64 { (MT_TOP_SPI_ADIE_BASE(band)).wrapping_add(ofs) }

pub const TMAC_ODTR: u64 = 1;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP_SPI_BUSY_CR(band: u64) -> u64 { MT_TOP_SPI_ADIE(band, 0u64) }

pub const MT_TOP_SPI_POLLING_BIT: u64 = 32;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP_SPI_ADDR_CR(band: u64) -> u64 { MT_TOP_SPI_ADIE(band, 80u64) }

pub const MT_TOP_SPI_READ_ADDR_FORMAT: u64 = 0xb000;
pub const MT_TOP_SPI_WRITE_ADDR_FORMAT: u64 = 0xa000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP_SPI_WRITE_DATA_CR(band: u64) -> u64 { MT_TOP_SPI_ADIE(band, 84u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP_SPI_READ_DATA_CR(band: u64) -> u64 { MT_TOP_SPI_ADIE(band, 88u64) }

pub const TMAC_ATCR: u64 = 2;
pub const MT_INFRA_CKGEN_BASE: u64 = 0x1800_9000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_INFRA_CKGEN(ofs: u64) -> u64 { (MT_INFRA_CKGEN_BASE).wrapping_add(ofs) }

pub const MT_INFRA_CKGEN_BUS: u64 = 0x1800_9a00;
pub const MT_INFRA_CKGEN_BUS_CLK_SEL_MASK: u64 = 0x80_0000;
pub const MT_INFRA_CKGEN_BUS_RDY_SEL_MASK: u64 = 0x2000_0000;
pub const MT_INFRA_CKGEN_BUS_WPLL_DIV_1: u64 = 0x1800_9008;
pub const MT_INFRA_CKGEN_BUS_WPLL_DIV_2: u64 = 0x1800_900c;
pub const MT_INFRA_CKGEN_RFSPI_WPLL_DIV: u64 = 0x1800_9040;
pub const MT_INFRA_CKGEN_DIV_SEL_MASK: u64 = 252;
pub const TMAC_TRCR0: u64 = 3;
pub const MT_INFRA_CKGEN_DIV_EN_MASK: u64 = 1;
pub const MT_INFRA_BUS_BASE: u64 = 0x1800_e000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_INFRA_BUS(ofs: u64) -> u64 { (MT_INFRA_BUS_BASE).wrapping_add(ofs) }

pub const MT_INFRA_BUS_OFF_TIMEOUT: u64 = 0x1800_e300;
pub const MT_INFRA_BUS_TIMEOUT_LIMIT_MASK: u64 = 0x7f80;
pub const MT_INFRA_BUS_TIMEOUT_EN_MASK: u64 = 15;
pub const MT_INFRA_BUS_ON_TIMEOUT: u64 = 0x1800_e31c;
pub const MT_INFRA_BUS_EMI_START: u64 = 0x1800_e360;
pub const MT_INFRA_BUS_EMI_END: u64 = 0x1800_e364;
pub const TMAC_ICR0: u64 = 4;
pub const MT_CONNINFRA_SKU_DEC_ADDR: u64 = 0x1805_0000;
pub const MT_CONNINFRA_SKU_MASK: u64 = 0xffff;
pub const MT_ADIE_TYPE_MASK: u64 = 2;
pub const TMAC_ICR1: u64 = 5;
pub const TMAC_CTCR0: u64 = 6;
pub const MT_SWDEF_NORMAL_MODE: u64 = 0;
pub const MT_SWDEF_ICAP_MODE: u64 = 1;
pub const MT_SWDEF_SPECTRUM_MODE: u64 = 2;
pub const MT_SWDEF_PLE1_MDP_RIOC_HANG_ERR: u64 = 8;
pub const TMAC_TFCR0: u64 = 7;
pub const MT_DIC_CMD_REG_BASE: u64 = 0x41_f000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_DIC_CMD_REG(ofs: u64) -> u64 { (MT_DIC_CMD_REG_BASE).wrapping_add(ofs) }

pub const MT_DIC_CMD_REG_CMD: u64 = 0x41_f010;
pub const MT_CPU_UTIL_BASE: u64 = 0x41_f030;
#[must_use]
#[rustfmt::skip]
pub const fn MT_CPU_UTIL(ofs: u64) -> u64 { (MT_CPU_UTIL_BASE).wrapping_add(ofs) }

pub const MDP_BNRCFR0: u64 = 8;
pub const MT_CPU_UTIL_BUSY_PCT: u64 = 0x41_f030;
pub const MT_CPU_UTIL_PEAK_BUSY_PCT: u64 = 0x41_f034;
pub const MT_CPU_UTIL_IDLE_CNT: u64 = 0x41_f038;
pub const MT_CPU_UTIL_PEAK_IDLE_CNT: u64 = 0x41_f03c;
pub const MT_CPU_UTIL_CTRL: u64 = 0x41_f04c;
pub const MT_LED_TOP_BASE: u64 = 0x1801_3000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_LED_PHYS(n: u64) -> u64 { (MT_LED_TOP_BASE).wrapping_add(n) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_LED_CTRL(n: u64) -> u64 { MT_LED_PHYS((0u64).wrapping_add((n).wrapping_mul(4u64))) }

pub const MT_LED_CTRL_KICK: u64 = 128;
pub const MT_LED_CTRL_BAND: u64 = 16;
pub const MT_LED_CTRL_BLINK_MODE: u64 = 4;
pub const MDP_BNRCFR1: u64 = 9;
pub const MT_LED_CTRL_POLARITY: u64 = 2;
#[must_use]
#[rustfmt::skip]
pub const fn MT_LED_TX_BLINK(n: u64) -> u64 { MT_LED_PHYS((16u64).wrapping_add((n).wrapping_mul(4u64))) }

pub const MT_LED_TX_BLINK_ON_MASK: u64 = 255;
pub const MT_LED_TX_BLINK_OFF_MASK: u64 = 0xff00;
#[must_use]
#[rustfmt::skip]
pub const fn MT_LED_STATUS_0(n: u64) -> u64 { MT_LED_PHYS((32u64).wrapping_add((n).wrapping_mul(8u64))) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_LED_STATUS_1(n: u64) -> u64 { MT_LED_PHYS((36u64).wrapping_add((n).wrapping_mul(8u64))) }

pub const MT_LED_STATUS_OFF: u64 = 0xff00_0000;
pub const MT_LED_STATUS_ON: u64 = 0xff_0000;
pub const MT_LED_STATUS_DURATION: u64 = 0xffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_LED_EN(n: u64) -> u64 { MT_LED_PHYS((64u64).wrapping_add((n).wrapping_mul(4u64))) }

pub const ARB_DRNGR0: u64 = 10;
pub const MT_LED_GPIO_MUX0: u64 = 0x7000_5050;
pub const MT_LED_GPIO_MUX1: u64 = 0x7000_5054;
pub const MT_LED_GPIO_MUX2: u64 = 0x7000_5058;
pub const MT_LED_GPIO_MUX3: u64 = 0x7000_505c;
pub const MT_TOP_BASE: u64 = 0x1806_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP(ofs: u64) -> u64 { (MT_TOP_BASE).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP_LPCR_HOST_BAND(band: u64) -> u64 { MT_TOP((16u64).wrapping_add((band).wrapping_mul(16u64))) }

pub const MT_TOP_LPCR_HOST_FW_OWN: u64 = 1;
pub const MT_TOP_LPCR_HOST_DRV_OWN: u64 = 2;
pub const MT_TOP_LPCR_HOST_FW_OWN_STAT: u64 = 4;
pub const ARB_SCR: u64 = 11;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP_LPCR_HOST_BAND_IRQ_STAT(band: u64) -> u64 { MT_TOP((20u64).wrapping_add((band).wrapping_mul(16u64))) }

pub const MT_TOP_LPCR_HOST_BAND_STAT: u64 = 1;
pub const MT_TOP_MISC: u64 = 0x1806_00f0;
pub const MT_TOP_MISC_FW_STATE: u64 = 7;
pub const MT_TOP_WFSYS_WAKEUP: u64 = 0x1806_01a4;
pub const MT_TOP_WFSYS_WAKEUP_MASK: u64 = 1;
pub const RMAC_MIB_AIRTIME14: u64 = 12;
pub const MT_TOP_MCU_EMI_BASE: u64 = 0x1806_01c4;
pub const MT_TOP_MCU_EMI_BASE_MASK: u64 = 0xf_ffff;
pub const MT_TOP_WF_AP_PERI_BASE: u64 = 0x1806_01c8;
pub const MT_TOP_WF_AP_PERI_BASE_MASK: u64 = 0xf_ffff;
pub const MT_TOP_EFUSE_BASE: u64 = 0x1806_01cc;
pub const MT_TOP_EFUSE_BASE_MASK: u64 = 0xf_ffff;
pub const MT_TOP_CONN_INFRA_WAKEUP: u64 = 0x1806_01a0;
pub const MT_TOP_CONN_INFRA_WAKEUP_MASK: u64 = 1;
pub const MT_TOP_WFSYS_RESET_STATUS: u64 = 0x1806_02cc;
pub const MT_TOP_WFSYS_RESET_STATUS_MASK: u64 = 0x4000_0000;
pub const MT_SEMA_BASE: u64 = 0x1807_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_SEMA(ofs: u64) -> u64 { (MT_SEMA_BASE).wrapping_add(ofs) }

pub const MT_SEMA_RFSPI_STATUS: u64 = 0x1807_202c;
pub const MT_SEMA_RFSPI_RELEASE: u64 = 0x1807_222c;
pub const AGG_AWSCR0: u64 = 13;
pub const MT_SEMA_RFSPI_STATUS_MASK: u64 = 2;
pub const MT_MCU_BUS_BASE: u64 = 0x1840_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MCU_BUS(ofs: u64) -> u64 { (MT_MCU_BUS_BASE).wrapping_add(ofs) }

pub const MT_MCU_BUS_TIMEOUT: u64 = 0x184f_0440;
pub const MT_MCU_BUS_TIMEOUT_SET_MASK: u64 = 255;
pub const MT_MCU_BUS_TIMEOUT_CG_EN_MASK: u64 = 0x1000_0000;
pub const MT_MCU_BUS_TIMEOUT_EN_MASK: u64 = 0x8000_0000;
pub const MT_MCU_BUS_REMAP: u64 = 0x1840_0120;
pub const AGG_PCR0: u64 = 14;
pub const MT_TOP_CFG_BASE: u64 = 0x184b_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP_CFG(ofs: u64) -> u64 { (MT_TOP_CFG_BASE).wrapping_add(ofs) }

pub const MT_TOP_CFG_IP_VERSION_ADDR: u64 = 0x184b_0010;
pub const MT_TOP_CFG_ON_BASE: u64 = 0x184c_1000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP_CFG_ON(ofs: u64) -> u64 { (MT_TOP_CFG_ON_BASE).wrapping_add(ofs) }

pub const MT_TOP_CFG_ON_ROM_IDX: u64 = 0x184c_1604;
pub const AGG_ACR0: u64 = 15;
pub const MT_SLP_BASE: u64 = 0x184c_3000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_SLP(ofs: u64) -> u64 { (MT_SLP_BASE).wrapping_add(ofs) }

pub const MT_SLP_STATUS: u64 = 0x184c_300c;
pub const MT_SLP_WFDMA2CONN_MASK: u64 = 0xa0_0000;
pub const MT_SLP_CTRL_EN_MASK: u64 = 1;
pub const MT_SLP_CTRL_BSY_MASK: u64 = 2;
pub const AGG_ACR4: u64 = 16;
pub const MT_MCU_BUS_DBG_BASE: u64 = 0x1850_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MCU_BUS_DBG(ofs: u64) -> u64 { (MT_MCU_BUS_DBG_BASE).wrapping_add(ofs) }

pub const MT_MCU_BUS_DBG_TIMEOUT: u64 = 0x1850_0000;
pub const MT_MCU_BUS_DBG_TIMEOUT_SET_MASK: u64 = 0xffff_0000;
pub const MT_MCU_BUS_DBG_TIMEOUT_CK_EN_MASK: u64 = 8;
pub const MT_MCU_BUS_DBG_TIMEOUT_EN_MASK: u64 = 4;
pub const MT_HW_BOUND: u64 = 0x7001_0020;
pub const MT_HW_REV: u64 = 0x7001_0204;
pub const MT_WF_SUBSYS_RST: u64 = 0x7000_2600;
pub const AGG_MRCR: u64 = 17;
pub const MT_PCIE_MAC_BASE: u64 = 0x7403_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_PCIE_MAC(ofs: u64) -> u64 { (MT_PCIE_MAC_BASE).wrapping_add(ofs) }

pub const MT_PCIE_MAC_INT_ENABLE: u64 = 0x7403_0188;
pub const MT_PCIE1_MAC_INT_ENABLE: u64 = 0x7402_0188;
pub const MT_PCIE1_MAC_INT_ENABLE_MT7916: u64 = 0x7409_0188;
pub const MT_WM_MCU_PC: u64 = 0x7c06_0204;
pub const MT_WA_MCU_PC: u64 = 0x7c06_020c;
pub const AGG_ATCR0: u64 = 18;
pub const MT_WF_PP_TOP_BASE: u64 = 0x820c_c000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PP_TOP(ofs: u64) -> u64 { (MT_WF_PP_TOP_BASE).wrapping_add(ofs) }

pub const MT_WF_PP_TOP_RXQ_WFDMA_CF_5: u64 = 0x820c_c0e8;
pub const MT_WF_PP_TOP_RXQ_QID6_WFDMA_HIF_SEL_MASK: u64 = 64;
pub const MT_WF_IRPI_BASE: u64 = 0x8300_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_IRPI(ofs: u64) -> u64 { (MT_WF_IRPI_BASE).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_IRPI_NSS(phy: u64, nss: u64) -> u64 { MT_WF_IRPI(((0x6000u64).wrapping_add(shl(phy, 20u64))).wrapping_add(shl(nss, 16u64))) }

pub const AGG_ATCR1: u64 = 19;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_IRPI_NSS_MT7916(phy: u64, nss: u64) -> u64 { MT_WF_IRPI(((0x1000u64).wrapping_add(shl(phy, 20u64))).wrapping_add(shl(nss, 16u64))) }

pub const MT_WF_PHY_BASE: u64 = 0x8308_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHY(ofs: u64) -> u64 { (MT_WF_PHY_BASE).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHY_RX_CTRL1(phy: u64) -> u64 { MT_WF_PHY((0x2004u64).wrapping_add(shl(phy, 16u64))) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHY_RX_CTRL1_MT7916(phy: u64) -> u64 { MT_WF_PHY((0x2004u64).wrapping_add(shl(phy, 20u64))) }

pub const MT_WF_PHY_RX_CTRL1_IPI_EN: u64 = 7;
pub const MT_WF_PHY_RX_CTRL1_STSCNT_EN: u64 = 0xe00;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHY_RXTD12(phy: u64) -> u64 { MT_WF_PHY((0x8230u64).wrapping_add(shl(phy, 16u64))) }

pub const AGG_ATCR3: u64 = 20;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHY_RXTD12_MT7916(phy: u64) -> u64 { MT_WF_PHY((0x8230u64).wrapping_add(shl(phy, 20u64))) }

pub const MT_WF_PHY_RXTD12_IRPI_SW_CLR_ONLY: u64 = 0x4_0000;
pub const MT_WF_PHY_RXTD12_IRPI_SW_CLR: u64 = 0x2000_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHY_TPC_CTRL_STAT(phy: u64) -> u64 { MT_WF_PHY((0xe7a0u64).wrapping_add(shl(phy, 16u64))) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHY_TPC_CTRL_STAT_MT7916(phy: u64) -> u64 { MT_WF_PHY((0xe7a0u64).wrapping_add(shl(phy, 20u64))) }

pub const MT_WF_PHY_TPC_POWER: u64 = 0xff00;
pub const MT_MCU_WM_CIRQ_BASE: u64 = 0x8901_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MCU_WM_CIRQ(ofs: u64) -> u64 { (MT_MCU_WM_CIRQ_BASE).wrapping_add(ofs) }

pub const MT_MCU_WM_CIRQ_IRQ_MASK_CLR_ADDR: u64 = 0x8901_0080;
pub const MT_MCU_WM_CIRQ_IRQ_SOFT_ADDR: u64 = 0x8901_00c0;
pub const LPON_UTTR0: u64 = 21;
pub const MT_MCU_WM_CIRQ_EINT_MASK_CLR_ADDR: u64 = 0x8901_0108;
pub const MT_MCU_WM_CIRQ_EINT_SOFT_ADDR: u64 = 0x8901_0118;
pub const LPON_UTTR1: u64 = 22;
pub const LPON_FRCR: u64 = 23;
pub const MIB_SDR3: u64 = 24;
pub const MIB_SDR4: u64 = 25;
pub const MIB_SDR5: u64 = 26;
pub const MIB_SDR7: u64 = 27;
pub const MIB_SDR8: u64 = 28;
pub const MIB_SDR9: u64 = 29;
pub const MIB_SDR10: u64 = 30;
pub const MIB_SDR11: u64 = 31;
pub const MIB_SDR12: u64 = 32;
pub const MIB_SDR13: u64 = 33;
pub const MIB_SDR14: u64 = 34;
pub const MIB_SDR15: u64 = 35;
pub const MIB_SDR16: u64 = 36;
pub const MIB_SDR17: u64 = 37;
pub const MIB_SDR18: u64 = 38;
pub const MIB_SDR19: u64 = 39;
pub const MIB_SDR20: u64 = 40;
pub const MIB_SDR21: u64 = 41;
pub const MIB_SDR22: u64 = 42;
pub const MIB_SDR23: u64 = 43;
pub const MIB_SDR24: u64 = 44;
pub const MIB_SDR25: u64 = 45;
pub const MIB_SDR27: u64 = 46;
pub const MIB_SDR28: u64 = 47;
pub const MIB_SDR29: u64 = 48;
pub const MIB_SDRVEC: u64 = 49;
pub const MIB_SDR31: u64 = 50;
pub const MIB_SDR32: u64 = 51;
pub const MIB_SDRMUBF: u64 = 52;
pub const MIB_DR8: u64 = 53;
pub const MIB_DR9: u64 = 54;
pub const MIB_DR11: u64 = 55;
pub const MIB_MB_SDR0: u64 = 56;
pub const MIB_MB_SDR1: u64 = 57;
pub const TX_AGG_CNT: u64 = 58;
pub const TX_AGG_CNT2: u64 = 59;
pub const MIB_ARNG: u64 = 60;
pub const WTBLON_TOP_WDUCR: u64 = 61;
pub const WTBL_UPDATE: u64 = 62;
pub const PLE_FL_Q_EMPTY: u64 = 63;
pub const PLE_FL_Q_CTRL: u64 = 64;
pub const PLE_AC_QEMPTY: u64 = 65;
pub const PLE_FREEPG_CNT: u64 = 66;
pub const PLE_FREEPG_HEAD_TAIL: u64 = 67;
pub const PLE_PG_HIF_GROUP: u64 = 68;
pub const PLE_HIF_PG_INFO: u64 = 69;
pub const AC_OFFSET: u64 = 70;
pub const ETBF_PAR_RPT0: u64 = 71;
pub const __MT_OFFS_MAX: u64 = 72;
