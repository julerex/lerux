//! `mt792x_acpi_sar.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT792x_ASAR_MIN_DYN: u64 = 1;
pub const MT792x_ASAR_MAX_DYN: u64 = 8;
pub const MT792x_ASAR_MIN_GEO: u64 = 3;
pub const MT792x_ASAR_MAX_GEO: u64 = 8;
pub const MT792x_ASAR_MIN_FG: u64 = 8;
pub const MT792x_ACPI_MTCL: &[u8] = b"MTCL";
pub const MT792x_ACPI_MTDS: &[u8] = b"MTDS";
pub const MT792x_ACPI_MTGS: &[u8] = b"MTGS";
pub const MT792x_ACPI_MTFG: &[u8] = b"MTFG";
pub const MT792X_ACPI_MTCL_INVALID: u64 = 0xffff_ffff;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_asar_dyn_limit {
    pub idx: u8,
    pub frp: [u8; 5],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_asar_dyn_limit_v2 {
    pub idx: u8,
    pub frp: [u8; 11],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_asar_geo_band {
    pub pwr: u8,
    pub offset: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_asar_geo_limit {
    pub idx: u8,
    pub band: [mt792x_asar_geo_band; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_asar_geo_limit_v2 {
    pub idx: u8,
    pub band: [mt792x_asar_geo_band; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_asar_cl_v3 {
    pub names: [u8; 4],
    pub version: u8,
    pub mode_6g: u8,
    pub cl6g: [u8; 6],
    pub mode_5g9: u8,
    pub cl5g9: [u8; 6],
    pub mode_be: u8,
    pub clbe: [u8; 6],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_asar_cl {
    pub names: [u8; 4],
    pub version: u8,
    pub mode_6g: u8,
    pub cl6g: [u8; 6],
    pub mode_5g9: u8,
    pub cl5g9: [u8; 6],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_asar_fg {
    pub names: [u8; 4],
    pub version: u8,
    pub rsvd: u8,
    pub nr_flag: u8,
    pub rsvd1: u8,
}
pub const mt792x_asar_fg_FLEX: usize = core::mem::size_of::<u8>();
