//! `mt7925/mac.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT_WTBL_TXRX_CAP_RATE_OFFSET: u64 = 7;
pub const MT_WTBL_TXRX_RATE_G2_HE: u64 = 24;
pub const MT_WTBL_TXRX_RATE_G2: u64 = 12;
pub const MT_WTBL_AC0_CTT_OFFSET: u64 = 20;
