//! `mt7996/mac.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_dfs_pulse {
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
pub struct mt7996_dfs_pattern {
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
