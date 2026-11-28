//! `mt792x.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT792x_MAX_INTERFACES: u64 = 4;
pub const MT792x_WTBL_SIZE: u64 = 20;
pub const MT792x_WTBL_RESERVED: u64 = 19;
pub const MT792x_WTBL_STA: u64 = 15;
pub const MT792x_CFEND_RATE_DEFAULT: u64 = 73;
pub const MT792x_CFEND_RATE_11B: u64 = 3;
pub const MT792x_FW_TAG_FEATURE: u64 = 4;
pub const MT792x_FW_CAP_NAN: u64 = 32;
pub const MT792x_FW_CAP_CNM: u64 = 128;
pub const MT792x_CHIP_CAP_CLC_EVT_EN: u64 = 1;
pub const MT792x_CHIP_CAP_RSSI_NOTIFY_EVT_EN: u64 = 2;
pub const MT792x_CHIP_CAP_WF_RF_PIN_CTRL_EVT_EN: u64 = 8;
pub const MT792x_CHIP_CAP_11D_EN: u64 = 16;
pub const MT792x_CHIP_CAP_REGD_EN: u64 = 32;
pub const MT792x_CHIP_CAP_MLO_EN: u64 = 256;
pub const MT792x_CHIP_CAP_MLO_EML_EN: u64 = 512;
pub const MT792x_BASIC_RATES_TBL: u64 = 14;
pub const MT792x_DRV_OWN_RETRY_COUNT: u64 = 10;
pub const MT792x_MCU_INIT_RETRY_COUNT: u64 = 10;
pub const MT792x_WFSYS_INIT_RETRY_COUNT: u64 = 2;
pub const MT7902_FIRMWARE_WM: &[u8] = b"mediatek/WIFI_RAM_CODE_MT7902_1.bin";
pub const MT7920_FIRMWARE_WM: &[u8] = b"mediatek/WIFI_RAM_CODE_MT7961_1a.bin";
pub const MT7921_FIRMWARE_WM: &[u8] = b"mediatek/WIFI_RAM_CODE_MT7961_1.bin";
pub const MT7922_FIRMWARE_WM: &[u8] = b"mediatek/WIFI_RAM_CODE_MT7922_1.bin";
pub const MT7925_FIRMWARE_WM: &[u8] = b"mediatek/mt7925/WIFI_RAM_CODE_MT7925_1_1.bin";
pub const MT7927_FIRMWARE_WM: &[u8] = b"mediatek/mt7927/WIFI_RAM_CODE_MT6639_2_1.bin";
pub const MT7928_FIRMWARE_WM: &[u8] = b"mediatek/mt7928/WIFI_RAM_CODE_MT7935_1_1.bin";
pub const MT7902_ROM_PATCH: &[u8] = b"mediatek/WIFI_MT7902_patch_mcu_1_1_hdr.bin";
pub const MT7920_ROM_PATCH: &[u8] = b"mediatek/WIFI_MT7961_patch_mcu_1a_2_hdr.bin";
pub const MT7921_ROM_PATCH: &[u8] = b"mediatek/WIFI_MT7961_patch_mcu_1_2_hdr.bin";
pub const MT7922_ROM_PATCH: &[u8] = b"mediatek/WIFI_MT7922_patch_mcu_1_1_hdr.bin";
pub const MT7925_ROM_PATCH: &[u8] = b"mediatek/mt7925/WIFI_MT7925_PATCH_MCU_1_1_hdr.bin";
pub const MT7927_ROM_PATCH: &[u8] = b"mediatek/mt7927/WIFI_MT6639_PATCH_MCU_2_1_hdr.bin";
pub const MT7928_ROM_PATCH: &[u8] = b"mediatek/mt7928/WIFI_MT7935_PATCH_MCU_1_1_hdr.bin";
pub const MT7928_CB_ROM_PATCH: &[u8] = b"mediatek/mt7928/CBMCU_CODE_MT7935_1_1.bin";
pub const MT7928_PHY_RAM: &[u8] = b"mediatek/mt7928/WIFI_MT7935_PHY_RAM_CODE_1_1.bin";
pub const MT792x_SDIO_HDR_TX_BYTES: u64 = 0xffff;
pub const MT792x_SDIO_HDR_PKT_TYPE: u64 = 0x3_0000;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt792x_realease_info {
    pub len: Le16,
    pub pad_len: u8,
    pub tag: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_fw_features {
    pub segment: u8,
    pub data: u8,
    pub rsv: [u8; 14],
}

pub const MT792x_CLC_POWER: u64 = 0;
pub const MT792x_CLC_POWER_EXT: u64 = 1;
pub const MT792x_CLC_BE_CTRL: u64 = 2;
pub const MT792x_CLC_REGD: u64 = 3;
pub const MT792x_CLC_MAX_NUM: u64 = 4;
pub const MT_AP_UNSET: u64 = 0;
pub const MT_AP_DEFAULT: u64 = 1;
pub const MT_AP_LPI: u64 = 2;
pub const MT_AP_SP: u64 = 3;
pub const MT_AP_VLP: u64 = 4;
pub const MT792x_MLO_LINK_DISASSOC: u64 = 0;
pub const MT792x_MLO_LINK_ASSOC: u64 = 1;
pub const MT792x_MLO_CHANGED_PS_PENDING: u64 = 2;
pub const MT792x_MLO_CHANGED_PS: u64 = 3;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_irq_map_tx {
    pub all_complete_mask: u32,
    pub mcu_complete_mask: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_irq_map_rx {
    pub all_complete_mask: u32,
    pub data_complete_mask: u32,
    pub wm_complete_mask: u32,
    pub wm2_complete_mask: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_irq_map {
    pub host_irq_enable: u32,
    pub tx: mt792x_irq_map_tx,
    pub rx: mt792x_irq_map_rx,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_dma_ring {
    pub qid: u8,
    pub pad_0: u8,
    pub n_desc: u16,
    pub ring_base: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_dma_layout {
    pub tx_data0: mt792x_dma_ring,
    pub tx_mcu: mt792x_dma_ring,
    pub tx_fwdl: mt792x_dma_ring,
    pub tx_done: mt792x_dma_ring,
    pub rx_data: mt792x_dma_ring,
    pub rx_mcu: mt792x_dma_ring,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt792x_pcie_reg {
    pub imask: u32,
    pub pm: u32,
}
