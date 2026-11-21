//! `mt7996/mt7996.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT7996_MAX_RADIOS: u64 = 3;
pub const MT7996_MAX_INTERFACES: u64 = 19;
pub const MT7996_MAX_WMM_SETS: u64 = 4;
pub const MT7996_TX_RING_SIZE: u64 = 0x800;
pub const MT7996_TX_MCU_RING_SIZE: u64 = 256;
pub const MT7996_TX_FWDL_RING_SIZE: u64 = 128;
pub const MT7996_RX_RING_SIZE: u64 = 0x600;
pub const MT7996_RX_MCU_RING_SIZE: u64 = 512;
pub const MT7996_RX_MCU_RING_SIZE_WA: u64 = 0x400;
pub const MT7996_NPU_TX_RING_SIZE: u64 = 0x400;
pub const MT7996_NPU_RX_RING_SIZE: u64 = 0x400;
pub const MT7996_NPU_TXD_SIZE: u64 = 3;
pub const MT7996_DEVICE_ID: u64 = 0x7990;
pub const MT7996_DEVICE_ID_2: u64 = 0x7991;
pub const MT7992_DEVICE_ID: u64 = 0x7992;
pub const MT7992_DEVICE_ID_2: u64 = 0x799a;
pub const MT7990_DEVICE_ID: u64 = 0x7993;
pub const MT7990_DEVICE_ID_2: u64 = 0x799b;
pub const MT7996_FIRMWARE_WA: &[u8] = b"mediatek/mt7996/mt7996_wa.bin";
pub const MT7996_FIRMWARE_WM: &[u8] = b"mediatek/mt7996/mt7996_wm.bin";
pub const MT7996_FIRMWARE_DSP: &[u8] = b"mediatek/mt7996/mt7996_dsp.bin";
pub const MT7996_ROM_PATCH: &[u8] = b"mediatek/mt7996/mt7996_rom_patch.bin";
pub const MT7996_FIRMWARE_WA_233: &[u8] = b"mediatek/mt7996/mt7996_wa_233.bin";
pub const MT7996_FIRMWARE_WM_233: &[u8] = b"mediatek/mt7996/mt7996_wm_233.bin";
pub const MT7996_ROM_PATCH_233: &[u8] = b"mediatek/mt7996/mt7996_rom_patch_233.bin";
pub const MT7992_FIRMWARE_WA: &[u8] = b"mediatek/mt7996/mt7992_wa.bin";
pub const MT7992_FIRMWARE_WM: &[u8] = b"mediatek/mt7996/mt7992_wm.bin";
pub const MT7992_FIRMWARE_DSP: &[u8] = b"mediatek/mt7996/mt7992_dsp.bin";
pub const MT7992_ROM_PATCH: &[u8] = b"mediatek/mt7996/mt7992_rom_patch.bin";
pub const MT7992_FIRMWARE_WA_23: &[u8] = b"mediatek/mt7996/mt7992_wa_23.bin";
pub const MT7992_FIRMWARE_WM_23: &[u8] = b"mediatek/mt7996/mt7992_wm_23.bin";
pub const MT7992_FIRMWARE_DSP_23: &[u8] = b"mediatek/mt7996/mt7992_dsp_23.bin";
pub const MT7992_ROM_PATCH_23: &[u8] = b"mediatek/mt7996/mt7992_rom_patch_23.bin";
pub const MT7992_FIRMWARE_WA_24: &[u8] = b"mediatek/mt7996/mt7992_wa_24.bin";
pub const MT7992_FIRMWARE_WM_24: &[u8] = b"mediatek/mt7996/mt7992_wm_24.bin";
pub const MT7992_FIRMWARE_DSP_24: &[u8] = b"mediatek/mt7996/mt7992_dsp_24.bin";
pub const MT7992_ROM_PATCH_24: &[u8] = b"mediatek/mt7996/mt7992_rom_patch_24.bin";
pub const MT7990_FIRMWARE_WA: &[u8] = b"";
pub const MT7990_FIRMWARE_WM: &[u8] = b"mediatek/mt7996/mt7990_wm.bin";
pub const MT7990_FIRMWARE_DSP: &[u8] = b"";
pub const MT7990_ROM_PATCH: &[u8] = b"mediatek/mt7996/mt7990_rom_patch.bin";
pub const MT7996_EEPROM_DEFAULT: &[u8] = b"mediatek/mt7996/mt7996_eeprom.bin";
pub const MT7996_EEPROM_DEFAULT_INT: &[u8] = b"mediatek/mt7996/mt7996_eeprom_2i5i6i.bin";
pub const MT7996_EEPROM_DEFAULT_233: &[u8] = b"mediatek/mt7996/mt7996_eeprom_233.bin";
pub const MT7996_EEPROM_DEFAULT_233_INT: &[u8] = b"mediatek/mt7996/mt7996_eeprom_233_2i5i6i.bin";
pub const MT7992_EEPROM_DEFAULT: &[u8] = b"mediatek/mt7996/mt7992_eeprom.bin";
pub const MT7992_EEPROM_DEFAULT_INT: &[u8] = b"mediatek/mt7996/mt7992_eeprom_2i5i.bin";
pub const MT7992_EEPROM_DEFAULT_MIX: &[u8] = b"mediatek/mt7996/mt7992_eeprom_2i5e.bin";
pub const MT7992_EEPROM_DEFAULT_23: &[u8] = b"mediatek/mt7996/mt7992_eeprom_23.bin";
pub const MT7992_EEPROM_DEFAULT_23_INT: &[u8] = b"mediatek/mt7996/mt7992_eeprom_23_2i5i.bin";
pub const MT7992_EEPROM_DEFAULT_24: &[u8] = b"mediatek/mt7996/mt7992_eeprom_24_2i5i.bin";
pub const MT7990_EEPROM_DEFAULT: &[u8] = b"mediatek/mt7996/mt7990_eeprom.bin";
pub const MT7990_EEPROM_DEFAULT_INT: &[u8] = b"mediatek/mt7996/mt7990_eeprom_2i5i.bin";
pub const MT7996_EEPROM_SIZE: u64 = 0x1e00;
pub const MT7996_EEPROM_BLOCK_SIZE: u64 = 16;
pub const MT7996_EXT_EEPROM_BLOCK_SIZE: u64 = 0x400;
pub const MT7996_TOKEN_SIZE: u64 = 0x4000;
pub const MT7996_HW_TOKEN_SIZE: u64 = 0x2000;
pub const MT7996_CFEND_RATE_DEFAULT: u64 = 73;
pub const MT7996_CFEND_RATE_11B: u64 = 3;
pub const MT7996_IBF_MAX_NC: u64 = 2;
pub const MT7996_IBF_TIMEOUT: u64 = 24;
pub const MT7996_IBF_TIMEOUT_LEGACY: u64 = 72;
pub const MT7992_CFEND_RATE_DEFAULT: u64 = 75;
pub const MT7992_IBF_TIMEOUT: u64 = 255;
pub const MT7996_SKU_RATE_NUM: u64 = 417;
pub const MT7996_SKU_PATH_NUM: u64 = 494;
pub const MT7996_MAX_TWT_AGRT: u64 = 16;
pub const MT7996_MAX_STA_TWT_AGRT: u64 = 8;
pub const MT7996_MIN_TWT_DUR: u64 = 64;
pub const MT7996_MAX_QUEUE: u64 = 27;
pub const MT7996_BASIC_RATES_TBL: u64 = 31;
pub const MT7996_BEACON_RATES_TBL: u64 = 25;
pub const MT7996_THERMAL_THROTTLE_MAX: u64 = 100;
pub const MT7996_CDEV_THROTTLE_MAX: u64 = 99;
pub const MT7996_CRIT_TEMP_IDX: u64 = 0;
pub const MT7996_MAX_TEMP_IDX: u64 = 1;
pub const MT7996_CRIT_TEMP: u64 = 110;
pub const MT7996_MAX_TEMP: u64 = 120;
pub const MT7996_MAX_HIF_RXD_IN_PG: u64 = 5;
pub const MT7996_RRO_MSDU_PG_HASH_SIZE: u64 = 127;
pub const MT7996_RRO_MAX_SESSION: u64 = 0x400;
pub const MT7996_RRO_WINDOW_MAX_LEN: u64 = 0x400;
pub const MT7996_RRO_ADDR_ELEM_LEN: u64 = 128;
pub const MT7996_RRO_BA_BITMAP_LEN: u64 = 2;
pub const MT7996_RRO_BA_BITMAP_CR_SIZE: u64 = 0x1_0000;
pub const MT7996_RRO_BA_BITMAP_SESSION_SIZE: u64 = 8;
pub const MT7996_RRO_WINDOW_MAX_SIZE: u64 = 0x2000;
pub const MT7996_RRO_MSDU_PG_CR_CNT: u64 = 8;
pub const MT7996_RRO_MSDU_PG_SIZE_PER_CR: u64 = 0x1_0000;
pub const MT7996_EFUSE_BASE_OFFS_ADIE0: u64 = 0x400;
pub const MT7996_EFUSE_BASE_OFFS_ADIE1: u64 = 0x1e00;
pub const MT7996_EFUSE_BASE_OFFS_ADIE2: u64 = 0x1200;
pub const MT7992_EFUSE_BASE_OFFS_ADIE1: u64 = 0x1200;
pub const WED_RRO_ADDR_SIGNATURE_MASK: u64 = 0xff00_0000;
pub const WED_RRO_ADDR_COUNT_MASK: u64 = 0x7ff0;
pub const WED_RRO_ADDR_HEAD_HIGH_MASK: u64 = 15;
pub const RRO_HIF_DATA1_LS_MASK: u64 = 0x4000_0000;
pub const RRO_HIF_DATA1_SDL_MASK: u64 = 0x3fff_0000;
pub const RRO_HIF_DATA4_RX_TOKEN_ID_MASK: u64 = 0xffff;
pub const MSDU_PAGE_INFO_OWNER_MASK: u64 = 0x8000_0000;
pub const MSDU_PAGE_INFO_PG_HIGH_MASK: u64 = 15;
pub const MT7996_MAX_RRO_RRS_RING: u64 = 4;
pub const MT7996_RAM_TYPE_WM: u64 = 0;
pub const MT7996_RAM_TYPE_WA: u64 = 1;
pub const MT7996_RAM_TYPE_DSP: u64 = 2;
pub const MT7996_VAR_TYPE_444: u64 = 0;
pub const MT7996_VAR_TYPE_233: u64 = 1;
pub const MT7992_VAR_TYPE_44: u64 = 0;
pub const MT7992_VAR_TYPE_23: u64 = 1;
pub const MT7992_VAR_TYPE_24: u64 = 2;
pub const MT7990_VAR_TYPE_23: u64 = 0;
pub const MT7996_FEM_EXT: u64 = 0;
pub const MT7996_FEM_INT: u64 = 1;
pub const MT7996_FEM_MIX: u64 = 2;
pub const EEPROM_MODE_DEFAULT_BIN: u64 = 0;
pub const EEPROM_MODE_EFUSE: u64 = 1;
pub const EEPROM_MODE_FLASH: u64 = 2;
pub const EEPROM_MODE_EXT: u64 = 3;
pub const MT7996_TXQ_FWDL: u64 = 16;
pub const MT7996_TXQ_MCU_WM: u64 = 17;
pub const MT7996_TXQ_BAND0: u64 = 18;
pub const MT7996_TXQ_BAND1: u64 = 19;
pub const MT7996_TXQ_MCU_WA: u64 = 20;
pub const MT7996_TXQ_BAND2: u64 = 21;
pub const MT7996_RXQ_MCU_WM: u64 = 0;
pub const MT7996_RXQ_MCU_WA: u64 = 1;
pub const MT7996_RXQ_MCU_WA_MAIN: u64 = 2;
pub const MT7996_RXQ_MCU_WA_EXT: u64 = 3;
pub const MT7996_RXQ_MCU_WA_TRI: u64 = 3;
pub const MT7996_RXQ_BAND0: u64 = 4;
pub const MT7996_RXQ_BAND1: u64 = 5;
pub const MT7996_RXQ_BAND2: u64 = 5;
pub const MT7996_RXQ_RRO_BAND0: u64 = 8;
pub const MT7996_RXQ_RRO_BAND1: u64 = 9;
pub const MT7996_RXQ_RRO_BAND2: u64 = 6;
pub const MT7996_RXQ_MSDU_PG_BAND0: u64 = 10;
pub const MT7996_RXQ_MSDU_PG_BAND1: u64 = 11;
pub const MT7996_RXQ_MSDU_PG_BAND2: u64 = 12;
pub const MT7996_RXQ_TXFREE0: u64 = 9;
pub const MT7996_RXQ_TXFREE1: u64 = 9;
pub const MT7996_RXQ_TXFREE2: u64 = 7;
pub const MT7996_RXQ_RRO_IND: u64 = 0;
pub const MT7996_RXQ_RRO_RXDMAD_C: u64 = 0;
pub const MT7990_RXQ_TXFREE0: u64 = 6;
pub const MT7990_RXQ_TXFREE1: u64 = 7;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_wed_rro_addr {
    pub head_low: Le32,
    pub data: Le32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_rro_hif {
    pub data0: Le32,
    pub data1: Le32,
    pub data2: Le32,
    pub data3: Le32,
    pub data4: Le32,
    pub data5: Le32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_msdu_page_info {
    pub rxd: [mt7996_rro_hif; 5],
    pub pg_low: Le32,
    pub data: Le32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_rro_queue_regs_emi_ring {
    pub idx: Le16,
    pub rsv: Le16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_rro_queue_regs_emi {
    pub ring: [mt7996_rro_queue_regs_emi_ring; 4],
}

pub const WFDMA0: u64 = 0;
pub const WFDMA1: u64 = 1;
pub const WFDMA_EXT: u64 = 2;
pub const __MT_WFDMA_MAX: u64 = 3;
pub const MT_RDD_IDX_BAND2: u64 = 0;
pub const MT_RDD_IDX_BAND1: u64 = 1;
pub const MT_RDD_IDX_BACKGROUND: u64 = 2;
pub const RDD_STOP: u64 = 0;
pub const RDD_START: u64 = 1;
pub const RDD_DET_MODE: u64 = 2;
pub const RDD_RADAR_EMULATE: u64 = 3;
pub const RDD_START_TXQ: u64 = 20;
pub const RDD_CAC_START: u64 = 50;
pub const RDD_CAC_END: u64 = 51;
pub const RDD_NORMAL_START: u64 = 52;
pub const RDD_DISABLE_DFS_CAL: u64 = 53;
pub const RDD_PULSE_DBG: u64 = 54;
pub const RDD_READ_PULSE: u64 = 55;
pub const RDD_RESUME_BF: u64 = 56;
pub const RDD_IRQ_OFF: u64 = 57;
