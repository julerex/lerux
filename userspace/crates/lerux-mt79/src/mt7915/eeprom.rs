//! `mt7915/eeprom.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT_EE_WIFI_CAL_GROUP: u64 = 1;
pub const MT_EE_WIFI_CAL_DPD_2G: u64 = 4;
pub const MT_EE_WIFI_CAL_DPD_5G: u64 = 2;
pub const MT_EE_WIFI_CAL_DPD_6G: u64 = 8;
pub const MT_EE_WIFI_CAL_DPD: u64 = 14;
pub const MT_EE_CAL_UNIT: u64 = 0x400;
pub const MT_EE_CAL_GROUP_SIZE_7915: u64 = 0xc410;
pub const MT_EE_CAL_GROUP_SIZE_7916: u64 = 0xd810;
pub const MT_EE_CAL_GROUP_SIZE_7975: u64 = 0xd810;
pub const MT_EE_CAL_GROUP_SIZE_7976: u64 = 0x1_7810;
pub const MT_EE_CAL_GROUP_SIZE_7916_6G: u64 = 0x1_7810;
pub const MT_EE_CAL_GROUP_SIZE_7981: u64 = 0x2_4010;
pub const MT_EE_CAL_DPD_SIZE_V1: u64 = 0xd800;
pub const MT_EE_CAL_DPD_SIZE_V2: u64 = 0x4_b000;
pub const MT_EE_WIFI_CONF0_TX_PATH: u64 = 7;
pub const MT_EE_WIFI_CONF0_RX_PATH: u64 = 56;
pub const MT_EE_WIFI_CONF0_BAND_SEL: u64 = 192;
pub const MT_EE_WIFI_CONF1_BAND_SEL: u64 = 192;
pub const MT_EE_WIFI_CONF_STREAM_NUM: u64 = 224;
pub const MT_EE_WIFI_CONF3_TX_PATH_B0: u64 = 3;
pub const MT_EE_WIFI_CONF3_TX_PATH_B1: u64 = 48;
pub const MT_EE_WIFI_CONF7_TSSI0_2G: u64 = 1;
pub const MT_EE_WIFI_CONF7_TSSI0_5G: u64 = 4;
pub const MT_EE_WIFI_CONF7_TSSI1_5G: u64 = 16;
pub const MT_EE_RATE_DELTA_MASK: u64 = 63;
pub const MT_EE_RATE_DELTA_SIGN: u64 = 64;
pub const MT_EE_RATE_DELTA_EN: u64 = 128;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct cal_data {
    pub count: u8,
    pub pad_0: u8,
    pub offset: [u16; 60],
}

pub const MT_EE_CHIP_ID: u64 = 0;
pub const MT_EE_VERSION: u64 = 2;
pub const MT_EE_MAC_ADDR: u64 = 4;
pub const MT_EE_MAC_ADDR2: u64 = 10;
pub const MT_EE_DDIE_FT_VERSION: u64 = 80;
pub const MT_EE_DO_PRE_CAL: u64 = 98;
pub const MT_EE_WIFI_CONF: u64 = 400;
pub const MT_EE_DO_PRE_CAL_V2: u64 = 410;
pub const MT_EE_RATE_DELTA_2G: u64 = 594;
pub const MT_EE_RATE_DELTA_5G: u64 = 669;
pub const MT_EE_TX0_POWER_2G: u64 = 764;
pub const MT_EE_TX0_POWER_5G: u64 = 843;
pub const MT_EE_RATE_DELTA_2G_V2: u64 = 0x7d3;
pub const MT_EE_RATE_DELTA_5G_V2: u64 = 0x81e;
pub const MT_EE_RATE_DELTA_6G_V2: u64 = 0x884;
pub const MT_EE_TX0_POWER_2G_V2: u64 = 0x441;
pub const MT_EE_TX0_POWER_5G_V2: u64 = 0x445;
pub const MT_EE_TX0_POWER_6G_V2: u64 = 0x465;
pub const MT_EE_ADIE_FT_VERSION: u64 = 0x9a0;
pub const __MT_EE_MAX: u64 = 0xe00;
pub const __MT_EE_MAX_V2: u64 = 0x1000;
pub const MT_EE_PRECAL: u64 = 0xe10;
pub const MT_EE_PRECAL_V2: u64 = 0x1010;
pub const MT7976_ONE_ADIE_DBDC: u64 = 7;
pub const MT7975_ONE_ADIE: u64 = 8;
pub const MT7976_ONE_ADIE: u64 = 10;
pub const MT7975_DUAL_ADIE: u64 = 13;
pub const MT7976_DUAL_ADIE: u64 = 15;
pub const MT_EE_BAND_SEL_DEFAULT: u64 = 0;
pub const MT_EE_BAND_SEL_5GHZ: u64 = 1;
pub const MT_EE_BAND_SEL_2GHZ: u64 = 2;
pub const MT_EE_BAND_SEL_DUAL: u64 = 3;
pub const MT_EE_V2_BAND_SEL_2GHZ: u64 = 0;
pub const MT_EE_V2_BAND_SEL_5GHZ: u64 = 1;
pub const MT_EE_V2_BAND_SEL_6GHZ: u64 = 2;
pub const MT_EE_V2_BAND_SEL_5GHZ_6GHZ: u64 = 3;
pub const SKU_CCK: u64 = 0;
pub const SKU_OFDM: u64 = 1;
pub const SKU_HT_BW20: u64 = 2;
pub const SKU_HT_BW40: u64 = 3;
pub const SKU_VHT_BW20: u64 = 4;
pub const SKU_VHT_BW40: u64 = 5;
pub const SKU_VHT_BW80: u64 = 6;
pub const SKU_VHT_BW160: u64 = 7;
pub const SKU_HE_RU26: u64 = 8;
pub const SKU_HE_RU52: u64 = 9;
pub const SKU_HE_RU106: u64 = 10;
pub const SKU_HE_RU242: u64 = 11;
pub const SKU_HE_RU484: u64 = 12;
pub const SKU_HE_RU996: u64 = 13;
pub const SKU_HE_RU2x996: u64 = 14;
pub const MAX_SKU_RATE_GROUP_NUM: u64 = 15;
