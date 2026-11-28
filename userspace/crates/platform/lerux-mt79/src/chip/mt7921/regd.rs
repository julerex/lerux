//! `mt7921/regd.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7921_regd_rule_header {
    pub alpha2: [u8; 2],
    pub dfs_region: u8,
    pub rsv: [u8; 13],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7921_regd_rule {
    pub start_freq: Le32,
    pub end_freq: Le32,
    pub max_bw: Le32,
    pub eirp: Le32,
    pub flags: Le32,
    pub rsv: [u8; 12],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7921_regd_cc {
    pub alpha2: [u8; 2],
    pub ver: u8,
    pub rsv: u8,
    pub n_reg_rules: Le32,
    pub sign_type: u8,
    pub rsv1: [u8; 7],
}
pub const mt7921_regd_cc_FLEX: usize = core::mem::size_of::<u8>();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7921_regd_rule_ev {
    pub tag: Le16,
    pub len: Le16,
    pub n_reg_rules: Le32,
    pub dfs_region: u8,
    pub rsv: [u8; 15],
}
pub const mt7921_regd_rule_ev_FLEX: usize = core::mem::size_of::<mt7921_regd_rule>();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7921_regd_query_req {
    pub ver: u8,
    pub sign_type: u8,
    pub rsv1: [u8; 2],
    pub size: Le32,
    pub alpha2: [u8; 2],
    pub rsv2: [u8; 2],
    pub n_reg_rules: Le32,
    pub rsv3: [u8; 64],
}
pub const mt7921_regd_query_req_FLEX: usize = core::mem::size_of::<u8>();
