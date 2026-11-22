//! `mt7925/mcu.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT_RA_RATE_NSS: u64 = 448;
pub const MT_RA_RATE_MCS: u64 = 15;
pub const MT_RA_RATE_TX_MODE: u64 = 0x1e00;
pub const MT_RA_RATE_DCM_EN: u64 = 16;
pub const MT_RA_RATE_BW: u64 = 0x6000;
pub const MCU_EXT_EVENT_RATE_REPORT: u64 = 135;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_mcu_eeprom_info {
    pub addr: Le32,
    pub valid: Le32,
    pub data: [u8; 16],
}

pub const MT7925_RNR_SCAN_MAX_BSSIDS: u64 = 10;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_mcu_rxd {
    pub rxd: [Le32; 8],
    pub len: Le16,
    pub pkt_type_id: Le16,
    pub eid: u8,
    pub seq: u8,
    pub option: u8,
    pub __rsv: u8,
    pub ext_eid: u8,
    pub __rsv1: [u8; 2],
    pub s2d_index: u8,
}
pub const mt7925_mcu_rxd_FLEX: usize = core::mem::size_of::<u8>();

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_mcu_uni_event {
    pub cid: u8,
    pub pad: [u8; 3],
    pub status: Le32,
}

pub const MT_CONNAC3_SKU_POWER_LIMIT: u64 = 449;
pub const MT_EBF: u64 = 1;
pub const MT_IBF: u64 = 2;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_mcu_reg_event {
    pub reg: Le32,
    pub val: Le32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_mcu_ant_id_config {
    pub ant_id: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_txpwr_req {
    pub _rsv: [u8; 4],
    pub tag: Le16,
    pub len: Le16,
    pub format_id: u8,
    pub catg: u8,
    pub band_idx: u8,
    pub _rsv1: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_txpwr_event {
    pub rsv: [u8; 4],
    pub tag: Le16,
    pub len: Le16,
    pub catg: u8,
    pub band_idx: u8,
    pub ch_band: u8,
    pub format: u8,
    pub txpwr: crate::mt7925::mt7925::mt7925_txpwr,
    pub pwr_max: i8,
    pub pwr_min: i8,
    pub rsv1: u8,
}

pub const TM_SWITCH_MODE: u64 = 0;
pub const TM_SET_AT_CMD: u64 = 1;
pub const TM_QUERY_AT_CMD: u64 = 2;
pub const MT7925_TM_NORMAL: u64 = 0;
pub const MT7925_TM_TESTMODE: u64 = 1;
pub const MT7925_TM_ICAP: u64 = 2;
pub const MT7925_TM_ICAP_OVERLAP: u64 = 3;
pub const MT7925_TM_WIFISPECTRUM: u64 = 4;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_rftest_evt {
    pub param0: Le32,
    pub param1: Le32,
}

pub const UNI_CHANNEL_SWITCH: u64 = 0;
pub const UNI_CHANNEL_RX_PATH: u64 = 1;
pub const UNI_CHIP_CONFIG_CHIP_CFG: u64 = 2;
pub const UNI_CHIP_CONFIG_NIC_CAPA: u64 = 3;
pub const UNI_BAND_CONFIG_RADIO_ENABLE: u64 = 0;
pub const UNI_BAND_CONFIG_RTS_THRESHOLD: u64 = 8;
pub const UNI_BAND_CONFIG_SET_MAC80211_RX_FILTER: u64 = 12;
pub const UNI_WSYS_CONFIG_FW_LOG_CTRL: u64 = 0;
pub const UNI_WSYS_CONFIG_FW_DBG_CTRL: u64 = 1;
pub const UNI_EFUSE_ACCESS: u64 = 1;
pub const UNI_EFUSE_BUFFER_MODE: u64 = 2;
pub const UNI_EFUSE_FREE_BLOCK: u64 = 3;
pub const UNI_EFUSE_BUFFER_RD: u64 = 4;
pub const UNI_CMD_ACCESS_REG_BASIC: u64 = 0;
pub const UNI_CMD_ACCESS_RF_REG_BASIC: u64 = 1;
pub const UNI_MBMC_SETTING: u64 = 0;
pub const UNI_MBMC_NO_RESP_SETTING: u64 = 1;
pub const UNI_EVENT_SCAN_DONE_BASIC: u64 = 0;
pub const UNI_EVENT_SCAN_DONE_CHNLINFO: u64 = 2;
pub const UNI_EVENT_SCAN_DONE_NLO: u64 = 3;
pub const UNI_CMD_RSSI_MONITOR_SET: u64 = 0;
pub const UNI_EVENT_RSSI_MONITOR_INFO: u64 = 0;
pub const CONNAC3_CIPHER_NONE: u64 = 0;
pub const CONNAC3_CIPHER_WEP40: u64 = 1;
pub const CONNAC3_CIPHER_TKIP: u64 = 2;
pub const CONNAC3_CIPHER_AES_CCMP: u64 = 4;
pub const CONNAC3_CIPHER_WEP104: u64 = 5;
pub const CONNAC3_CIPHER_BIP_CMAC_128: u64 = 6;
pub const CONNAC3_CIPHER_WEP128: u64 = 7;
pub const CONNAC3_CIPHER_WAPI: u64 = 8;
pub const CONNAC3_CIPHER_CCMP_256: u64 = 10;
pub const CONNAC3_CIPHER_GCMP: u64 = 11;
pub const CONNAC3_CIPHER_GCMP_256: u64 = 12;
pub const DMASHDL_GROUP_0: u64 = 0;
pub const DMASHDL_GROUP_1: u64 = 1;
pub const DMASHDL_GROUP_2: u64 = 2;
pub const DMASHDL_GROUP_3: u64 = 3;
pub const DMASHDL_GROUP_4: u64 = 4;
pub const DMASHDL_GROUP_5: u64 = 5;
pub const DMASHDL_GROUP_6: u64 = 6;
pub const DMASHDL_GROUP_7: u64 = 7;
pub const DMASHDL_GROUP_8: u64 = 8;
pub const DMASHDL_GROUP_9: u64 = 9;
pub const DMASHDL_GROUP_10: u64 = 10;
pub const DMASHDL_GROUP_11: u64 = 11;
pub const DMASHDL_GROUP_12: u64 = 12;
pub const DMASHDL_GROUP_13: u64 = 13;
pub const DMASHDL_GROUP_14: u64 = 14;
pub const DMASHDL_GROUP_15: u64 = 15;
pub const DMASHDL_GROUP_NUM: u64 = 16;
pub const DMASHDL_LITE_GROUP_NUM: u64 = 64;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_mcu_scan_chinfo_event {
    pub nr_chan: u8,
    pub alpha2: [u8; 3],
}

pub const UNI_SCAN_REQ: u64 = 1;
pub const UNI_SCAN_CANCEL: u64 = 2;
pub const UNI_SCAN_SCHED_REQ: u64 = 3;
pub const UNI_SCAN_SCHED_ENABLE: u64 = 4;
pub const UNI_SCAN_SSID: u64 = 10;
pub const UNI_SCAN_BSSID: u64 = 11;
pub const UNI_SCAN_CHANNEL: u64 = 12;
pub const UNI_SCAN_IE: u64 = 13;
pub const UNI_SCAN_MISC: u64 = 14;
pub const UNI_SCAN_SSID_MATCH_SETS: u64 = 15;
pub const UNI_SNIFFER_ENABLE: u64 = 0;
pub const UNI_SNIFFER_CONFIG: u64 = 1;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct scan_hdr_tlv {
    pub seq_num: u8,
    pub bss_idx: u8,
    pub pad: [u8; 2],
}
pub const scan_hdr_tlv_FLEX: usize = core::mem::size_of::<u8>();

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct scan_req_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub scan_type: u8,
    pub probe_req_num: u8,
    pub scan_func: u8,
    pub src_mask: u8,
    pub channel_min_dwell_time: Le16,
    pub channel_dwell_time: Le16,
    pub timeout_value: Le16,
    pub probe_delay_time: Le16,
    pub func_mask_ext: Le32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct scan_chan_info_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub channel_type: u8,
    pub channels_num: u8,
    pub pad: [u8; 2],
    pub channels: [crate::mt76_connac_mcu::mt76_connac_mcu_scan_channel; 64],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct scan_ie_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub ies_len: Le16,
    pub band: u8,
    pub pad: u8,
}
pub const scan_ie_tlv_FLEX: usize = core::mem::size_of::<u8>();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct scan_sched_req {
    pub tag: Le16,
    pub len: Le16,
    pub version: u8,
    pub stop_on_match: u8,
    pub intervals_num: u8,
    pub scan_func: u8,
    pub intervals: [Le16; 10],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct scan_sched_enable {
    pub tag: Le16,
    pub len: Le16,
    pub active: u8,
    pub rsv: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mbmc_set_req {
    pub pad: [u8; 4],
}
pub const mbmc_set_req_FLEX: usize = core::mem::size_of::<u8>();

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mbmc_conf_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub mbmc_en: u8,
    pub band: u8,
    pub pad: [u8; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct edca {
    pub tag: Le16,
    pub len: Le16,
    pub queue: u8,
    pub set: u8,
    pub cw_min: u8,
    pub cw_max: u8,
    pub txop: Le16,
    pub aifs: u8,
    pub __rsv: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct bss_req_hdr {
    pub bss_idx: u8,
    pub __rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_rate_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub __rsv1: [u8; 2],
    pub basic_rate: Le16,
    pub bc_trans: Le16,
    pub mc_trans: Le16,
    pub short_preamble: u8,
    pub bc_fixed_rate: u8,
    pub mc_fixed_rate: u8,
    pub __rsv2: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_eht_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub is_eht_op_present: u8,
    pub is_eth_dscb_present: u8,
    pub eht_ctrl: u8,
    pub eht_ccfs0: u8,
    pub eht_ccfs1: u8,
    pub pad1: u8,
    pub eht_dis_sub_chan_bitmap: Le16,
    pub pad2: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_ba_uni {
    pub tag: Le16,
    pub len: Le16,
    pub tid: u8,
    pub ba_type: u8,
    pub amsdu: u8,
    pub ba_en: u8,
    pub ssn: Le16,
    pub winsize: Le16,
    pub ba_rdd_rro: u8,
    pub __rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_eht {
    pub tag: Le16,
    pub len: Le16,
    pub tid_bitmap: u8,
    pub _rsv: u8,
    pub mac_cap: Le16,
    pub phy_cap: Le64,
    pub phy_cap_ext: Le64,
    pub mcs_map_bw20: [u8; 4],
    pub mcs_map_bw80: [u8; 3],
    pub mcs_map_bw160: [u8; 3],
    pub mcs_map_bw320: [u8; 3],
    pub _rsv2: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_sec_uni {
    pub tag: Le16,
    pub len: Le16,
    pub add: u8,
    pub tx_key: u8,
    pub key_type: u8,
    pub is_authenticator: u8,
    pub peer_addr: [u8; 6],
    pub bss_idx: u8,
    pub cipher_id: u8,
    pub key_id: u8,
    pub key_len: u8,
    pub wlan_idx: u8,
    pub mgmt_prot: u8,
    pub key: [u8; 32],
    pub key_rsc: [u8; 16],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_hdr_trans {
    pub tag: Le16,
    pub len: Le16,
    pub from_ds: u8,
    pub to_ds: u8,
    pub dis_rx_hdr_tran: u8,
    pub rsv: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_eht_mld {
    pub tag: Le16,
    pub len: Le16,
    pub nsep: u8,
    pub mld_type: u8,
    pub __rsv1: [u8; 1],
    pub str_cap: [u8; 3],
    pub eml_cap: [u8; 3],
    pub __rsv2: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_ifs_time_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub slot_valid: u8,
    pub sifs_valid: u8,
    pub rifs_valid: u8,
    pub eifs_valid: u8,
    pub slot_time: Le16,
    pub sifs_time: Le16,
    pub rifs_time: Le16,
    pub eifs_time: Le16,
    pub eifs_cck_valid: u8,
    pub rsv: u8,
    pub eifs_cck_time: Le16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_rlm_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub control_channel: u8,
    pub center_chan: u8,
    pub center_chan2: u8,
    pub bw: u8,
    pub tx_streams: u8,
    pub rx_streams: u8,
    pub ht_op_info: u8,
    pub sco: u8,
    pub band: u8,
    pub pad: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_sku_tlv {
    pub channel: u8,
    pub pwr_limit: [i8; 449],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_tx_power_limit_tlv {
    pub rsv: [u8; 4],
    pub tag: Le16,
    pub len: Le16,
    pub ver: u8,
    pub pad0: u8,
    pub rsv1: Le16,
    pub n_chan: u8,
    pub band: u8,
    pub last_msg: u8,
    pub limit_type: u8,
    pub alpha2: [u8; 4],
    pub pad2: [u8; 32],
}
pub const mt7925_tx_power_limit_tlv_FLEX: usize = core::mem::size_of::<u8>();

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7925_arpns_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub enable: u8,
    pub ips_num: u8,
    pub rsv: [u8; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_wow_pattern_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub bss_idx: u8,
    pub index: u8,
    pub enable: u8,
    pub data_len: u8,
    pub offset: u8,
    pub mask: [u8; 16],
    pub pattern: [u8; 128],
    pub rsv: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct roc_acquire_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub bss_idx: u8,
    pub tokenid: u8,
    pub control_channel: u8,
    pub sco: u8,
    pub band: u8,
    pub bw: u8,
    pub center_chan: u8,
    pub center_chan2: u8,
    pub bw_from_ap: u8,
    pub center_chan_from_ap: u8,
    pub center_chan2_from_ap: u8,
    pub reqtype: u8,
    pub maxinterval: Le32,
    pub dbdcband: u8,
    pub rsv: [u8; 3],
}

pub const CMD_TEST_CTRL_ACT_SWITCH_MODE: u64 = 0;
pub const CMD_TEST_CTRL_ACT_SET_AT: u64 = 1;
pub const CMD_TEST_CTRL_ACT_GET_AT: u64 = 2;
pub const CMD_TEST_CTRL_ACT_SET_AT_ENG: u64 = 3;
pub const CMD_TEST_CTRL_ACT_GET_AT_ENG: u64 = 4;
pub const CMD_TEST_CTRL_ACT_NUM: u64 = 5;
pub const CMD_TEST_CTRL_ACT_SWITCH_MODE_NORMAL: u64 = 0;
pub const CMD_TEST_CTRL_ACT_SWITCH_MODE_RF_TEST: u64 = 1;
pub const CMD_TEST_CTRL_ACT_SWITCH_MODE_ICAP: u64 = 2;
pub const CMD_TEST_CTRL_ACT_SWITCH_MODE_NUM: u64 = 3;
#[allow(missing_debug_implementations)] // union fields share one store
#[repr(C)]
#[derive(Clone, Copy)]
pub union testmode_data {
    pub op_mode: Le32,
    pub channel_freq: Le32,
    pub rf_at_info: [u8; 84],
}

#[allow(missing_debug_implementations)] // union fields share one store
#[repr(C)]
#[derive(Clone, Copy)]
pub union testmode_evt {
    pub op_mode: Le32,
    pub channel_freq: Le32,
    pub rf_at_info: [u8; 1024],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct uni_cmd_testmode_ctrl {
    pub tag: u16,
    pub length: u16,
    pub action: u8,
    pub reserved: [u8; 3],
    pub data: testmode_data,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7925_rftest_cmd {
    pub padding: [u8; 4],
    pub ctrl: uni_cmd_testmode_ctrl,
}
