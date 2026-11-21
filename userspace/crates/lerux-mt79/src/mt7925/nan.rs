//! `mt7925/nan.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const NAN_MAX_SOCIAL_CHANNELS: u64 = 3;
pub const NAN_ANCHOR_MASTER_RANK_NUM: u64 = 8;
pub const NAN_5G_LOW_DISC_CHANNEL: u64 = 44;
pub const NAN_5G_HIGH_DISC_CHANNEL: u64 = 149;
pub const NAN_MAX_MASTER_PREFERENCE: u64 = 255;
pub const NAN_DEFAULT_DW_INTERVAL: u64 = 1;
pub const NAN_DEFAULT_DISC_BCN_INTERVAL: u64 = 100;
pub const NAN_TOTAL_DW: u64 = 16;
pub const NAN_SUPPORTED_2G_FAW_CH_NUM: u64 = 4;
pub const NAN_SUPPORTED_5G_FAW_CH_NUM: u64 = 4;
pub const NAN_TIMELINE_MGMT_SIZE: u64 = 2;
pub const NAN_TIMELINE_MGMT_CHNL_LIST_NUM: u64 = 4;
pub const NAN_NUM_AVAIL_DB: u64 = 2;
pub const NAN_NDC_ATTRIBUTE_ID_LENGTH: u64 = 6;
pub const NAN_MAX_CONN_CFG: u64 = 8;
pub const NAN_MAX_NDP_CXT: u64 = 4;
pub const NAN_AVAIL_ATTR_ID_OFFSET: u64 = 0;
pub const NAN_AVAIL_ATTR_LEN_OFFSET: u64 = 1;
pub const NAN_AVAIL_SEQ_ID_OFFSET: u64 = 3;
pub const NAN_AVAIL_ATTR_CTRL_OFFSET: u64 = 4;
pub const NAN_AVAIL_CTRL_MAPID: u64 = 15;
pub const NAN_AVAIL_CTRL_COMMIT_CHANGED: u64 = 16;
pub const NAN_AVAIL_CTRL_POTN_CHANGED: u64 = 32;
pub const NAN_AVAIL_CTRL_PUBLIC_AVAIL_CHANGED: u64 = 64;
pub const NAN_AVAIL_CTRL_NDC_CHANGED: u64 = 128;
pub const NAN_AVAIL_CTRL_CHECK_FOR_CHANGED: u64 = 240;
pub const UNII1_LOWER_BOUND: u64 = 36;
pub const UNII1_UPPER_BOUND: u64 = 50;
pub const UNII3_LOWER_BOUND: u64 = 149;
pub const UNII3_UPPER_BOUND: u64 = 165;
pub const NAN_CH_CTRL_OP_CLASS: u64 = 0xff00;
pub const NAN_CH_CTRL_PRIMARY_CH: u64 = 0xff_0000;
pub const NAN_CRB_USE_DATA_PATH: u64 = 1;
pub const NAN_CRB_AVAIL_6G_FORMAT: u64 = 6;
pub const NAN_UNI_CMD_SET_MASTER_PREFERENCE: u64 = 0;
pub const NAN_UNI_CMD_ENABLE_REQUEST: u64 = 7;
pub const NAN_UNI_CMD_DISABLE_REQUEST: u64 = 8;
pub const NAN_UNI_CMD_UPDATE_AVAILABILITY: u64 = 9;
pub const NAN_UNI_CMD_UPDATE_CRB: u64 = 10;
pub const NAN_UNI_CMD_MANAGE_PEER_SCH_RECORD: u64 = 12;
pub const NAN_UNI_CMD_MAP_STA_RECORD: u64 = 13;
pub const NAN_UNI_CMD_UPDATE_AVAILABILITY_CTRL: u64 = 20;
pub const NAN_UNI_CMD_UPDATE_PEER_CAPABILITY: u64 = 21;
pub const NAN_UNI_CMD_CHANGE_NMI_ADDRESS: u64 = 24;
pub const NAN_UNI_CMD_SET_DW_INTERVAL: u64 = 26;
pub const NAN_UNI_CMD_SET_SYNC_RSSI: u64 = 39;
pub const NAN_UNI_CMD_SET_CLUSTER_ID: u64 = 40;
pub const NAN_UNI_CMD_KEY_MANAGEMENT: u64 = 53;
pub const NAN_UNI_EVENT_ID_DE_EVENT_IND: u64 = 19;
pub const NAN_UNI_EVENT_REPORT_DW_END: u64 = 60;
pub const NAN_EVENT_ID_DISC_MAC_ADDR: u64 = 0;
pub const NAN_EVENT_ID_JOINED_CLUSTER: u64 = 2;
pub const NAN_SUPPORTED_BAND_ID_2P4G: u64 = 2;
pub const NAN_SUPPORTED_BAND_ID_5G: u64 = 4;
pub const NAN_PROPRIETARY_BAND_ID_6G: u64 = 6;
pub const NAN_SUPPORTED_BAND_ID_6G: u64 = 7;
pub const NAN_SUPPORTED_BN_2G: u64 = 0;
pub const NAN_SUPPORTED_BN_5G_LOW: u64 = 1;
pub const NAN_SUPPORTED_BN_5G_HIGH: u64 = 2;
pub const NAN_SUPPORTED_BN_6G: u64 = 3;
pub const NAN_SUPPORTED_BN_NUM: u64 = 4;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_nan_social_ch_scan_params {
    pub dwell_time: [u8; 3],
    pub scan_period: [Le16; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_nan_conf_dw {
    pub config_2dot4g_dw_band: u8,
    pub dw_2dot4g_interval_val: Le32,
    pub config_5g_dw_band: u8,
    pub dw_5g_interval_val: Le32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_nan_common_hdr {
    pub reserved: [u8; 4],
}

#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct mt7925_nan_master_preference_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub master_preference: u8,
    pub reserved: [u8; 3],
}

#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct mt7925_nan_dw_interval_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub dw_interval: u8,
    pub vendor_ioctl: u8,
    pub disc_bcn_interval: Le16,
}

#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct mt7925_nan_sync_rssi_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub rssi_close_2g: i8,
    pub rssi_middle_2g: i8,
    pub rssi_close_5g: i8,
    pub rssi_middle_5g: i8,
}

