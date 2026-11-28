//! `mt7996/regs.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT_RRO_TOP_BASE: u64 = 0xa000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_RRO_TOP(ofs: u64) -> u64 { (MT_RRO_TOP_BASE).wrapping_add(ofs) }

pub const MT_RRO_BA_BITMAP_BASE0: u64 = 0xa008;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct __map {
    pub phys: u32,
    pub mapped: u32,
    pub size: u32,
}

pub const MT_RRO_BA_BITMAP_BASE1: u64 = 0xa00c;
pub const WF_RRO_AXI_MST_CFG: u64 = 0xa0b8;
pub const WF_RRO_AXI_MST_CFG_DIDX_OK: u64 = 0x1000;
pub const MT_RRO_ADDR_ARRAY_BASE0: u64 = 0xa030;
pub const MT_RRO_ADDR_ARRAY_BASE1: u64 = 0xa034;
pub const MT_RRO_ADDR_ARRAY_ELEM_ADDR_SEG_MODE: u64 = 0x8000_0000;
pub const MT_RRO_IND_CMD_SIGNATURE_BASE0: u64 = 0xa038;
pub const MT_RRO_IND_CMD_SIGNATURE_BASE1: u64 = 0xa03c;
pub const MT_RRO_IND_CMD_0_CTRL0: u64 = 0xa040;
pub const MT_RRO_IND_CMD_SIGNATURE_BASE1_EN: u64 = 0x8000_0000;
pub const MT_RRO_PARTICULAR_CFG0: u64 = 0xa05c;
pub const MT_RRO_PARTICULAR_CFG1: u64 = 0xa060;
pub const MT_RRO_PARTICULAR_CONFG_EN: u64 = 0x8000_0000;
pub const MT_RRO_PARTICULAR_SID: u64 = 0x7fff_0000;
pub const MT_RRO_BA_BITMAP_BASE_EXT0: u64 = 0xa070;
pub const MT_RRO_BA_BITMAP_BASE_EXT1: u64 = 0xa074;
pub const MT_RRO_HOST_INT_ENA: u64 = 0xa204;
pub const MT_RRO_HOST_INT_ENA_HOST_RRO_DONE_ENA: u64 = 1;
pub const MT_RRO_ADDR_ELEM_SEG_ADDR0: u64 = 0xa400;
pub const MT_RRO_3_0_EMU_CONF: u64 = 0xa600;
pub const MT_RRO_3_0_EMU_CONF_EN_MASK: u64 = 0x800;
pub const MT_RRO_3_1_GLOBAL_CONFIG: u64 = 0xa604;
pub const MT_RRO_3_1_GLOBAL_CONFIG_RXDMAD_SEL: u64 = 64;
pub const MT_RRO_3_1_GLOBAL_CONFIG_RX_CIDX_RD_EN: u64 = 8;
pub const MT_RRO_3_1_GLOBAL_CONFIG_RX_DIDX_WR_EN: u64 = 4;
pub const MT_RRO_3_1_GLOBAL_CONFIG_INTERLEAVE_EN: u64 = 1;
pub const MT_RRO_MSDU_PG_SEG_ADDR0: u64 = 0xa620;
pub const MT_RRO_RX_RING_AP_CIDX_ADDR: u64 = 0xa6f0;
pub const MT_RRO_RX_RING_AP_DIDX_ADDR: u64 = 0xa6f4;
pub const MT_RRO_ACK_SN_CTRL: u64 = 0xa050;
pub const MT_RRO_ACK_SN_CTRL_SN_MASK: u64 = 0xfff_0000;
pub const MT_RRO_ACK_SN_CTRL_SESSION_MASK: u64 = 0xfff;
pub const MT_RRO_DBG_RD_CTRL: u64 = 0xa0e0;
pub const MT_RRO_DBG_RD_ADDR: u64 = 0xffff;
pub const MT_RRO_DBG_RD_EXEC: u64 = 0x8000_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_RRO_DBG_RDAT_DW(n: u64) -> u64 { MT_RRO_TOP((240u64).wrapping_add((n).wrapping_mul(4u64))) }

pub const MT_MCU_INT_EVENT: u64 = 0x2108;
pub const MT_MCU_INT_EVENT_DMA_STOPPED: u64 = 1;
pub const MT_MCU_INT_EVENT_DMA_INIT: u64 = 2;
pub const MT_MCU_INT_EVENT_RESET_DONE: u64 = 8;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct __base {
    pub band_base: [u32; 3],
}

pub const MT_PLE_BASE: u64 = 0x820c_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_PLE(ofs: u64) -> u64 { (MT_PLE_BASE).wrapping_add(ofs) }

