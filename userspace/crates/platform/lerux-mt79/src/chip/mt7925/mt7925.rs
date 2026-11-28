//! `mt7925/mt7925.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT7925_BEACON_RATES_TBL: u64 = 25;
pub const MT7925_TX_RING_SIZE: u64 = 0x800;
pub const MT7925_TX_MCU_RING_SIZE: u64 = 256;
pub const MT7925_TX_FWDL_RING_SIZE: u64 = 128;
pub const MT7925_RX_RING_SIZE: u64 = 0x600;
pub const MT7925_RX_MCU_RING_SIZE: u64 = 512;
pub const MT7928_RX_MCU_WA_RING_SIZE: u64 = 512;
pub const MT7925_EEPROM_SIZE: u64 = 0xe00;
pub const MT7925_TOKEN_SIZE: u64 = 0x2000;
pub const MT7925_EEPROM_BLOCK_SIZE: u64 = 16;
pub const MT7925_SKU_RATE_NUM: u64 = 161;
pub const MT7925_SKU_MAX_DELTA_IDX: u64 = 161;
pub const MT7925_SKU_TABLE_SIZE: u64 = 162;
pub const MCU_UNI_EVENT_ROC: u64 = 39;
pub const HIF_TRAFFIC_IDLE: u64 = 2;
pub const UNI_EVENT_HIF_CTRL_BASIC: u64 = 0;
#[must_use]
#[rustfmt::skip]
pub const fn to_rcpi(rssi: u64) -> u64 { ((2u64).wrapping_mul(rssi)).wrapping_add(220u64) }

pub const UNI_EVENT_HIF_CTRL_TAG_NUM: u64 = 1;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_mcu_hif_ctrl_basic_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub cid: u8,
    pub pad: [u8; 3],
    pub status: u32,
    pub hif_type: u8,
    pub hif_tx_traffic_status: u8,
    pub hif_rx_traffic_status: u8,
    pub hifsuspend: u8,
    pub rsv: [u8; 4],
}

