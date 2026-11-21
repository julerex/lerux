#![cfg_attr(not(test), no_std)]
//! Standalone mt76 layouts reconstructed from Linux v7.3.5.
//! The MIT Swift driver does not link this crate.

#![allow(non_camel_case_types)] // C record names keep their kernel spelling
#![allow(non_snake_case)] // C fields and function-like macros keep their spelling
#![allow(non_upper_case_globals)] // C macros keep their spelling
#![allow(missing_debug_implementations)] // wire records are copied, not formatted

mod bits;
mod endian;
pub use bits::{bit, div, genmask, rem, shl, shr};
pub use endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub mod dma;
pub mod mt7921;

pub use dma::{
    mt76_desc, mt76_rro_rxdmad_c, mt76_wed_rro_desc, EVT_CMD_DONE, EVT_CMD_ERROR, EVT_CMD_RETRY,
    EVT_EVENT_CARRIER_DETECT_RSP, EVT_EVENT_DFS_DETECT_RSP, EVT_EVENT_PWR_RSP, EVT_EVENT_WOW_RSP,
    MT_DMA_CTL_DMA_DONE, MT_DMA_CTL_DROP, MT_DMA_CTL_LAST_SEC0, MT_DMA_CTL_LAST_SEC1,
    MT_DMA_CTL_M_DONE, MT_DMA_CTL_PN_CHK_FAIL, MT_DMA_CTL_SDP0_H, MT_DMA_CTL_SDP1_H,
    MT_DMA_CTL_SD_LEN0, MT_DMA_CTL_SD_LEN1, MT_DMA_CTL_TOKEN, MT_DMA_CTL_TO_HOST,
    MT_DMA_CTL_TO_HOST_A, MT_DMA_CTL_VER_MASK, MT_DMA_CTL_WO_DROP, MT_DMA_HDR_LEN,
    MT_DMA_INFO_DMA_FRAG, MT_DMA_INFO_PPE_VLD, MT_DMA_MAGIC_CNT, MT_DMA_MAGIC_MASK,
    MT_DMA_PPE_CPU_REASON, MT_DMA_PPE_ENTRY, MT_DMA_RRO_EN, MT_DMA_SDP0, MT_DMA_TOKEN_ID,
    MT_DMA_WED_IND_CMD_CNT, MT_DMA_WED_IND_REASON, MT_DMA_WED_IND_REASON_NORMAL,
    MT_DMA_WED_IND_REASON_OLDPKT, MT_DMA_WED_IND_REASON_REPEAT, MT_FCE_INFO_LEN, MT_QSEL_EDCA,
    MT_QSEL_EDCA_2, MT_QSEL_HCCA, MT_QSEL_MGMT, MT_RING_SIZE, MT_RX_INFO_LEN, MT_RX_RXWI_LEN,
    RRO_RXDMAD_DATA1_LS_MASK, RRO_RXDMAD_DATA1_SDL0_MASK, RRO_RXDMAD_DATA2_IND_REASON_MASK,
    RRO_RXDMAD_DATA2_RX_TOKEN_ID_MASK, RRO_RXDMAD_DATA3_MAGIC_CNT_MASK,
};
pub use mt7921::mcu::{
    mt7921_clc_info_tlv, mt7921_mcu_ant_id_config, mt7921_mcu_tx_done_event, mt7921_rftest_cmd,
    mt7921_rftest_evt, mt7921_txpwr_req, mt7921_wf_rf_pin_ctrl_event, MCU_EXT_EVENT_RATE_REPORT,
    MT7921_TM_ICAP, MT7921_TM_ICAP_OVERLAP, MT7921_TM_NORMAL, MT7921_TM_TESTMODE,
    MT7921_TM_WIFISPECTRUM, MT_EBF, MT_IBF, MT_RA_RATE_BW, MT_RA_RATE_DCM_EN, MT_RA_RATE_MCS,
    MT_RA_RATE_NSS, MT_RA_RATE_TX_MODE, TM_QUERY_AT_CMD, TM_SET_AT_CMD, TM_SWITCH_MODE,
};

#[cfg(test)]
mod prove;