#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct mt7925_nan_avail_ctrl_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub avail_ctrl: Le16,
    pub seq_id: u8,
    pub reserved: [u8; 1],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_nan_ch_timeline {
    pub is_valid: u8,
    pub reserved: [u8; 3],
    pub ch_info: Le32,
    pub num: Le32,
    pub avail_map: [Le32; 16],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_nan_avail_entry_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub map_id: u8,
    pub is_cond_avail: u8,
    pub timeline_idx: u8,
    pub is_multi_map: u8,
    pub ch_list: [mt7925_nan_ch_timeline; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_nan_sched_update_peer_cap_tlv__packed {
    pub tag: Le16,
    pub len: Le16,
    pub sch_idx: Le32,
    pub supported_bands: u8,
    pub max_chnl_switch_time: Le16,
    pub peer_supported_bands: u8,
}
#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct mt7925_nan_sched_update_peer_cap_tlv(pub mt7925_nan_sched_update_peer_cap_tlv__packed);

#[allow(missing_debug_implementations)] // union fields share one store
#[repr(C)]
#[derive(Clone, Copy)]
pub union mt7925_nan_sched_timeline_anon_3 {
    pub avail_map: [Le32; 16],
    pub avail_block: [u8; 64],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_nan_sched_timeline {
    pub map_id: u8,
    pub local_map_id: u8,
    pub reserved: [u8; 2],
    pub mt7925_nan_sched_timeline_anon_3: mt7925_nan_sched_timeline_anon_3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_nan_sched_faw_ndc_timeline {
    pub avail_map: [Le32; 16],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_nan_sched_ndc_ctrl {
    pub is_valid: u8,
    pub ndc_id: [u8; 6],
    pub ndc_idx: u8,
    pub timeline: [mt7925_nan_sched_timeline; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_nan_sched_update_crb_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub sch_idx: Le32,
    pub flags: u8,
    pub is_use_ranging: u8,
    pub reserved: [u8; 2],
    pub comm_ranging_timeline: [mt7925_nan_sched_timeline; 2],
    pub comm_faw_timeline: [mt7925_nan_sched_timeline; 2],
    pub comm_ndc_ctrl: mt7925_nan_sched_ndc_ctrl,
    pub faw_ndc_timeline: [mt7925_nan_sched_faw_ndc_timeline; 2],
}