pub const MT_FL_Q_EMPTY: u64 = 0x820c_0360;
pub const MT_FL_Q0_CTRL: u64 = 0x820c_03e0;
pub const MT_FL_Q2_CTRL: u64 = 0x820c_03e8;
pub const MT_FL_Q3_CTRL: u64 = 0x820c_03ec;
pub const MT_PLE_FREEPG_CNT: u64 = 0x820c_0380;
pub const MT_PLE_FREEPG_HEAD_TAIL: u64 = 0x820c_0384;
pub const MT_PLE_PG_HIF_GROUP: u64 = 0x820c_000c;
pub const MT_PLE_HIF_PG_INFO: u64 = 0x820c_0388;
#[must_use]
#[rustfmt::skip]
pub const fn MT_PLE_AC_QEMPTY(ac: u64, n: u64) -> u64 { MT_PLE(((0x600u64).wrapping_add((128u64).wrapping_mul(ac))).wrapping_add(shl(n, 2u64))) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_PLE_AMSDU_PACK_MSDU_CNT(n: u64) -> u64 { MT_PLE((0x10e0u64).wrapping_add(shl(n, 2u64))) }

pub const MT_MDP_BASE: u64 = 0x820c_c000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MDP(ofs: u64) -> u64 { (MT_MDP_BASE).wrapping_add(ofs) }

pub const MT_MDP_DCR0: u64 = 0x820c_c800;
pub const MT_MDP_DCR0_RX_HDR_TRANS_EN: u64 = 0x8_0000;
pub const MT_MDP_DCR2: u64 = 0x820c_c8e8;
pub const MT_MDP_DCR2_RX_TRANS_SHORT: u64 = 4;
pub const MT_TMAC_TCR0_TX_BLINK: u64 = 192;
pub const MT_TIMEOUT_VAL_PLCP: u64 = 0xffff;
pub const MT_TIMEOUT_VAL_CCA: u64 = 0xffff_0000;
pub const MT_IFS_EIFS_OFDM: u64 = 511;
pub const MT_IFS_RIFS: u64 = 0x7c00;
pub const MT_IFS_SIFS: u64 = 0x7f_0000;
pub const MT_IFS_SLOT: u64 = 0x7f00_0000;
pub const MT_IFS_EIFS_CCK: u64 = 511;
pub const MT_DMA_DCR0_RXD_G5_EN: u64 = 0x80_0000;
pub const MT_DMA_TCRF1_QIDX: u64 = 0xe000;
pub const MT_WTBLOFF_RSCR_RCPI_MODE: u64 = 0xc000_0000;
pub const MT_WTBLOFF_RSCR_RCPI_PARAM: u64 = 0x300_0000;
pub const MT_WTBLOFF_ADM_BACKOFFTIME: u64 = 0x2000_0000;
pub const MT_ETBF_RX_FB_BW: u64 = 0x700;
pub const MT_ETBF_RX_FB_NC: u64 = 240;
pub const MT_ETBF_RX_FB_NR: u64 = 15;
pub const MT_LPON_TCR_SW_MODE: u64 = 3;
pub const MT_LPON_TCR_SW_WRITE: u64 = 1;
pub const MT_LPON_TCR_SW_ADJUST: u64 = 2;
pub const MT_LPON_TCR_SW_READ: u64 = 3;
pub const MT_MIB_SDR6_CHANNEL_IDL_CNT_MASK: u64 = 0xffff;
pub const MT_MIB_SDR27_TX_RWP_FAIL_CNT: u64 = 0xffff;
pub const MT_MIB_SDR28_TX_RWP_NEED_CNT: u64 = 0xffff;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MIB_ARNCR_RANGE(val: u64, n: u64) -> u64 { (shr(val, shl(n, 4u64))) & (genmask(9u64, 0u64)) }

pub const MT_WF_UMIB_BASE: u64 = 0x820c_d000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_UMIB(ofs: u64) -> u64 { (MT_WF_UMIB_BASE).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_UMIB_RPDCR(band: u64) -> u64 { (MT_WF_UMIB(0x594u64)).wrapping_add((band).wrapping_mul(356u64)) }

pub const MT_WTBLON_TOP_BASE: u64 = 0x820d_4000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WTBLON_TOP(ofs: u64) -> u64 { (MT_WTBLON_TOP_BASE).wrapping_add(ofs) }

pub const MT_WTBLON_TOP_WDUCR_GROUP: u64 = 31;
pub const MT_WTBL_UPDATE_WLAN_IDX: u64 = 0xfff;
pub const MT_WTBL_UPDATE_ADM_COUNT_CLEAR: u64 = 0x4000;
pub const MT_WTBL_UPDATE_BUSY: u64 = 0x8000_0000;
pub const MT_WTBL_ITCR_WR: u64 = 0x1_0000;
pub const MT_WTBL_ITCR_EXEC: u64 = 0x8000_0000;
pub const MT_WTBL_SPE_IDX_SEL: u64 = 64;
pub const MT_WTBL_BASE: u64 = 0x820d_8000;
pub const MT_WTBL_LMAC_ID: u64 = 0x7f00;
pub const MT_WTBL_LMAC_DW: u64 = 252;
pub const MT_AGG_ACR_PPDU_TXS2H: u64 = 2;
pub const MT_ARB_SCR_TX_DISABLE: u64 = 256;
pub const MT_ARB_SCR_RX_DISABLE: u64 = 512;
pub const MT_WF_RFCR_DROP_STBC_MULTI: u64 = 1;
pub const MT_WF_RFCR_DROP_FCSFAIL: u64 = 2;
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
pub const WF_AGG_BASE: u64 = 0;
pub const MT_WF_RFCR_DROP_DUPLICATE: u64 = 0x1_0000;
pub const MT_WF_RFCR_DROP_OTHER_BSS: u64 = 0x2_0000;
pub const MT_WF_RFCR_DROP_OTHER_UC: u64 = 0x4_0000;
pub const MT_WF_RFCR_DROP_OTHER_TIM: u64 = 0x8_0000;
pub const MT_WF_RFCR_DROP_NDPA: u64 = 0x10_0000;
pub const MT_WF_RFCR_DROP_UNWANTED_CTL: u64 = 0x20_0000;
pub const MT_WF_RFCR1_DROP_ACK: u64 = 16;
pub const MT_WF_RFCR1_DROP_BF_POLL: u64 = 32;
pub const MT_WF_RFCR1_DROP_BA: u64 = 64;
pub const MT_WF_RFCR1_DROP_CFEND: u64 = 128;
pub const MT_WF_RFCR1_DROP_CFACK: u64 = 256;
pub const WF_ARB_BASE: u64 = 1;
pub const MT_WF_RMAC_MIB_RXTIME_CLR: u64 = 0x8000_0000;
pub const MT_WF_RMAC_MIB_ED_OFFSET: u64 = 0x1f_0000;
pub const MT_WF_RMAC_MIB_OBSS_BACKOFF: u64 = 0xffff;
pub const MT_WF_RMAC_MIB_NONQOSD_BACKOFF: u64 = 0xffff_0000;
pub const MT_WF_RMAC_MIB_QOS01_BACKOFF: u64 = 0xffff_ffff;
pub const MT_WF_RMAC_MIB_QOS23_BACKOFF: u64 = 0xffff_ffff;
pub const WF_TMAC_BASE: u64 = 2;
pub const MT_WF_RMAC_RSVD0_EIFS_CLR: u64 = 0x20_0000;
pub const MT_RATE_HRCR0_CFEND_RATE: u64 = 0x7fff;
pub const MT_WFDMA0_BASE: u64 = 0xd_4000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WFDMA0(ofs: u64) -> u64 { (MT_WFDMA0_BASE).wrapping_add(ofs) }

pub const MT_WFDMA0_RST: u64 = 0xd_4100;
pub const WF_RMAC_BASE: u64 = 3;
pub const MT_WFDMA0_RST_LOGIC_RST: u64 = 16;
pub const MT_WFDMA0_RST_DMASHDL_ALL_RST: u64 = 32;
pub const MT_WFDMA0_BUSY_ENA: u64 = 0xd_413c;
pub const MT_WFDMA0_BUSY_ENA_TX_FIFO0: u64 = 1;
pub const MT_WFDMA0_BUSY_ENA_TX_FIFO1: u64 = 2;
pub const MT_WFDMA0_BUSY_ENA_RX_FIFO: u64 = 4;
pub const MT_WFDMA0_RX_INT_PCIE_SEL: u64 = 0xd_4154;
pub const MT_WFDMA0_RX_INT_SEL_RING3: u64 = 8;
pub const MT_WFDMA0_RX_INT_SEL_RING5: u64 = 32;
pub const MT_WFDMA0_RX_INT_SEL_RING6: u64 = 64;
pub const MT_WFDMA0_RX_INT_SEL_RING9: u64 = 512;
pub const MT_WFDMA0_MCU_HOST_INT_ENA: u64 = 0xd_41f4;
pub const WF_DMA_BASE: u64 = 4;
pub const MT_WFDMA0_GLO_CFG: u64 = 0xd_4208;
pub const MT_WFDMA0_GLO_CFG_TX_DMA_EN: u64 = 1;
pub const MT_WFDMA0_GLO_CFG_RX_DMA_EN: u64 = 4;
pub const MT_WFDMA0_GLO_CFG_OMIT_RX_INFO_PFET2: u64 = 0x20_0000;
pub const MT_WFDMA0_GLO_CFG_EXT_EN: u64 = 0x400_0000;
pub const MT_WFDMA0_GLO_CFG_OMIT_RX_INFO: u64 = 0x800_0000;
pub const MT_WFDMA0_GLO_CFG_OMIT_TX_INFO: u64 = 0x1000_0000;
pub const MT_WFDMA0_PAUSE_RX_Q_45_TH: u64 = 0xd_4268;
pub const MT_WFDMA0_PAUSE_RX_Q_67_TH: u64 = 0xd_426c;
pub const MT_WFDMA0_PAUSE_RX_Q_89_TH: u64 = 0xd_4270;
pub const MT_WFDMA0_PAUSE_RX_Q_RRO_TH: u64 = 0xd_427c;
pub const WF_WTBLOFF_BASE: u64 = 5;
pub const WF_WFDMA0_GLO_CFG_EXT0: u64 = 0xd_42b0;
pub const WF_WFDMA0_GLO_CFG_EXT0_OUTSTAND_MASK: u64 = 0xf00_0000;
pub const WF_WFDMA0_GLO_CFG_EXT0_RX_WB_RXD: u64 = 0x4_0000;
pub const WF_WFDMA0_GLO_CFG_EXT0_WED_MERGE_MODE: u64 = 0x4000;
pub const WF_WFDMA0_GLO_CFG_EXT1: u64 = 0xd_42b4;
pub const WF_WFDMA0_GLO_CFG_EXT1_CALC_MODE: u64 = 0x8000_0000;
pub const WF_WFDMA0_GLO_CFG_EXT1_TX_FCTRL_MODE: u64 = 0x1000_0000;
pub const MT_WFDMA0_RST_DTX_PTR: u64 = 0xd_420c;
pub const MT_WFDMA0_PRI_DLY_INT_CFG0: u64 = 0xd_42f0;
pub const MT_WFDMA0_PRI_DLY_INT_CFG1: u64 = 0xd_42f4;
pub const MT_WFDMA0_PRI_DLY_INT_CFG2: u64 = 0xd_42f8;
pub const MT_WFDMA1_BASE: u64 = 0xd_5000;
pub const WF_ETBF_BASE: u64 = 6;
pub const MT_WFDMA_EXT_CSR_BASE: u64 = 0xd_7000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WFDMA_EXT_CSR(ofs: u64) -> u64 { (MT_WFDMA_EXT_CSR_BASE).wrapping_add(ofs) }

pub const MT_WFDMA_HOST_CONFIG: u64 = 0xd_7030;
pub const MT_WFDMA_HOST_CONFIG_PDMA_BAND: u64 = 1;
pub const MT_WFDMA_HOST_CONFIG_BAND0_PCIE1: u64 = 0x10_0000;
pub const MT_WFDMA_HOST_CONFIG_BAND1_PCIE1: u64 = 0x20_0000;
pub const MT_WFDMA_HOST_CONFIG_BAND2_PCIE1: u64 = 0x40_0000;
pub const MT_WFDMA_EXT_CSR_HIF_MISC: u64 = 0xd_7044;
pub const MT_WFDMA_EXT_CSR_HIF_MISC_BUSY: u64 = 1;
pub const MT_WFDMA_AXI_R2A_CTRL: u64 = 0xd_7500;
pub const MT_WFDMA_AXI_R2A_CTRL_OUTSTAND_MASK: u64 = 31;
pub const WF_LPON_BASE: u64 = 7;
pub const MT_WFDMA_AXI_R2A_CTRL2: u64 = 0xd_7508;
pub const MT_WFDMA_AXI_R2A_CTRL2_OUTSTAND_MASK: u64 = 0xf000_0000;
pub const MT_PCIE_RECOG_ID: u64 = 0xd_7090;
pub const MT_PCIE_RECOG_ID_MASK: u64 = 0x7fff_ffff;
pub const MT_PCIE_RECOG_ID_SEM: u64 = 0x8000_0000;
pub const MT_WFDMA0_PCIE1_BASE: u64 = 0xd_8000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WFDMA0_PCIE1(ofs: u64) -> u64 { (MT_WFDMA0_PCIE1_BASE).wrapping_add(ofs) }

pub const MT_INT_PCIE1_SOURCE_CSR_EXT: u64 = 0xd_8118;
pub const MT_INT_PCIE1_MASK_CSR: u64 = 0xd_811c;
pub const MT_WFDMA0_PCIE1_BUSY_ENA: u64 = 0xd_813c;
pub const WF_MIB_BASE: u64 = 8;
pub const MT_WFDMA0_PCIE1_BUSY_ENA_TX_FIFO0: u64 = 1;
pub const MT_WFDMA0_PCIE1_BUSY_ENA_TX_FIFO1: u64 = 2;
pub const MT_WFDMA0_PCIE1_BUSY_ENA_RX_FIFO: u64 = 4;
pub const WF_RATE_BASE: u64 = 9;
pub const MT_RXQ_RRO_IND_RING_BASE: u64 = 0xa040;
pub const MT_RXQ_RRO_AP_RING_BASE: u64 = 0xa650;
pub const MT_INT_SOURCE_CSR: u64 = 0xd_4200;
pub const MT_INT_MASK_CSR: u64 = 0xd_4204;
pub const __MT_REG_BASE_MAX: u64 = 10;
pub const MT_INT1_SOURCE_CSR: u64 = 0xd_8200;
pub const MT_INT1_MASK_CSR: u64 = 0xd_8204;
pub const MT_INT_RX_DONE_BAND0: u64 = 0x1000;
pub const MT_INT_RX_DONE_BAND1: u64 = 0x2000;
pub const MT_INT_RX_DONE_BAND2: u64 = 0x2000;
pub const MT_INT_RX_DONE_WM: u64 = 1;
pub const MT_INT_RX_DONE_WA: u64 = 2;
pub const MT_INT_RX_DONE_WA_MAIN: u64 = 4;
pub const MT_INT_RX_DONE_WA_EXT: u64 = 8;
pub const MT_INT_RX_DONE_WA_TRI: u64 = 8;
pub const MT_INT_RX_TXFREE_MAIN: u64 = 0x2_0000;
pub const MT_INT_RX_TXFREE_BAND1: u64 = 0x8000;
pub const MT_INT_RX_TXFREE_TRI: u64 = 0x8000;
pub const MT_INT_RX_TXFREE_BAND1_EXT: u64 = 0x8_0000;
pub const MT_INT_RX_TXFREE_BAND0_MT7990: u64 = 0x4000;
pub const MT_INT_RX_TXFREE_BAND1_MT7990: u64 = 0x8000;
pub const MT_INT_RX_DONE_BAND2_EXT: u64 = 0x80_0000;
pub const MT_INT_RX_TXFREE_EXT: u64 = 0x400_0000;
pub const MT_INT_MCU_CMD: u64 = 0x2000_0000;
pub const MT_INT_RX_DONE_RRO_BAND0: u64 = 0x1_0000;
pub const MT_INT_RX_DONE_RRO_BAND1: u64 = 0x2_0000;
pub const MT_INT_RX_DONE_RRO_BAND2: u64 = 0x4000;
pub const MT_INT_RX_DONE_RRO_IND: u64 = 0x800;
pub const MT_INT_RX_DONE_RRO_RXDMAD_C: u64 = 0x800;
pub const MT_INT_RX_DONE_MSDU_PG_BAND0: u64 = 0x4_0000;
pub const MT_INT_RX_DONE_MSDU_PG_BAND1: u64 = 0x8_0000;
pub const MT_INT_RX_DONE_MSDU_PG_BAND2: u64 = 0x80_0000;
pub const MIB_RVSR0: u64 = 0;
pub const MT_INT_TX_DONE_FWDL: u64 = 0x400_0000;
pub const MT_INT_TX_DONE_MCU_WM: u64 = 0x800_0000;
pub const MT_INT_TX_DONE_MCU_WA: u64 = 0x40_0000;
pub const MT_INT_TX_DONE_BAND0: u64 = 0x4000_0000;
pub const MT_INT_TX_DONE_BAND1: u64 = 0x8000_0000;
pub const MT_INT_TX_DONE_BAND2: u64 = 0x8000;
pub const MT_INT_TX_RX_DONE_EXT: u64 = 0x480_8000;
pub const MIB_RVSR1: u64 = 1;
pub const MT_MCU_CMD: u64 = 0xd_41f0;
pub const MT_MCU_CMD_STOP_DMA: u64 = 4;
pub const MT_MCU_CMD_RESET_DONE: u64 = 8;
pub const MT_MCU_CMD_RECOVERY_DONE: u64 = 16;
pub const MT_MCU_CMD_NORMAL_STATE: u64 = 32;
pub const MT_MCU_CMD_ERROR_MASK: u64 = 62;
pub const MT_MCU_CMD_WA_WDT: u64 = 0x8000_0000;
pub const MT_MCU_CMD_WM_WDT: u64 = 0x4000_0000;
pub const MIB_BTSCR5: u64 = 2;
pub const MT_MCU_CMD_WDT_MASK: u64 = 0xc000_0000;
pub const CONN_BUS_CR_VON_BASE: u64 = 0x15_5000;
pub const MT_HIF_REMAP_L1_MASK_7996: u64 = 0xffff_0000;
pub const MT_HIF_REMAP_L1_MASK: u64 = 0xffff;
pub const MT_HIF_REMAP_L1_OFFSET: u64 = 0xffff;
pub const MT_HIF_REMAP_L1_BASE: u64 = 0xffff_0000;
pub const MT_HIF_REMAP_L2_MASK: u64 = 0xf_ffff;
pub const MIB_BTSCR6: u64 = 3;
pub const MT_HIF_REMAP_L2_OFFSET: u64 = 0xfff;
pub const MT_HIF_REMAP_L2_BASE: u64 = 0xffff_f000;
pub const MT_HIF_REMAP_L2_MASK_7990: u64 = 0xffff;
pub const MT_HIF_REMAP_L2_OFFSET_7990: u64 = 0xffff;
pub const MT_HIF_REMAP_L2_BASE_7990: u64 = 0xffff_0000;
pub const MT_HIF_REMAP_CBTOP: u64 = 0x1f_6554;
pub const MT_HIF_REMAP_CBTOP_MASK: u64 = 0xffff;
pub const MT_HIF_REMAP_CBTOP_OFFSET: u64 = 0xffff;
pub const MT_HIF_REMAP_CBTOP_BASE: u64 = 0xffff_0000;
pub const MT_HIF_REMAP_BASE_CBTOP: u64 = 0x1c_0000;
pub const MIB_RSCR1: u64 = 4;
pub const MT_INFRA_BASE: u64 = 0x1800_0000;
pub const MT_WFSYS0_PHY_START: u64 = 0x1840_0000;
pub const MT_WFSYS1_PHY_START: u64 = 0x1880_0000;
pub const MT_WFSYS1_PHY_END: u64 = 0x18bf_ffff;
pub const MT_CBTOP1_PHY_START: u64 = 0x7000_0000;
pub const MT_CBTOP2_PHY_START: u64 = 0xf000_0000;
pub const MT_INFRA_MCU_START: u64 = 0x7c00_0000;
pub const MIB_RSCR27: u64 = 5;
pub const MT_FW_ASSERT_CNT: u64 = 0x220_8274;
pub const MT_FW_DUMP_STATE: u64 = 0x220_9e90;
pub const MT_SWDEF_BASE: u64 = 0x40_1400;
#[must_use]
#[rustfmt::skip]
pub const fn MT_SWDEF(ofs: u64) -> u64 { (MT_SWDEF_BASE).wrapping_add(ofs) }

pub const MT_SWDEF_MODE: u64 = 0x40_143c;
pub const MT_SWDEF_NORMAL_MODE: u64 = 0;
pub const MT_SWDEF_SER_STATS: u64 = 0x40_1440;
pub const MT_SWDEF_PLE_STATS: u64 = 0x40_1444;
pub const MT_SWDEF_PLE1_STATS: u64 = 0x40_1448;
pub const MT_SWDEF_PLE_AMSDU_STATS: u64 = 0x40_144c;
pub const MIB_RSCR28: u64 = 6;
pub const MT_SWDEF_PSE_STATS: u64 = 0x40_1450;
pub const MT_SWDEF_PSE1_STATS: u64 = 0x40_1454;
pub const MT_SWDEF_LAMC_WISR6_BN0_STATS: u64 = 0x40_1458;
pub const MT_SWDEF_LAMC_WISR6_BN1_STATS: u64 = 0x40_145c;
pub const MT_SWDEF_LAMC_WISR6_BN2_STATS: u64 = 0x40_1460;
pub const MT_SWDEF_LAMC_WISR7_BN0_STATS: u64 = 0x40_1464;
pub const MT_SWDEF_LAMC_WISR7_BN1_STATS: u64 = 0x40_1468;
pub const MT_SWDEF_LAMC_WISR7_BN2_STATS: u64 = 0x40_146c;
pub const MT_LED_TOP_BASE: u64 = 0x1801_3000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_LED_PHYS(n: u64) -> u64 { (MT_LED_TOP_BASE).wrapping_add(n) }

pub const MIB_RSCR29: u64 = 7;
#[must_use]
#[rustfmt::skip]
pub const fn MT_LED_CTRL(n: u64) -> u64 { MT_LED_PHYS((0u64).wrapping_add((n).wrapping_mul(4u64))) }

pub const MT_LED_CTRL_KICK: u64 = 128;
pub const MT_LED_CTRL_BLINK_BAND_SEL: u64 = 16;
pub const MT_LED_CTRL_BLINK_MODE: u64 = 4;
pub const MT_LED_CTRL_POLARITY: u64 = 2;
#[must_use]
#[rustfmt::skip]
pub const fn MT_LED_TX_BLINK(n: u64) -> u64 { MT_LED_PHYS((16u64).wrapping_add((n).wrapping_mul(4u64))) }

pub const MT_LED_TX_BLINK_ON_MASK: u64 = 255;
pub const MT_LED_TX_BLINK_OFF_MASK: u64 = 0xff00;
#[must_use]
#[rustfmt::skip]
pub const fn MT_LED_EN(n: u64) -> u64 { MT_LED_PHYS((64u64).wrapping_add((n).wrapping_mul(4u64))) }

pub const MIB_RSCR30: u64 = 8;
pub const MT_CONN_DBG_CTL_BASE: u64 = 0x1802_3000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_CONN_DBG_CTL(ofs: u64) -> u64 { (MT_CONN_DBG_CTL_BASE).wrapping_add(ofs) }

pub const MT_CONN_DBG_CTL_OUT_SEL: u64 = 0x1802_3604;
pub const MT_CONN_DBG_CTL_PC_LOG_SEL: u64 = 0x1802_360c;
pub const MT_CONN_DBG_CTL_PC_LOG: u64 = 0x1802_3610;
pub const MT_LED_GPIO_MUX2: u64 = 0x7000_5058;
pub const MT_LED_GPIO_MUX3: u64 = 0x7000_505c;
pub const MT_LED_GPIO_SEL_MASK: u64 = 0xf00;
pub const MT_TOP_BASE: u64 = 0xe_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP(ofs: u64) -> u64 { (MT_TOP_BASE).wrapping_add(ofs) }

pub const MIB_RSCR31: u64 = 9;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP_LPCR_HOST_BAND(band: u64) -> u64 { MT_TOP((16u64).wrapping_add((band).wrapping_mul(16u64))) }

pub const MT_TOP_LPCR_HOST_FW_OWN: u64 = 1;
pub const MT_TOP_LPCR_HOST_DRV_OWN: u64 = 2;
pub const MT_TOP_LPCR_HOST_FW_OWN_STAT: u64 = 4;
#[must_use]
#[rustfmt::skip]
pub const fn MT_TOP_LPCR_HOST_BAND_IRQ_STAT(band: u64) -> u64 { MT_TOP((20u64).wrapping_add((band).wrapping_mul(16u64))) }

pub const MT_TOP_LPCR_HOST_BAND_STAT: u64 = 1;
pub const MT_TOP_MISC: u64 = 0xe_00f0;
pub const MT_TOP_MISC_FW_STATE: u64 = 7;
pub const MIB_RSCR33: u64 = 10;
#[must_use]
#[rustfmt::skip]
pub const fn MT_ADIE_CHIP_ID(idx: u64) -> u64 { (0xf00_002cu64).wrapping_add(shl(idx, 28u64)) }

pub const MT_ADIE_VERSION_MASK: u64 = 0xffff;
pub const MT_ADIE_CHIP_ID_MASK: u64 = 0xffff_0000;
pub const MT_PAD_GPIO: u64 = 0x7000_56f0;
pub const MT_PAD_GPIO_ADIE_COMB: u64 = 0x1_8000;
pub const MT_PAD_GPIO_2ADIE_TBTC: u64 = 0x8_0000;
pub const MT_PAD_GPIO_ADIE_COMB_7992: u64 = 0x3_0000;
pub const MT_PAD_GPIO_ADIE_SINGLE: u64 = 0x8000;
pub const MT_HW_REV: u64 = 0x7001_0204;
pub const MT_HW_REV1: u64 = 0x8a00;
pub const MIB_RSCR35: u64 = 11;
pub const MT_WF_L05_RST: u64 = 0x7002_8550;
pub const MT_WF_L05_RST_WF_RST_MASK: u64 = 31;
pub const MT_WF_SUBSYS_RST: u64 = 0x7002_8600;
pub const MT_WF_SUBSYS_RST_WHOLE_PATH_RST: u64 = 1;
pub const MT_WF_SUBSYS_RST_WHOLE_PATH_RST_REVERT: u64 = 32;
pub const MT_WF_SUBSYS_RST_BYPASS_WFDMA_SLP_PROT: u64 = 64;
pub const MT_WF_SUBSYS_RST_BYPASS_WFDMA2_SLP_PROT: u64 = 0x1_0000;
pub const MT_WF_SUBSYS_RST_WHOLE_PATH_RST_REVERT_CYCLE: u64 = 0xff00;
pub const MT_PCIE_MAC_BASE: u64 = 0x7403_0000;
pub const MIB_RSCR36: u64 = 12;
#[must_use]
#[rustfmt::skip]
pub const fn MT_PCIE_MAC(ofs: u64) -> u64 { (MT_PCIE_MAC_BASE).wrapping_add(ofs) }

pub const MT_PCIE_MAC_INT_ENABLE: u64 = 0x7403_0188;
pub const MT_PCIE1_MAC_BASE: u64 = 0x7409_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_PCIE1_MAC(ofs: u64) -> u64 { (MT_PCIE1_MAC_BASE).wrapping_add(ofs) }

pub const MT_PCIE1_MAC_INT_ENABLE: u64 = 0x7409_0188;
pub const MT_WF_PHYRX_CSD_BASE: u64 = 0x8300_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHYRX_CSD(band: u64, wf: u64, ofs: u64) -> u64 { (((MT_WF_PHYRX_CSD_BASE).wrapping_add(shl(band, 20u64))).wrapping_add(shl(wf, 16u64))).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHYRX_CSD_IRPI(band: u64, wf: u64) -> u64 { MT_WF_PHYRX_CSD(band, wf, 0x1000u64) }

pub const MIB_BSCR0: u64 = 13;
pub const MT_WF_PHYRX_BAND_BASE: u64 = 0x8308_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHYRX_BAND(band: u64, ofs: u64) -> u64 { ((MT_WF_PHYRX_BAND_BASE).wrapping_add(shl(band, 20u64))).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHYRX_BAND_GID_TAB_VLD0(band: u64) -> u64 { MT_WF_PHYRX_BAND(band, 0x1054u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHYRX_BAND_GID_TAB_VLD1(band: u64) -> u64 { MT_WF_PHYRX_BAND(band, 0x1058u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHYRX_BAND_GID_TAB_POS0(band: u64) -> u64 { MT_WF_PHYRX_BAND(band, 0x105cu64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHYRX_BAND_GID_TAB_POS1(band: u64) -> u64 { MT_WF_PHYRX_BAND(band, 0x1060u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHYRX_BAND_GID_TAB_POS2(band: u64) -> u64 { MT_WF_PHYRX_BAND(band, 0x1064u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHYRX_BAND_GID_TAB_POS3(band: u64) -> u64 { MT_WF_PHYRX_BAND(band, 0x1068u64) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHYRX_BAND_RX_CTRL1(band: u64) -> u64 { MT_WF_PHYRX_BAND(band, 0x2004u64) }

pub const MIB_BSCR1: u64 = 14;
pub const MT_WF_PHYRX_BAND_RX_CTRL1_IPI_EN: u64 = 7;
pub const MT_WF_PHYRX_BAND_RX_CTRL1_STSCNT_EN: u64 = 0xe00;
#[must_use]
#[rustfmt::skip]
pub const fn MT_WF_PHYRX_CSD_BAND_RXTD12(band: u64) -> u64 { MT_WF_PHYRX_BAND(band, 0x8230u64) }

pub const MT_WF_PHYRX_CSD_BAND_RXTD12_IRPI_SW_CLR_ONLY: u64 = 0x4_0000;
pub const MT_WF_PHYRX_CSD_BAND_RXTD12_IRPI_SW_CLR: u64 = 0x2000_0000;
pub const MT_MCU_WM_EXCP_BASE: u64 = 0x8905_0000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_MCU_WM_EXCP(ofs: u64) -> u64 { (MT_MCU_WM_EXCP_BASE).wrapping_add(ofs) }

pub const MT_MCU_WM_EXCP_PC_CTRL: u64 = 0x8905_0100;
pub const MIB_BSCR2: u64 = 15;
pub const MT_MCU_WM_EXCP_PC_LOG: u64 = 0x8905_0104;
pub const MT_MCU_WM_EXCP_LR_CTRL: u64 = 0x8905_0200;
pub const MT_MCU_WM_EXCP_LR_LOG: u64 = 0x8905_0204;
pub const MT_AFE_CTL_BASE: u64 = 0x1804_3000;
#[must_use]
#[rustfmt::skip]
pub const fn MT_AFE_CTL_BAND(band: u64, ofs: u64) -> u64 { ((MT_AFE_CTL_BASE).wrapping_add((band).wrapping_mul(0x1000u64))).wrapping_add(ofs) }

#[must_use]
#[rustfmt::skip]
pub const fn MT_AFE_CTL_BAND_PLL_03(band: u64) -> u64 { MT_AFE_CTL_BAND(band, 44u64) }

pub const MT_AFE_CTL_BAND_PLL_03_MSB_EN: u64 = 2;
pub const MIB_BSCR3: u64 = 16;
pub const MIB_BSCR4: u64 = 17;
pub const MIB_BSCR5: u64 = 18;
pub const MIB_BSCR6: u64 = 19;
pub const MIB_BSCR7: u64 = 20;
pub const MIB_BSCR17: u64 = 21;
pub const MIB_TRDR1: u64 = 22;
pub const MIB_TSCR0: u64 = 23;
pub const MIB_TSCR1: u64 = 24;
pub const MIB_TSCR2: u64 = 25;
pub const MIB_TSCR3: u64 = 26;
pub const MIB_TSCR4: u64 = 27;
pub const MIB_TSCR5: u64 = 28;
pub const MIB_TSCR6: u64 = 29;
pub const MIB_TSCR7: u64 = 30;
pub const HIF_REMAP_L1: u64 = 31;
pub const HIF_REMAP_BASE_L1: u64 = 32;
pub const HIF_REMAP_L2: u64 = 33;
pub const HIF_REMAP_BASE_L2: u64 = 34;
pub const CBTOP1_PHY_END: u64 = 35;
pub const INFRA_MCU_END: u64 = 36;
pub const WTBLON_WDUCR: u64 = 37;
pub const WTBL_UPDATE: u64 = 38;
pub const WTBL_ITCR: u64 = 39;
pub const WTBL_ITCR0: u64 = 40;
pub const WTBL_ITCR1: u64 = 41;
pub const __MT_OFFS_MAX: u64 = 42;
