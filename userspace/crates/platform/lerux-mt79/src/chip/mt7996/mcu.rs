//! `mt7996/mcu.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_rxd {
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

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_uni_event {
    pub cid: Le16,
    pub __rsv: [u8; 2],
    pub status: Le32,
}

pub const MT7996_PS_MULTI_WCID: u64 = 0x7ff;
pub const MT7996_PS_MULTI_PS_BIT: u64 = 0x8000;
#[must_use]
#[rustfmt::skip]
pub const fn MCU_PQ_ID(p: u64, q: u64) -> u64 { (shl(p, 15u64)) | (shl(q, 10u64)) }

pub const MCU_PKT_ID: u64 = 160;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_thermal_ctrl_anon_2_type {
    pub protect_type: u8,
    pub trigger_type: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_thermal_ctrl_anon_2_duty {
    pub duty_level: u8,
    pub duty_cycle: u8,
}

#[allow(missing_debug_implementations)] // union fields share one store
#[repr(C)]
#[derive(Clone, Copy)]
pub union mt7996_mcu_thermal_ctrl_anon_2 {
    pub r#type: mt7996_mcu_thermal_ctrl_anon_2_type,
    pub duty: mt7996_mcu_thermal_ctrl_anon_2_duty,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_thermal_ctrl {
    pub ctrl_id: u8,
    pub band_idx: u8,
    pub mt7996_mcu_thermal_ctrl_anon_2: mt7996_mcu_thermal_ctrl_anon_2,
}

pub const MAX_BEACON_NUM: u64 = 32;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_thermal_enable {
    pub trigger_temp: Le32,
    pub restore_temp: Le32,
    pub sustain_time: Le16,
    pub rsv: [u8; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_countdown_notify {
    pub omac_idx: u8,
    pub count: u8,
    pub csa_failure_reason: u8,
    pub rsv: u8,
}

pub const MT7996_MAX_BSS_OFFLOAD_SIZE: u64 = 0x800;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_rdd_report_long_pulse {
    pub start: Le32,
    pub pulse_width: Le16,
    pub pulse_power: Le16,
    pub mdrdy_flag: u8,
    pub rsv: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_rdd_report_periodic_pulse {
    pub start: Le32,
    pub pulse_width: Le16,
    pub pulse_power: Le16,
    pub mdrdy_flag: u8,
    pub rsv: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_rdd_report_hw_pulse {
    pub start: Le32,
    pub pulse_width: Le16,
    pub pulse_power: Le16,
    pub sc_pass: u8,
    pub sw_reset: u8,
    pub mdrdy_flag: u8,
    pub tx_active: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_rdd_report {
    pub rxd: mt7996_mcu_rxd,
    pub __rsv1: [u8; 4],
    pub tag: Le16,
    pub len: Le16,
    pub rdd_idx: u8,
    pub long_detected: u8,
    pub constant_prf_detected: u8,
    pub staggered_prf_detected: u8,
    pub radar_type_idx: u8,
    pub periodic_pulse_num: u8,
    pub long_pulse_num: u8,
    pub hw_pulse_num: u8,
    pub out_lpn: u8,
    pub out_spn: u8,
    pub out_crpn: u8,
    pub out_crpw: u8,
    pub out_crbn: u8,
    pub out_stgpn: u8,
    pub out_stgpw: u8,
    pub __rsv2: u8,
    pub out_pri_const: Le32,
    pub out_pri_stg: [Le32; 3],
    pub out_pri_stg_dmin: Le32,
    pub long_pulse: [mt7996_mcu_rdd_report_long_pulse; 32],
    pub periodic_pulse: [mt7996_mcu_rdd_report_periodic_pulse; 32],
    pub hw_pulse: [mt7996_mcu_rdd_report_hw_pulse; 32],
}

pub const MT7996_PATCH_SEC: u64 = 0xff00_0000;
pub const MT7996_PATCH_SCRAMBLE_KEY: u64 = 0xff00;
pub const MT7996_PATCH_AES_KEY: u64 = 255;
pub const MT7996_SEC_ENCRYPT: u64 = 1;
pub const MT7996_SEC_KEY_IDX: u64 = 6;
pub const MT7996_SEC_IV: u64 = 8;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_background_chain_ctrl {
    pub _rsv: [u8; 4],
    pub tag: Le16,
    pub len: Le16,
    pub chan: u8,
    pub central_chan: u8,
    pub bw: u8,
    pub tx_stream: u8,
    pub rx_stream: u8,
    pub monitor_chan: u8,
    pub monitor_central_chan: u8,
    pub monitor_bw: u8,
    pub monitor_tx_stream: u8,
    pub monitor_rx_stream: u8,
    pub scan_mode: u8,
    pub band_idx: u8,
    pub monitor_scan_type: u8,
    pub band: u8,
    pub rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_eeprom_update {
    pub _rsv: [u8; 4],
    pub tag: Le16,
    pub len: Le16,
    pub buffer_mode: u8,
    pub format: u8,
    pub buf_len: Le16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_eeprom_info {
    pub _rsv: [u8; 4],
    pub tag: Le16,
    pub len: Le16,
    pub addr: Le32,
    pub valid: Le32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_phy_rx_info {
    pub category: u8,
    pub rate: u8,
    pub mode: u8,
    pub nsts: u8,
    pub gi: u8,
    pub coding: u8,
    pub stbc: u8,
    pub bw: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_mib {
    pub tag: Le16,
    pub len: Le16,
    pub offs: Le32,
    pub data: Le64,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct all_sta_trx_rate {
    pub wlan_idx: Le16,
    pub __rsv1: [u8; 2],
    pub tx_mode: u8,
    pub flags: u8,
    pub tx_stbc: u8,
    pub tx_gi: u8,
    pub tx_bw: u8,
    pub tx_ldpc: u8,
    pub tx_mcs: u8,
    pub tx_nss: u8,
    pub rx_rate: u8,
    pub rx_mode: u8,
    pub rx_nsts: u8,
    pub rx_gi: u8,
    pub rx_coding: u8,
    pub rx_stbc: u8,
    pub rx_bw: u8,
    pub __rsv2: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_wed_rro_event {
    pub rxd: mt7996_mcu_rxd,
    pub __rsv1: [u8; 4],
    pub tag: Le16,
    pub len: Le16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_wed_rro_ba_event {
    pub tag: Le16,
    pub len: Le16,
    pub wlan_id: Le16,
    pub tid: u8,
    pub __rsv1: u8,
    pub status: Le32,
    pub id: Le16,
    pub __rsv2: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_wed_rro_ba_delete_event {
    pub tag: Le16,
    pub len: Le16,
    pub session_id: Le16,
    pub wlan_id: Le16,
    pub tid: u8,
    pub __rsv1: u8,
    pub __rsv2: [u8; 2],
}

pub const UNI_WED_RRO_BA_SESSION_STATUS: u64 = 0;
pub const UNI_WED_RRO_BA_SESSION_TBL: u64 = 1;
pub const UNI_WED_RRO_BA_SESSION_DELETE: u64 = 2;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_ps_sync_event {
    pub rxd: mt7996_mcu_rxd,
    pub bss_idx: u8,
    pub __rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_ps_client_info {
    pub tag: Le16,
    pub len: Le16,
    pub ps_bit: u8,
    pub __rsv: u8,
    pub wlan_idx: Le16,
    pub buffer_size: u8,
    pub __rsv2: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_ps_multi_client_info {
    pub tag: Le16,
    pub len: Le16,
    pub sta_cnt: Le16,
}
pub const mt7996_mcu_ps_multi_client_info_FLEX: usize = core::mem::size_of::<Le16>();

pub const UNI_PS_CLIENT_INFO: u64 = 0;
pub const UNI_PS_MULTI_CLIENT_INFO: u64 = 1;
pub const UNI_PS_MULTI_CLIENT_INFO_BITMAP: u64 = 2;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7996_mcu_thermal_notify {
    pub rxd: mt7996_mcu_rxd,
    pub __rsv1: [u8; 4],
    pub tag: Le16,
    pub len: Le16,
    pub event_id: u8,
    pub band_idx: u8,
    pub level_idx: u8,
    pub duty_percent: u8,
    pub restore_temp: Le32,
    pub __rsv2: [u8; 4],
}

pub const UNI_MIB_OBSS_AIRTIME: u64 = 26;
pub const UNI_MIB_NON_WIFI_TIME: u64 = 27;
pub const UNI_MIB_TX_TIME: u64 = 28;
pub const UNI_MIB_RX_TIME: u64 = 29;
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

pub const MCU_FW_LOG_WM: u64 = 0;
pub const MCU_FW_LOG_WA: u64 = 1;
pub const MCU_FW_LOG_TO_HOST: u64 = 2;
pub const MCU_FW_LOG_RELAY: u64 = 16;
pub const MCU_TWT_AGRT_ADD: u64 = 0;
pub const MCU_TWT_AGRT_MODIFY: u64 = 1;
pub const MCU_TWT_AGRT_DELETE: u64 = 2;
pub const MCU_TWT_AGRT_TEARDOWN: u64 = 3;
pub const MCU_TWT_AGRT_GET_TSF: u64 = 4;
pub const MCU_WA_PARAM_CMD_QUERY: u64 = 0;
pub const MCU_WA_PARAM_CMD_SET: u64 = 1;
pub const MCU_WA_PARAM_CMD_CAPABILITY: u64 = 2;
pub const MCU_WA_PARAM_CMD_DEBUG: u64 = 3;
pub const MCU_WA_PARAM_PDMA_RX: u64 = 4;
pub const MCU_WA_PARAM_CPU_UTIL: u64 = 11;
pub const MCU_WA_PARAM_RED: u64 = 14;
pub const MCU_WA_PARAM_HW_PATH_HIF_VER: u64 = 47;
pub const MCU_MMPS_STATIC: u64 = 0;
pub const MCU_MMPS_DYNAMIC: u64 = 1;
pub const MCU_MMPS_RSV: u64 = 2;
pub const MCU_MMPS_DISABLE: u64 = 3;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_rate_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub __rsv1: [u8; 4],
    pub bc_trans: Le16,
    pub mc_trans: Le16,
    pub short_preamble: u8,
    pub bc_fixed_rate: u8,
    pub mc_fixed_rate: u8,
    pub __rsv2: [u8; 9],
}

pub const BP_DISABLE: u64 = 0;
pub const BP_SW_MODE: u64 = 1;
pub const BP_HW_MODE: u64 = 2;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_ra_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub short_preamble: u8,
    pub force_sgi: u8,
    pub force_gf: u8,
    pub ht_mode: u8,
    pub se_off: u8,
    pub antenna_idx: u8,
    pub max_phyrate: Le16,
    pub force_tx_streams: u8,
    pub __rsv: [u8; 3],
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
    pub __rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_color_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub enable: u8,
    pub color: u8,
    pub rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_inband_discovery_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub tx_type: u8,
    pub tx_mode: u8,
    pub tx_interval: u8,
    pub enable: u8,
    pub wcid: Le16,
    pub prob_rsp_len: Le16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_bcn_content_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub tim_ie_pos: Le16,
    pub csa_ie_pos: Le16,
    pub bcc_ie_pos: Le16,
    pub enable: u8,
    pub r#type: u8,
    pub pkt_len: Le16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct bss_bcn_cntdwn_tlv_anon_3_csa {
    pub static_pp: u8,
    pub abort: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct bss_bcn_cntdwn_tlv_anon_3_cca {
    pub abort: u8,
}

#[allow(missing_debug_implementations)] // union fields share one store
#[repr(C)]
#[derive(Clone, Copy)]
pub union bss_bcn_cntdwn_tlv_anon_3 {
    pub csa: bss_bcn_cntdwn_tlv_anon_3_csa,
    pub cca: bss_bcn_cntdwn_tlv_anon_3_cca,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_bcn_cntdwn_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub cnt: u8,
    pub bss_bcn_cntdwn_tlv_anon_3: bss_bcn_cntdwn_tlv_anon_3,
    pub rsv: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct bss_bcn_mbss_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub bitmap: Le32,
    pub offset: [Le16; 32],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_txcmd_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub txcmd_mode: u8,
    pub __rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_sec_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub __rsv1: [u8; 2],
    pub cipher: u8,
    pub __rsv2: [u8; 1],
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
pub struct bss_power_save {
    pub tag: Le16,
    pub len: Le16,
    pub profile: u8,
    pub _rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_prot_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub prot_mode: Le32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_ht_uni {
    pub tag: Le16,
    pub len: Le16,
    pub ht_cap: Le16,
    pub ht_cap_ext: Le16,
    pub ampdu_param: u8,
    pub _rsv: [u8; 3],
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
pub struct sec_key_uni {
    pub wlan_idx: Le16,
    pub mgmt_prot: u8,
    pub cipher_id: u8,
    pub cipher_len: u8,
    pub key_id: u8,
    pub key_len: u8,
    pub need_resp: u8,
    pub key: [u8; 32],
    pub pn: [u8; 6],
    pub bcn_mode: u8,
    pub _rsv: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_sec_uni {
    pub tag: Le16,
    pub len: Le16,
    pub add: u8,
    pub n_cipher: u8,
    pub rsv: [u8; 2],
    pub key: [sec_key_uni; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sta_phy_uni {
    pub r#type: u8,
    pub flag: u8,
    pub stbc: u8,
    pub sgi: u8,
    pub bw: u8,
    pub ldpc: u8,
    pub mcs: u8,
    pub nss: u8,
    pub he_ltf: u8,
    pub rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_ra_uni {
    pub tag: Le16,
    pub len: Le16,
    pub valid: u8,
    pub auto_rate: u8,
    pub phy_mode: u8,
    pub channel: u8,
    pub bw: u8,
    pub disable_cck: u8,
    pub ht_mcs32: u8,
    pub ht_gf: u8,
    pub ht_mcs: [u8; 4],
    pub mmps_mode: u8,
    pub gband_256: u8,
    pub af: u8,
    pub auth_wapi_mode: u8,
    pub rate_len: u8,
    pub supp_mode: u8,
    pub supp_cck_rate: u8,
    pub supp_ofdm_rate: u8,
    pub supp_ht_mcs: Le32,
    pub supp_vht_mcs: [Le16; 4],
    pub op_mode: u8,
    pub op_vht_chan_width: u8,
    pub op_vht_rx_nss: u8,
    pub op_vht_rx_nss_type: u8,
    pub sta_cap: Le32,
    pub phy: sta_phy_uni,
    pub rx_rcpi: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_ra_fixed_uni {
    pub tag: Le16,
    pub len: Le16,
    pub field: Le32,
    pub op_mode: u8,
    pub op_vht_chan_width: u8,
    pub op_vht_rx_nss: u8,
    pub op_vht_rx_nss_type: u8,
    pub phy: sta_phy_uni,
    pub spe_idx: u8,
    pub short_preamble: u8,
    pub is_5g: u8,
    pub mmps_mode: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_hdrt {
    pub tag: Le16,
    pub len: Le16,
    pub hdrt_mode: u8,
    pub rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_hdr_trans {
    pub tag: Le16,
    pub len: Le16,
    pub from_ds: u8,
    pub to_ds: u8,
    pub dis_rx_hdr_tran: u8,
    pub mesh: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_ps_leave {
    pub tag: Le16,
    pub len: Le16,
    pub __rsv: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_eht_mld {
    pub tag: Le16,
    pub len: Le16,
    pub nsep: u8,
    pub __rsv1: [u8; 2],
    pub str_cap: [u8; 3],
    pub eml_cap: Le16,
    pub __rsv2: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mld_setup_link {
    pub wcid: Le16,
    pub bss_idx: u8,
    pub __rsv: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mld_reconf_stop_link {
    pub tag: Le16,
    pub len: Le16,
    pub link_bitmap: Le16,
    pub rsv: [u8; 2],
    pub bss_idx: [u8; 16],
}

pub const UNI_CMD_MLD_RECONF_AP_REM_TIMER: u64 = 3;
pub const UNI_CMD_MLD_RECONF_STOP_LINK: u64 = 4;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct hdr_trans_en {
    pub tag: Le16,
    pub len: Le16,
    pub enable: u8,
    pub check_bssid: u8,
    pub mode: u8,
    pub __rsv: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct hdr_trans_vlan {
    pub tag: Le16,
    pub len: Le16,
    pub insert_vlan: u8,
    pub remove_vlan: u8,
    pub tid: u8,
    pub __rsv: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct hdr_trans_blacklist {
    pub tag: Le16,
    pub len: Le16,
    pub idx: u8,
    pub enable: u8,
    pub r#type: Le16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct uni_header {
    pub __rsv: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct vow_rx_airtime {
    pub tag: Le16,
    pub len: Le16,
    pub enable: u8,
    pub band: u8,
    pub __rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bf_sounding_on {
    pub tag: Le16,
    pub len: Le16,
    pub snd_mode: u8,
    pub sta_num: u8,
    pub __rsv: [u8; 2],
    pub wlan_id: [Le16; 4],
    pub snd_period: Le32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bf_hw_en_status_update {
    pub tag: Le16,
    pub len: Le16,
    pub ebf: u8,
    pub ibf: u8,
    pub __rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bf_mod_en_ctrl {
    pub tag: Le16,
    pub len: Le16,
    pub bf_num: u8,
    pub bf_bitmap: u8,
    pub bf_sel: [u8; 8],
    pub __rsv: [u8; 2],
}

#[allow(missing_debug_implementations)] // union fields share one store
#[repr(C)]
#[derive(Clone, Copy)]
pub union bf_tag_tlv {
    pub bf_snd: bf_sounding_on,
    pub bf_hw_en: bf_hw_en_status_update,
    pub bf_mod_en: bf_mod_en_ctrl,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ra_rate {
    pub wlan_idx: Le16,
    pub mode: u8,
    pub stbc: u8,
    pub gi: Le16,
    pub bw: u8,
    pub ldpc: u8,
    pub mcs: u8,
    pub nss: u8,
    pub ltf: Le16,
    pub spe: u8,
    pub preamble: u8,
    pub __rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ra_fixed_rate {
    pub tag: Le16,
    pub len: Le16,
    pub version: Le16,
    pub rate: ra_rate,
}

pub const UNI_RA_FIXED_RATE: u64 = 15;
pub const UNI_HDR_TRANS_EN: u64 = 0;
pub const UNI_HDR_TRANS_VLAN: u64 = 1;
pub const UNI_HDR_TRANS_BLACKLIST: u64 = 2;
pub const RATE_PARAM_FIXED: u64 = 3;
pub const RATE_PARAM_MMPS_UPDATE: u64 = 5;
pub const RATE_PARAM_FIXED_HE_LTF: u64 = 7;
pub const RATE_PARAM_FIXED_MCS: u64 = 8;
pub const RATE_PARAM_FIXED_GI: u64 = 11;
pub const RATE_PARAM_AUTO: u64 = 20;
pub const BF_SOUNDING_ON: u64 = 1;
pub const BF_HW_EN_UPDATE: u64 = 17;
pub const BF_MOD_EN_CTRL: u64 = 20;
pub const CMD_BAND_NONE: u64 = 0;
pub const CMD_BAND_24G: u64 = 1;
pub const CMD_BAND_5G: u64 = 2;
pub const CMD_BAND_6G: u64 = 3;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct bss_req_hdr {
    pub bss_idx: u8,
    pub __rsv: [u8; 3],
}

pub const UNI_CHANNEL_SWITCH: u64 = 0;
pub const UNI_CHANNEL_RX_PATH: u64 = 1;
pub const UNI_CHIP_CONFIG_NIC_CAPA: u64 = 3;
pub const UNI_CHIP_CONFIG_DUP_WTBL: u64 = 4;
pub const UNI_BAND_CONFIG_RADIO_ENABLE: u64 = 0;
pub const UNI_BAND_CONFIG_RTS_THRESHOLD: u64 = 8;
pub const UNI_BAND_CONFIG_MAC_ENABLE_CTRL: u64 = 12;
pub const UNI_BAND_CONFIG_BSSID_MAPPING_ADDR: u64 = 18;
pub const UNI_WSYS_CONFIG_FW_LOG_CTRL: u64 = 0;
pub const UNI_WSYS_CONFIG_FW_DBG_CTRL: u64 = 1;
pub const UNI_RDD_CTRL_PARM: u64 = 0;
pub const UNI_RDD_CTRL_SET_TH: u64 = 3;
pub const UNI_EFUSE_ACCESS: u64 = 1;
pub const UNI_EFUSE_BUFFER_MODE: u64 = 2;
pub const UNI_EFUSE_FREE_BLOCK: u64 = 3;
pub const UNI_EFUSE_BUFFER_RD: u64 = 4;
pub const UNI_EXT_EEPROM_ACCESS: u64 = 1;
pub const UNI_VOW_DRR_CTRL: u64 = 0;
pub const UNI_VOW_RX_AT_AIRTIME_EN: u64 = 11;
pub const UNI_VOW_RX_AT_AIRTIME_CLR_EN: u64 = 14;
pub const UNI_CMD_MIB_DATA: u64 = 0;
pub const UNI_POWER_OFF: u64 = 0;
pub const UNI_CMD_TWT_ARGT_UPDATE: u64 = 0;
pub const UNI_CMD_TWT_MGMT_OFFLOAD: u64 = 1;
pub const UNI_RRO_DEL_ENTRY: u64 = 1;
pub const UNI_RRO_SET_PLATFORM_TYPE: u64 = 2;
pub const UNI_RRO_GET_BA_SESSION_TABLE: u64 = 3;
pub const UNI_RRO_SET_BYPASS_MODE: u64 = 4;
pub const UNI_RRO_SET_TXFREE_PATH: u64 = 5;
pub const UNI_RRO_DEL_BA_SESSION: u64 = 6;
pub const UNI_RRO_SET_FLUSH_TIMEOUT: u64 = 7;
pub const UNI_CMD_SR_ENABLE: u64 = 1;
pub const UNI_CMD_SR_ENABLE_SD: u64 = 2;
pub const UNI_CMD_SR_ENABLE_MODE: u64 = 3;
pub const UNI_CMD_SR_ENABLE_DPD: u64 = 18;
pub const UNI_CMD_SR_ENABLE_TX: u64 = 19;
pub const UNI_CMD_SR_SET_SRG_BITMAP: u64 = 128;
pub const UNI_CMD_SR_SET_PARAM: u64 = 193;
pub const UNI_CMD_SR_SET_SIGA: u64 = 208;
pub const UNI_CMD_THERMAL_PROTECT_ENABLE: u64 = 6;
pub const UNI_CMD_THERMAL_PROTECT_DISABLE: u64 = 7;
pub const UNI_CMD_THERMAL_PROTECT_DUTY_CONFIG: u64 = 8;
pub const UNI_TXPOWER_POWER_LIMIT_TABLE_CTRL: u64 = 4;
pub const UNI_CMD_ACCESS_REG_BASIC: u64 = 0;
pub const UNI_CMD_ACCESS_RF_REG_BASIC: u64 = 1;
pub const UNI_CMD_SER_QUERY: u64 = 0;
pub const UNI_CMD_SER_SET_RECOVER_L1: u64 = 1;
pub const UNI_CMD_SER_SET_RECOVER_L2: u64 = 2;
pub const UNI_CMD_SER_SET_RECOVER_L3_RX_ABORT: u64 = 3;
pub const UNI_CMD_SER_SET_RECOVER_L3_TX_ABORT: u64 = 4;
pub const UNI_CMD_SER_SET_RECOVER_L3_TX_DISABLE: u64 = 5;
pub const UNI_CMD_SER_SET_RECOVER_L3_BF: u64 = 6;
pub const UNI_CMD_SER_SET_RECOVER_L4_MDP: u64 = 7;
pub const UNI_CMD_SER_SET_RECOVER_FROM_ETH: u64 = 8;
pub const UNI_CMD_SER_SET_RECOVER_FULL: u64 = 8;
pub const UNI_CMD_SER_SET_SYSTEM_ASSERT: u64 = 9;
pub const UNI_CMD_SER_ENABLE: u64 = 1;
pub const UNI_CMD_SER_SET: u64 = 2;
pub const UNI_CMD_SER_TRIGGER: u64 = 3;
pub const UNI_CMD_SDO_SET: u64 = 1;
pub const UNI_CMD_SDO_QUERY: u64 = 2;
pub const UNI_CMD_SDO_CP_MODE: u64 = 6;
pub const MT7996_SEC_MODE_PLAIN: u64 = 0;
pub const MT7996_SEC_MODE_AES: u64 = 1;
pub const MT7996_SEC_MODE_SCRAMBLE: u64 = 2;
pub const MT7996_SEC_MODE_MAX: u64 = 3;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct fixed_rate_table_ctrl {
    pub _rsv: [u8; 4],
    pub tag: Le16,
    pub len: Le16,
    pub table_idx: u8,
    pub antenna_idx: u8,
    pub rate_idx: Le16,
    pub spe_idx_sel: u8,
    pub spe_idx: u8,
    pub gi: u8,
    pub he_ltf: u8,
    pub ldpc: u8,
    pub txbf: u8,
    pub dynamic_bw: u8,
    pub _rsv2: u8,
}
