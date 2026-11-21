//! `mt7915/testmode.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_tm_trx {
    pub r#type: u8,
    pub enable: u8,
    pub band: u8,
    pub rsv: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_tm_freq_offset {
    pub band: u8,
    pub pad_0: [u8; 3],
    pub freq_offset: Le32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_tm_slot_time {
    pub slot_time: u8,
    pub sifs: u8,
    pub rifs: u8,
    pub _rsv: u8,
    pub eifs: Le16,
    pub band: u8,
    pub _rsv1: [u8; 5],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_tm_clean_txq {
    pub sta_pause: u8,
    pub wcid: u8,
    pub band: u8,
    pub rsv: u8,
}

#[allow(missing_debug_implementations)] // union fields share one store
#[repr(C)]
#[derive(Clone, Copy)]
pub union mt7915_tm_cmd_param {
    pub data: Le32,
    pub trx: mt7915_tm_trx,
    pub freq: mt7915_tm_freq_offset,
    pub slot: mt7915_tm_slot_time,
    pub clean: mt7915_tm_clean_txq,
    pub test: [u8; 72],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7915_tm_cmd {
    pub testmode_en: u8,
    pub param_idx: u8,
    pub _rsv: [u8; 2],
    pub param: mt7915_tm_cmd_param,
}

pub const TM_MAC_TX: u64 = 1;
pub const TM_MAC_RX: u64 = 2;
pub const TM_MAC_TXRX: u64 = 3;
pub const TM_MAC_TXRX_RXV: u64 = 4;
pub const TM_MAC_RXV: u64 = 5;
pub const TM_MAC_RX_RXV: u64 = 6;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct tm_tx_cont {
    pub control_ch: u8,
    pub center_ch: u8,
    pub bw: u8,
    pub tx_ant: u8,
    pub rateval: Le16,
    pub band: u8,
    pub txfd_mode: u8,
}

#[allow(missing_debug_implementations)] // union fields share one store
#[repr(C)]
#[derive(Clone, Copy)]
pub union mt7915_tm_rf_test_op_rf_param {
    pub func_data: Le32,
    pub cal_dump: Le32,
    pub tx_cont: tm_tx_cont,
    pub _pad: [u8; 80],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_tm_rf_test_op_rf {
    pub func_idx: Le32,
    pub param: mt7915_tm_rf_test_op_rf_param,
}

#[allow(missing_debug_implementations)] // union fields share one store
#[repr(C)]
#[derive(Clone, Copy)]
pub union mt7915_tm_rf_test_op {
    pub op_mode: Le32,
    pub freq: Le32,
    pub rf: mt7915_tm_rf_test_op_rf,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7915_tm_rf_test {
    pub action: u8,
    pub icap_len: u8,
    pub _rsv: [u8; 2],
    pub op: mt7915_tm_rf_test_op,
}

pub const RF_OPER_NORMAL: u64 = 0;
pub const RF_OPER_RF_TEST: u64 = 1;
pub const RF_OPER_ICAP: u64 = 2;
pub const RF_OPER_ICAP_OVERLAP: u64 = 3;
pub const RF_OPER_WIFI_SPECTRUM: u64 = 4;
pub const TAM_ARB_OP_MODE_NORMAL: u64 = 1;
pub const TAM_ARB_OP_MODE_TEST: u64 = 2;
pub const TAM_ARB_OP_MODE_FORCE_SU: u64 = 5;
