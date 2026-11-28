//! `mt76_connac.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT76_CONNAC_SCAN_IE_LEN: u64 = 600;
pub const MT76_CONNAC_MAX_NUM_SCHED_SCAN_INTERVAL: u64 = 10;
pub const MT76_CONNAC_MAX_SCHED_SCAN_SSID: u64 = 10;
pub const MT76_CONNAC_MAX_SCAN_MATCH: u64 = 16;
pub const MT76_CONNAC_MAX_WMM_SETS: u64 = 4;
pub const MT76_CONNAC_COREDUMP_SZ: u64 = 0x14_5000;
pub const MT_TXD_SIZE: u64 = 32;
pub const MT_USB_TXD_SIZE: u64 = 64;
pub const MT_USB_HDR_SIZE: u64 = 4;
pub const MT_USB_TAIL_SIZE: u64 = 4;
pub const MT_SDIO_TXD_SIZE: u64 = 64;
pub const MT_SDIO_TAIL_SIZE: u64 = 8;
pub const MT_SDIO_HDR_SIZE: u64 = 4;
pub const MT_MSDU_ID_VALID: u64 = 0x8000;
pub const MT_TXD_LEN_LAST: u64 = 0x8000;
pub const MT_TXD_LEN_MASK: u64 = 0xfff;
pub const MT_TXD_LEN_MSDU_LAST: u64 = 0x4000;
pub const MT_TXD_LEN_AMSDU_LAST: u64 = 0x8000;
pub const PCIE_AER_UNC_STATUS_OFFSET: u64 = 516;
pub const PCIE_AER_UNC_MASK_OFFSET: u64 = 520;
pub const PCIE_AER_CO_STATUS_OFFSET: u64 = 528;
pub const PKT_TYPE_TXS: u64 = 0;
pub const PKT_TYPE_TXRXV: u64 = 1;
pub const MT_TXP_MAX_BUF_NUM: u64 = 6;
pub const PKT_TYPE_NORMAL: u64 = 2;
pub const MT_HW_TXP_MAX_MSDU_NUM: u64 = 4;
pub const MT_HW_TXP_MAX_BUF_NUM: u64 = 4;
pub const PKT_TYPE_RX_DUP_RFB: u64 = 3;
pub const PKT_TYPE_RX_TMR: u64 = 4;
pub const PKT_TYPE_RETRIEVE: u64 = 5;
pub const PKT_TYPE_TXRX_NOTIFY: u64 = 6;
pub const PKT_TYPE_RX_EVENT: u64 = 7;
pub const PKT_TYPE_NORMAL_MCU: u64 = 8;
pub const PKT_TYPE_RX_FW_MONITOR: u64 = 12;
pub const PKT_TYPE_TXRX_NOTIFY_V0: u64 = 24;
pub const CMD_HE_MCS_BW80: u64 = 0;
pub const CMD_HE_MCS_BW160: u64 = 1;
pub const CMD_HE_MCS_BW8080: u64 = 2;
pub const CMD_HE_MCS_BW_NUM: u64 = 3;
pub const HW_BSSID_0: u64 = 0;
pub const HW_BSSID_1: u64 = 1;
pub const HW_BSSID_2: u64 = 2;
pub const HW_BSSID_3: u64 = 3;
pub const HW_BSSID_MAX: u64 = 3;
pub const EXT_BSSID_START: u64 = 16;
pub const EXT_BSSID_1: u64 = 17;
pub const EXT_BSSID_15: u64 = 31;
pub const EXT_BSSID_MAX: u64 = 31;
pub const REPEATER_BSSID_START: u64 = 32;
pub const REPEATER_BSSID_MAX: u64 = 63;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac_reg_map {
    pub phys: u32,
    pub maps: u32,
    pub size: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac_sta_key_conf {
    pub keyidx: i8,
    pub key: [u8; 16],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_fw_txp__packed {
    pub flags: Le16,
    pub token: Le16,
    pub bss_idx: u8,
    pub rept_wds_wcid: Le16,
    pub nbuf: u8,
    pub buf: [Le32; 6],
    pub len: [Le16; 6],
}
#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct mt76_connac_fw_txp(pub mt76_connac_fw_txp__packed);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac_txp_ptr {
    pub buf0: Le32,
    pub len0: Le16,
    pub len1: Le16,
    pub buf1: Le32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac_hw_txp {
    pub msdu_id: [Le16; 4],
    pub ptr: [mt76_connac_txp_ptr; 2],
}

#[allow(missing_debug_implementations)] // union fields share one store
#[repr(C)]
#[derive(Clone, Copy)]
pub union mt76_connac_txp_common_anon_0 {
    pub fw: mt76_connac_fw_txp,
    pub hw: mt76_connac_hw_txp,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac_txp_common {
    pub mt76_connac_txp_common_anon_0: mt76_connac_txp_common_anon_0,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac_tx_free {
    pub rx_byte_cnt: Le16,
    pub ctrl: Le16,
    pub txd: Le32,
}
