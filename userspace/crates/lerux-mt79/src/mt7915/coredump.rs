//! `mt7915/coredump.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct trace {
    pub id: u32,
    pub timestamp: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7915_coredump_mem {
    pub len: u32,
}
pub const mt7915_coredump_mem_FLEX: usize = core::mem::size_of::<u8>();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_mem_hdr {
    pub start: u32,
    pub len: u32,
}
pub const mt7915_mem_hdr_FLEX: usize = core::mem::size_of::<u8>();
