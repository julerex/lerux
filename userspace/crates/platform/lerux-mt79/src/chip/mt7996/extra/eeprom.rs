//! `mt7996/eeprom.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT_EE_WIFI_CONF0_TX_PATH: u64 = 7;
pub const MT_EE_WIFI_CONF0_BAND_SEL: u64 = 7;
pub const MT_EE_WIFI_CONF1_BAND_SEL: u64 = 56;
pub const MT_EE_WIFI_CONF2_BAND_SEL: u64 = 7;
pub const MT_EE_WIFI_CONF1_TX_PATH_BAND0: u64 = 56;
pub const MT_EE_WIFI_CONF2_TX_PATH_BAND1: u64 = 7;
pub const MT_EE_WIFI_CONF2_TX_PATH_BAND2: u64 = 56;
pub const MT_EE_WIFI_CONF3_RX_PATH_BAND0: u64 = 7;
pub const MT_EE_WIFI_CONF3_RX_PATH_BAND1: u64 = 56;
pub const MT_EE_WIFI_CONF4_RX_PATH_BAND2: u64 = 7;
pub const MT_EE_WIFI_CONF4_STREAM_NUM_BAND0: u64 = 56;
pub const MT_EE_WIFI_CONF5_STREAM_NUM_BAND1: u64 = 7;
pub const MT_EE_WIFI_CONF5_STREAM_NUM_BAND2: u64 = 56;
pub const MT_EE_WIFI_PA_LNA_CONFIG: u64 = 3;
pub const MT_EE_RATE_DELTA_MASK: u64 = 63;
pub const MT_EE_RATE_DELTA_SIGN: u64 = 64;
pub const MT_EE_RATE_DELTA_EN: u64 = 128;
pub const MT_EE_CHIP_ID: u64 = 0;
pub const MT_EE_VERSION: u64 = 2;
pub const MT_EE_MAC_ADDR: u64 = 4;
pub const MT_EE_MAC_ADDR2: u64 = 10;
pub const MT_EE_WIFI_CONF: u64 = 400;
pub const MT_EE_MAC_ADDR3: u64 = 704;
pub const MT_EE_RATE_DELTA_2G: u64 = 0x1400;
pub const MT_EE_RATE_DELTA_5G: u64 = 0x147d;
pub const MT_EE_RATE_DELTA_6G: u64 = 0x154a;
pub const MT_EE_TX0_POWER_2G: u64 = 0x1300;
pub const MT_EE_TX0_POWER_5G: u64 = 0x1301;
pub const MT_EE_TX0_POWER_6G: u64 = 0x1310;
pub const __MT_EE_MAX: u64 = 0x1dff;
pub const MT_EE_BAND_SEL_DEFAULT: u64 = 0;
pub const MT_EE_BAND_SEL_2GHZ: u64 = 1;
pub const MT_EE_BAND_SEL_5GHZ: u64 = 2;
pub const MT_EE_BAND_SEL_6GHZ: u64 = 3;
