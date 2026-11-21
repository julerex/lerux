//! `mt7915/mac.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT_TX_FREE_VER: u64 = 0x7_0000;
pub const MT_TX_FREE_MSDU_CNT_V0: u64 = 127;
pub const MT_TX_FREE_COUNT: u64 = 0x1fff;
pub const MT_TX_FREE_COUNT_V3: u64 = 0xf00_0000;
pub const MT_TX_FREE_STAT: u64 = 0x6000;
pub const MT_TX_FREE_STAT_V3: u64 = 0x3000_0000;
pub const MT_TX_FREE_MPDU_HEADER: u64 = 0x8000;
pub const MT_TX_FREE_MPDU_HEADER_V3: u64 = 0x4000_0000;
pub const MT_TX_FREE_MSDU_ID_V3: u64 = 0x7fff;
pub const MT_TXS5_F0_FINAL_MPDU: u64 = 0x8000_0000;
pub const MT_TXS5_F0_QOS: u64 = 0x4000_0000;
pub const MT_TXS5_F0_TX_COUNT: u64 = 0x3e00_0000;
pub const MT_TXS5_F0_FRONT_TIME: u64 = 0x1ff_ffff;
pub const MT_TXS5_F1_MPDU_TX_COUNT: u64 = 0xff00_0000;
pub const MT_TXS5_F1_MPDU_TX_BYTES: u64 = 0xff_ffff;
pub const MT_TXS6_F0_NOISE_3: u64 = 0xff00_0000;
pub const MT_TXS6_F0_NOISE_2: u64 = 0xff_0000;
pub const MT_TXS6_F0_NOISE_1: u64 = 0xff00;
pub const MT_TXS6_F0_NOISE_0: u64 = 255;
pub const MT_TXS6_F1_MPDU_FAIL_COUNT: u64 = 0xff00_0000;
pub const MT_TXS6_F1_MPDU_FAIL_BYTES: u64 = 0xff_ffff;
pub const MT_TXS7_F0_RCPI_3: u64 = 0xff00_0000;
pub const MT_TXS7_F0_RCPI_2: u64 = 0xff_0000;
pub const MT_TXS7_F0_RCPI_1: u64 = 0xff00;
pub const MT_TXS7_F0_RCPI_0: u64 = 255;
pub const MT_TXS7_F1_MPDU_RETRY_COUNT: u64 = 0xff00_0000;
pub const MT_TXS7_F1_MPDU_RETRY_BYTES: u64 = 0xff_ffff;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_dfs_pulse {
    pub max_width: u32,
    pub max_pwr: i32,
    pub min_pwr: i32,
    pub min_stgr_pri: u32,
    pub max_stgr_pri: u32,
    pub min_cr_pri: u32,
    pub max_cr_pri: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7915_dfs_pattern {
    pub enb: u8,
    pub stgr: u8,
    pub min_crpn: u8,
    pub max_crpn: u8,
    pub min_crpr: u8,
    pub min_pw: u8,
    pub min_pri: u32,
    pub max_pri: u32,
    pub max_pw: u8,
    pub min_crbn: u8,
    pub max_crbn: u8,
    pub min_stgpn: u8,
    pub max_stgpn: u8,
    pub min_stgpr: u8,
    pub rsv: [u8; 2],
    pub min_stgpr_diff: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_dfs_radar_spec {
    pub pulse_th: mt7915_dfs_pulse,
    pub radar_pattern: [mt7915_dfs_pattern; 16],
}