pub const MT_EE_HW_TYPE_ENCAP: u64 = 3;
pub const UNI_ROC_ACQUIRE: u64 = 0;
pub const UNI_ROC_ABORT: u64 = 1;
pub const UNI_ROC_SUB_LINK: u64 = 3;
pub const UNI_ROC_NUM: u64 = 4;
pub const MT7925_ROC_REQ_JOIN: u64 = 0;
pub const MT7925_ROC_REQ_ROC: u64 = 1;
pub const MT7925_ROC_REQ_SUB_LINK: u64 = 2;
pub const MT7925_ROC_REQ_MLSR_AG: u64 = 10;
pub const MT7925_ROC_REQ_MLSR_AA: u64 = 11;
pub const MT7925_ROC_REQ_NUM: u64 = 12;
pub const UNI_EVENT_ROC_GRANT: u64 = 0;
pub const UNI_EVENT_ROC_GRANT_SUB_LINK: u64 = 4;
pub const UNI_EVENT_ROC_TAG_NUM: u64 = 5;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_roc_grant_tlv {
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

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_beacon_loss_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub reason: u8,
    pub nr_btolink: u8,
    pub pad: [u8; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_uni_beacon_loss_event_hdr {
    pub bss_idx: u8,
    pub pad: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_uni_beacon_loss_event {
    pub hdr: mt7925_uni_beacon_loss_event_hdr,
    pub beacon_loss: mt7925_beacon_loss_tlv,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_uni_rssi_monitor_event {
    pub tag: Le16,
    pub len: Le16,
    pub rssi: Le32,
}

pub const MT7925_TXQ_BAND0: u64 = 0;
pub const MT7925_TXQ_BAND1: u64 = 1;
pub const MT7925_TXQ_MCU_WM: u64 = 15;
pub const MT7925_TXQ_FWDL: u64 = 16;
pub const MT7925_RXQ_BAND0: u64 = 2;
pub const MT7925_RXQ_BAND1: u64 = 3;
pub const MT7925_RXQ_MCU_WM: u64 = 0;
pub const MT7925_RXQ_MCU_WM2: u64 = 1;
pub const MT7927_TXQ_BAND0: u64 = 0;
pub const MT7927_TXQ_BAND1: u64 = 1;
pub const MT7927_TXQ_MCU_WM: u64 = 15;
pub const MT7927_TXQ_FWDL: u64 = 16;
pub const MT7927_RXQ_BAND0: u64 = 4;
pub const MT7927_RXQ_MCU_WM: u64 = 6;
pub const MT7927_RXQ_DATA2: u64 = 7;
pub const MT7928_RXQ_BAND0: u64 = 0;
pub const MT7928_RXQ_BAND1: u64 = 2;
pub const MT7928_RXQ_MCU_WM: u64 = 3;
pub const MT7928_RXQ_MCU_WM2: u64 = 1;
pub const MODE_OPEN: u64 = 0;
pub const MODE_SHARED: u64 = 1;
pub const MODE_WPA: u64 = 3;
pub const MODE_WPA_PSK: u64 = 4;
pub const MODE_WPA_NONE: u64 = 5;
pub const MODE_WPA2: u64 = 6;
pub const MODE_WPA2_PSK: u64 = 7;
pub const MODE_WPA3_SAE: u64 = 11;
pub const MT7925_CLC_POWER: u64 = 0;
pub const MT7925_CLC_CHAN: u64 = 1;
pub const MT7925_CLC_MAX_NUM: u64 = 2;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_clc_rule_v2 {
    pub flag: u32,
    pub alpha2: [u8; 2],
    pub rsv: [u8; 10],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_clc_rule {
    pub alpha2: [u8; 2],
    pub r#type: [u8; 2],
    pub seg_idx: u8,
    pub flag: u8,
    pub rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_clc_segment {
    pub idx: u8,
    pub rsv1: [u8; 3],
    pub offset: u32,
    pub len: u32,
    pub rsv2: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_clc_type0 {
    pub nr_country: u8,
    pub r#type: u8,
    pub nr_seg: u8,
    pub rsv: [u8; 7],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_clc_type2 {
    pub r#type: u8,
    pub rsv: [u8; 9],
}

#[allow(missing_debug_implementations)] // union fields share one store
#[repr(C)]
#[derive(Clone, Copy)]
pub union mt7925_clc_anon_3 {
    pub t0: mt7925_clc_type0,
    pub t2: mt7925_clc_type2,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_clc {
    pub len: Le32,
    pub idx: u8,
    pub ver: u8,
    pub mt7925_clc_anon_3: mt7925_clc_anon_3,
}
pub const mt7925_clc_FLEX: usize = core::mem::size_of::<u8>();

pub const MT_EE_CHIP_ID: u64 = 0;
pub const MT_EE_VERSION: u64 = 2;
pub const MT_EE_MAC_ADDR: u64 = 4;
pub const MT_EE_HW_TYPE: u64 = 0xa71;
pub const __MT_EE_MAX: u64 = 0x9ff;
pub const TXPWR_USER: u64 = 0;
pub const TXPWR_EEPROM: u64 = 1;
pub const TXPWR_MAC: u64 = 2;
pub const TXPWR_MAX_NUM: u64 = 3;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_txpwr {
    pub cck: [[i8; 2]; 4],
    pub ofdm: [[i8; 2]; 8],
    pub ht20: [[i8; 2]; 8],
    pub ht40: [[i8; 2]; 9],
    pub vht20: [[i8; 2]; 12],
    pub vht40: [[i8; 2]; 12],
    pub vht80: [[i8; 2]; 12],
    pub vht160: [[i8; 2]; 12],
    pub he26: [[i8; 2]; 12],
    pub he52: [[i8; 2]; 12],
    pub he106: [[i8; 2]; 12],
    pub he242: [[i8; 2]; 12],
    pub he484: [[i8; 2]; 12],
    pub he996: [[i8; 2]; 12],
    pub he996x2: [[i8; 2]; 12],
    pub eht26: [[i8; 2]; 16],
    pub eht52: [[i8; 2]; 16],
    pub eht106: [[i8; 2]; 16],
    pub eht242: [[i8; 2]; 16],
    pub eht484: [[i8; 2]; 16],
    pub eht996: [[i8; 2]; 16],
    pub eht996x2: [[i8; 2]; 16],
    pub eht996x4: [[i8; 2]; 16],
    pub eht26_52: [[i8; 2]; 16],
    pub eht26_106: [[i8; 2]; 16],
    pub eht484_242: [[i8; 2]; 16],
    pub eht996_484: [[i8; 2]; 16],
    pub eht996_484_242: [[i8; 2]; 16],
    pub eht996x2_484: [[i8; 2]; 16],
    pub eht996x3: [[i8; 2]; 16],
    pub eht996x3_484: [[i8; 2]; 16],
}
