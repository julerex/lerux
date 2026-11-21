//! `mt7921/mcu.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT_RA_RATE_NSS: u64 = 448;
pub const MT_RA_RATE_MCS: u64 = 15;
pub const MT_RA_RATE_TX_MODE: u64 = 0x1e00;
pub const MT_RA_RATE_DCM_EN: u64 = 16;
pub const MT_RA_RATE_BW: u64 = 0x6000;
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct mt7921_mcu_tx_done_event {
    pub pid: u8,
    pub status: u8,
    pub seq: Le16,
    pub wlan_idx: u8,
    pub tx_cnt: u8,
    pub tx_rate: Le16,
    pub flag: u8,
    pub tid: u8,
    pub rsp_rate: u8,
    pub mcs: u8,
    pub bw: u8,
    pub tx_pwr: u8,
    pub reason: u8,
    pub rsv0: [u8; 1],
    pub delay: Le32,
    pub timestamp: Le32,
    pub applied_flag: Le32,
    pub txs: [u8; 28],
    pub rsv1: [u8; 32],
}

pub const MCU_EXT_EVENT_RATE_REPORT: u64 = 135;
pub const MT_EBF: u64 = 1;
pub const MT_IBF: u64 = 2;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct mt7921_mcu_ant_id_config {
    pub ant_id: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct mt7921_txpwr_req {
    pub ver: u8,
    pub action: u8,
    pub len: Le16,
    pub dbdc_idx: u8,
    pub rsv: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct mt7921_wf_rf_pin_ctrl_event {
    pub result: u8,
    pub value: u8,
}

pub const TM_SWITCH_MODE: u64 = 0;
pub const TM_SET_AT_CMD: u64 = 1;
pub const TM_QUERY_AT_CMD: u64 = 2;
pub const MT7921_TM_NORMAL: u64 = 0;
pub const MT7921_TM_TESTMODE: u64 = 1;
pub const MT7921_TM_ICAP: u64 = 2;
pub const MT7921_TM_ICAP_OVERLAP: u64 = 3;
pub const MT7921_TM_WIFISPECTRUM: u64 = 4;
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct mt7921_rftest_cmd {
    pub action: u8,
    pub rsv: [u8; 3],
    pub param0: Le32,
    pub param1: Le32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct mt7921_rftest_evt {
    pub param0: Le32,
    pub param1: Le32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct mt7921_clc_info_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub chan_conf: u8,
    pub rsv: [u8; 63],
}
