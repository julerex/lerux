//! `mt7915/mcu.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MCU_ATE_SET_TRX: u64 = 1;
pub const MCU_ATE_SET_FREQ_OFFSET: u64 = 10;
pub const MCU_ATE_SET_SLOT_TIME: u64 = 19;
pub const MCU_ATE_CLEAN_TXQUEUE: u64 = 28;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_mcu_thermal_ctrl_anon_2_type {
    pub protect_type: u8,
    pub trigger_type: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_mcu_thermal_ctrl_anon_2_duty {
    pub duty_level: u8,
    pub duty_cycle: u8,
}

#[allow(missing_debug_implementations)] // union fields share one store
#[repr(C)]
#[derive(Clone, Copy)]
pub union mt7915_mcu_thermal_ctrl_anon_2 {
    pub r#type: mt7915_mcu_thermal_ctrl_anon_2_type,
    pub duty: mt7915_mcu_thermal_ctrl_anon_2_duty,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_mcu_thermal_ctrl {
    pub ctrl_id: u8,
    pub band_idx: u8,
    pub mt7915_mcu_thermal_ctrl_anon_2: mt7915_mcu_thermal_ctrl_anon_2,
}

pub const WMM_AIFS_SET: u64 = 1;
pub const WMM_CW_MIN_SET: u64 = 2;
pub const WMM_CW_MAX_SET: u64 = 4;
pub const WMM_TXOP_SET: u64 = 8;
pub const WMM_PARAM_SET: u64 = 15;
pub const MAX_BEACON_NUM: u64 = 32;
pub const RATE_CFG_MCS: u64 = 15;
pub const RATE_CFG_NSS: u64 = 240;
pub const RATE_CFG_GI: u64 = 0xf00;
pub const RATE_CFG_BW: u64 = 0xf000;
pub const RATE_CFG_STBC: u64 = 0xf_0000;
pub const RATE_CFG_LDPC: u64 = 0xf0_0000;
pub const RATE_CFG_PHY_TYPE: u64 = 0xf00_0000;
pub const RATE_CFG_HE_LTF: u64 = 0xf000_0000;
pub const MT7915_MAX_BEACON_SIZE: u64 = 0x51c;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_mcu_background_chain_ctrl {
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
pub struct mt7915_mcu_sr_ctrl {
    pub action: u8,
    pub argnum: u8,
    pub band_idx: u8,
    pub status: u8,
    pub drop_ta_idx: u8,
    pub sta_idx: u8,
    pub rsv: [u8; 2],
    pub val: Le32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7915_mcu_eeprom {
    pub buffer_mode: u8,
    pub format: u8,
    pub len: Le16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt7915_mcu_eeprom_info {
    pub addr: Le32,
    pub valid: Le32,
    pub data: [u8; 16],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_mcu_phy_rx_info {
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
pub struct mt7915_mcu_mib {
    pub band: Le32,
    pub offs: Le32,
    pub data: Le64,
}

pub const MIB_TX_TIME: u64 = 81;
pub const MIB_RX_TIME: u64 = 82;
pub const MIB_OBSS_AIRTIME: u64 = 86;
pub const MIB_NON_WIFI_TIME: u64 = 87;
pub const MIB_TXOP_INIT_COUNT: u64 = 88;
pub const MIB_TX_TIME_V2: u64 = 6;
pub const MIB_RX_TIME_V2: u64 = 8;
pub const MIB_OBSS_AIRTIME_V2: u64 = 490;
pub const MIB_NON_WIFI_TIME_V2: u64 = 491;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_mcu_txpower_sku {
    pub format_id: u8,
    pub limit_type: u8,
    pub band_idx: u8,
    pub txpower_sku: [i8; 161],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct edca {
    pub queue: u8,
    pub set: u8,
    pub aifs: u8,
    pub cw_min: u8,
    pub cw_max: Le16,
    pub txop: Le16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_mcu_muru_stats_dl {
    pub cck_cnt: Le32,
    pub ofdm_cnt: Le32,
    pub htmix_cnt: Le32,
    pub htgf_cnt: Le32,
    pub vht_su_cnt: Le32,
    pub vht_2mu_cnt: Le32,
    pub vht_3mu_cnt: Le32,
    pub vht_4mu_cnt: Le32,
    pub he_su_cnt: Le32,
    pub he_ext_su_cnt: Le32,
    pub he_2ru_cnt: Le32,
    pub he_2mu_cnt: Le32,
    pub he_3ru_cnt: Le32,
    pub he_3mu_cnt: Le32,
    pub he_4ru_cnt: Le32,
    pub he_4mu_cnt: Le32,
    pub he_5to8ru_cnt: Le32,
    pub he_9to16ru_cnt: Le32,
    pub he_gtr16ru_cnt: Le32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_mcu_muru_stats_ul {
    pub hetrig_su_cnt: Le32,
    pub hetrig_2ru_cnt: Le32,
    pub hetrig_3ru_cnt: Le32,
    pub hetrig_4ru_cnt: Le32,
    pub hetrig_5to8ru_cnt: Le32,
    pub hetrig_9to16ru_cnt: Le32,
    pub hetrig_gtr16ru_cnt: Le32,
    pub hetrig_2mu_cnt: Le32,
    pub hetrig_3mu_cnt: Le32,
    pub hetrig_4mu_cnt: Le32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt7915_mcu_muru_stats {
    pub event_id: Le32,
    pub dl: mt7915_mcu_muru_stats_dl,
    pub ul: mt7915_mcu_muru_stats_ul,
}

pub const MCU_FW_LOG_WM: u64 = 0;
pub const MCU_FW_LOG_WA: u64 = 1;
pub const MCU_FW_LOG_TO_HOST: u64 = 2;
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
pub const MCU_WA_PARAM_RED_SETTING: u64 = 64;
pub const MCU_MMPS_STATIC: u64 = 0;
pub const MCU_MMPS_DYNAMIC: u64 = 1;
pub const MCU_MMPS_RSV: u64 = 2;
pub const MCU_MMPS_DISABLE: u64 = 3;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_info_bmc_rate {
    pub tag: Le16,
    pub len: Le16,
    pub bc_trans: Le16,
    pub mc_trans: Le16,
    pub short_preamble: u8,
    pub rsv: [u8; 7],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_info_ra {
    pub tag: Le16,
    pub len: Le16,
    pub op_mode: u8,
    pub adhoc_en: u8,
    pub short_preamble: u8,
    pub tx_streams: u8,
    pub rx_streams: u8,
    pub algo: u8,
    pub force_sgi: u8,
    pub force_gf: u8,
    pub ht_mode: u8,
    pub has_20_sta: u8,
    pub bss_width_trigger_events: u8,
    pub vht_nss_cap: u8,
    pub vht_bw_signal: u8,
    pub vht_force_sgi: u8,
    pub se_off: u8,
    pub antenna_idx: u8,
    pub train_up_rule: u8,
    pub rsv: [u8; 3],
    pub train_up_high_thres: Le16,
    pub train_up_rule_rssi: Le16,
    pub low_traffic_thres: Le16,
    pub max_phyrate: Le16,
    pub phy_cap: Le32,
    pub interval: Le32,
    pub fast_interval: Le32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_info_hw_amsdu {
    pub tag: Le16,
    pub len: Le16,
    pub cmp_bitmap_0: Le32,
    pub cmp_bitmap_1: Le32,
    pub trig_thres: Le16,
    pub enable: u8,
    pub rsv: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_info_color {
    pub tag: Le16,
    pub len: Le16,
    pub disable: u8,
    pub color: u8,
    pub rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_info_he {
    pub tag: Le16,
    pub len: Le16,
    pub he_pe_duration: u8,
    pub vht_op_info_present: u8,
    pub he_rts_thres: Le16,
    pub max_nss_mcs: [Le16; 3],
    pub rsv: [u8; 6],
}

#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct bss_info_bcn {
    pub tag: Le16,
    pub len: Le16,
    pub ver: u8,
    pub enable: u8,
    pub sub_ntlv: Le16,
}

#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct bss_info_bcn_cntdwn {
    pub tag: Le16,
    pub len: Le16,
    pub cnt: u8,
    pub rsv: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct bss_info_bcn_mbss {
    pub tag: Le16,
    pub len: Le16,
    pub bitmap: Le32,
    pub offset: [Le16; 32],
    pub rsv: [u8; 8],
}

#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct bss_info_bcn_cont {
    pub tag: Le16,
    pub len: Le16,
    pub tim_ofs: Le16,
    pub csa_ofs: Le16,
    pub bcc_ofs: Le16,
    pub pkt_len: Le16,
}

#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct bss_info_inband_discovery {
    pub tag: Le16,
    pub len: Le16,
    pub tx_type: u8,
    pub tx_mode: u8,
    pub tx_interval: u8,
    pub enable: u8,
    pub rsv: Le16,
    pub prob_rsp_len: Le16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_info_prot {
    pub tag: Le16,
    pub len: Le16,
    pub prot_type: Le32,
    pub prot_mode: Le32,
    pub rts_len_thres: Le32,
    pub he_rts_thres: Le16,
    pub rts_pkt_thres: u8,
    pub rsv: [u8; 5],
}

pub const BSS_INFO_BCN_CSA: u64 = 0;
pub const BSS_INFO_BCN_BCC: u64 = 1;
pub const BSS_INFO_BCN_MBSSID: u64 = 2;
pub const BSS_INFO_BCN_CONTENT: u64 = 3;
pub const BSS_INFO_BCN_DISCOV: u64 = 4;
pub const BSS_INFO_BCN_MAX: u64 = 5;
pub const RATE_PARAM_FIXED: u64 = 3;
pub const RATE_PARAM_MMPS_UPDATE: u64 = 5;
pub const RATE_PARAM_FIXED_HE_LTF: u64 = 7;
pub const RATE_PARAM_FIXED_MCS: u64 = 8;
pub const RATE_PARAM_FIXED_GI: u64 = 11;
pub const RATE_PARAM_AUTO: u64 = 20;
pub const RATE_PARAM_SPE_UPDATE: u64 = 22;
pub const TX_POWER_LIMIT_ENABLE: u64 = 0;
pub const TX_POWER_LIMIT_PATH_ENABLE: u64 = 3;
pub const TX_POWER_LIMIT_TABLE: u64 = 4;
pub const TX_POWER_LIMIT_INFO: u64 = 7;
pub const TX_POWER_LIMIT_FRAME: u64 = 17;
pub const TX_POWER_LIMIT_FRAME_MIN: u64 = 18;
pub const TX_POWER_INFO_PATH: u64 = 1;
pub const TX_POWER_INFO_RATE: u64 = 2;
pub const SPR_ENABLE: u64 = 1;
pub const SPR_ENABLE_SD: u64 = 3;
pub const SPR_ENABLE_MODE: u64 = 5;
pub const SPR_ENABLE_DPD: u64 = 35;
pub const SPR_ENABLE_TX: u64 = 37;
pub const SPR_SET_SRG_BITMAP: u64 = 128;
pub const SPR_SET_PARAM: u64 = 194;
pub const SPR_SET_SIGA: u64 = 220;
pub const THERMAL_PROTECT_PARAMETER_CTRL: u64 = 0;
pub const THERMAL_PROTECT_BASIC_INFO: u64 = 1;
pub const THERMAL_PROTECT_ENABLE: u64 = 2;
pub const THERMAL_PROTECT_DISABLE: u64 = 3;
pub const THERMAL_PROTECT_DUTY_CONFIG: u64 = 4;
pub const THERMAL_PROTECT_MECH_INFO: u64 = 5;
pub const THERMAL_PROTECT_DUTY_INFO: u64 = 6;
pub const THERMAL_PROTECT_STATE_ACT: u64 = 7;
pub const MT_BF_SOUNDING_ON: u64 = 1;
pub const MT_BF_TYPE_UPDATE: u64 = 20;
pub const MT_BF_MODULE_UPDATE: u64 = 25;
pub const MURU_SET_ARB_OP_MODE: u64 = 14;
pub const MURU_SET_PLATFORM_TYPE: u64 = 25;
pub const MURU_PLATFORM_TYPE_PERF_LEVEL_1: u64 = 1;
pub const MURU_PLATFORM_TYPE_PERF_LEVEL_2: u64 = 2;
pub const MURU_SET_TXC_TX_STATS_EN: u64 = 150;
pub const MURU_GET_TXC_TX_STATS: u64 = 151;
pub const SER_QUERY: u64 = 0;
pub const SER_SET_RECOVER_L1: u64 = 1;
pub const SER_SET_RECOVER_L2: u64 = 2;
pub const SER_SET_RECOVER_L3_RX_ABORT: u64 = 3;
pub const SER_SET_RECOVER_L3_TX_ABORT: u64 = 4;
pub const SER_SET_RECOVER_L3_TX_DISABLE: u64 = 5;
pub const SER_SET_RECOVER_L3_BF: u64 = 6;
pub const SER_SET_RECOVER_FULL: u64 = 7;
pub const SER_SET_SYSTEM_ASSERT: u64 = 8;
pub const SER_ENABLE: u64 = 2;
pub const SER_RECOVER: u64 = 3;
