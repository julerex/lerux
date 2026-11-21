//! `mt7921/mt7921.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT7921_MAX_AID: u64 = 20;
pub const MT7921_TX_RING_SIZE: u64 = 0x800;
pub const MT7921_TX_MCU_RING_SIZE: u64 = 256;
pub const MT7921_TX_FWDL_RING_SIZE: u64 = 128;
pub const MT7921_RX_RING_SIZE: u64 = 0x600;
pub const MT7921_RX_MCU_RING_SIZE: u64 = 8;
pub const MT7921_RX_MCU_WA_RING_SIZE: u64 = 512;
pub const MT7902_RX_MCU_RING_SIZE: u64 = 512;
pub const MT7921_EEPROM_SIZE: u64 = 0xe00;
pub const MT7921_TOKEN_SIZE: u64 = 0x2000;
pub const MT7921_EEPROM_BLOCK_SIZE: u64 = 16;
pub const MT7921_SKU_RATE_NUM: u64 = 161;
pub const MT7921_SKU_MAX_DELTA_IDX: u64 = 161;
pub const MT7921_SKU_TABLE_SIZE: u64 = 162;
pub const MCU_UNI_EVENT_ROC: u64 = 39;
pub const MCU_UNI_EVENT_CLC: u64 = 128;
pub const EXT_CMD_RADIO_LED_CTRL_ENABLE: u64 = 1;
pub const EXT_CMD_RADIO_ON_LED: u64 = 2;
pub const EXT_CMD_RADIO_OFF_LED: u64 = 3;
pub const WF_RF_PIN_INIT: u64 = 0;
pub const WF_RF_PIN_POLL: u64 = 1;
#[must_use]
#[rustfmt::skip]
pub const fn to_rcpi(rssi: u64) -> u64 { ((2u64).wrapping_mul(rssi)).wrapping_add(220u64) }

pub const MT_EE_HW_TYPE_ENCAP: u64 = 1;
pub const UNI_ROC_ACQUIRE: u64 = 0;
pub const UNI_ROC_ABORT: u64 = 1;
pub const UNI_ROC_NUM: u64 = 2;
pub const MT7921_ROC_REQ_JOIN: u64 = 0;
pub const MT7921_ROC_REQ_ROC: u64 = 1;
pub const MT7921_ROC_REQ_NUM: u64 = 2;
pub const UNI_EVENT_ROC_GRANT: u64 = 0;
pub const UNI_EVENT_ROC_TAG_NUM: u64 = 1;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7921_realease_info {
    pub len: Le16,
    pub pad_len: u8,
    pub tag: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7921_fw_features {
    pub segment: u8,
    pub data: u8,
    pub rsv: [u8; 14],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7921_roc_grant_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub bss_idx: u8,
    pub tokenid: u8,
    pub status: u8,
    pub primarychannel: u8,
    pub rfsco: u8,
    pub rfband: u8,
    pub channelwidth: u8,
    pub centerfreqseg1: u8,
    pub centerfreqseg2: u8,
    pub reqtype: u8,
    pub dbdcband: u8,
    pub rsv: [u8; 1],
    pub max_interval: Le32,
}

pub const MT7921_SDIO_TXD: u64 = 0;
pub const MT7921_SDIO_DATA: u64 = 1;
pub const MT7921_SDIO_CMD: u64 = 2;
pub const MT7921_SDIO_FWDL: u64 = 3;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7921_sdio_intr_tx {
    pub wtqcr: [u32; 16],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7921_sdio_intr_rx {
    pub num: [u16; 2],
    pub len0: [u16; 16],
    pub len1: [u16; 128],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7921_sdio_intr {
    pub isr: u32,
    pub tx: mt7921_sdio_intr_tx,
    pub rx: mt7921_sdio_intr_rx,
    pub rec_mb: [u32; 2],
}

pub const MT7921_TXQ_BAND0: u64 = 0;
pub const MT7921_TXQ_BAND1: u64 = 1;
pub const MT7921_TXQ_FWDL: u64 = 16;
pub const MT7921_TXQ_MCU_WM: u64 = 17;
pub const MT7921_RXQ_BAND0: u64 = 0;
pub const MT7921_RXQ_BAND1: u64 = 1;
pub const MT7921_RXQ_MCU_WM: u64 = 0;
pub const MT7902_TXQ_MCU_WM: u64 = 15;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7921_dma_layout {
    pub mcu_wm_txq: u8,
    pub pad_0: u8,
    pub mcu_rxdone_ring_size: u16,
    pub has_mcu_wa: u8,
}

pub const MT7921_CLC_POWER: u64 = 0;
pub const MT7921_CLC_CHAN: u64 = 1;
pub const MT7921_CLC_MAX_NUM: u64 = 2;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7921_clc_rule {
    pub alpha2: [u8; 2],
    pub r#type: [u8; 2],
    pub len: Le16,
}
pub const mt7921_clc_rule_FLEX: usize = core::mem::size_of::<u8>();

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7921_clc {
    pub len: Le32,
    pub idx: u8,
    pub ver: u8,
    pub nr_country: u8,
    pub r#type: u8,
    pub rsv: [u8; 8],
}
pub const mt7921_clc_FLEX: usize = core::mem::size_of::<u8>();

pub const MT_EE_CHIP_ID: u64 = 0;
pub const MT_EE_VERSION: u64 = 2;
pub const MT_EE_MAC_ADDR: u64 = 4;
pub const MT_EE_WIFI_CONF: u64 = 124;
pub const MT_EE_HW_TYPE: u64 = 0x55b;
pub const __MT_EE_MAX: u64 = 0x9ff;
pub const TXPWR_USER: u64 = 0;
pub const TXPWR_EEPROM: u64 = 1;
pub const TXPWR_MAC: u64 = 2;
pub const TXPWR_MAX_NUM: u64 = 3;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7921_txpwr_data {
    pub ch: u8,
    pub cck: [u8; 4],
    pub ofdm: [u8; 8],
    pub ht20: [u8; 8],
    pub ht40: [u8; 9],
    pub vht20: [u8; 12],
    pub vht40: [u8; 12],
    pub vht80: [u8; 12],
    pub vht160: [u8; 12],
    pub he26: [u8; 12],
    pub he52: [u8; 12],
    pub he106: [u8; 12],
    pub he242: [u8; 12],
    pub he484: [u8; 12],
    pub he996: [u8; 12],
    pub he996x2: [u8; 12],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7921_txpwr {
    pub ch: u8,
    pub rsv: [u8; 3],
    pub data: [mt7921_txpwr_data; 3],
}
