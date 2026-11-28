//! `mt76_connac_mcu.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const FW_FEATURE_SET_ENCRYPT: u64 = 1;
pub const FW_FEATURE_SET_KEY_IDX: u64 = 6;
pub const FW_FEATURE_ENCRY_MODE: u64 = 16;
pub const FW_FEATURE_OVERRIDE_ADDR: u64 = 32;
pub const FW_FEATURE_NON_DL: u64 = 64;
pub const DL_MODE_ENCRYPT: u64 = 1;
pub const DL_MODE_KEY_IDX: u64 = 6;
pub const DL_MODE_RESET_SEC_IV: u64 = 8;
pub const DL_MODE_WORKING_PDA_CR4: u64 = 16;
pub const DL_MODE_VALID_RAM_ENTRY: u64 = 32;
pub const DL_CONFIG_ENCRY_MODE_SEL: u64 = 64;
pub const DL_MODE_NEED_RSP: u64 = 0x8000_0000;
pub const FW_START_OVERRIDE: u64 = 1;
pub const FW_START_WORKING_PDA_CR4: u64 = 4;
pub const FW_START_WORKING_PDA_DSP: u64 = 8;
pub const PATCH_SEC_NOT_SUPPORT: u64 = 0xffff_ffff;
pub const PATCH_SEC_TYPE_MASK: u64 = 0xffff;
pub const PATCH_SEC_TYPE_INFO: u64 = 2;
pub const PATCH_SEC_ENC_TYPE_MASK: u64 = 0xff00_0000;
pub const PATCH_SEC_ENC_TYPE_PLAIN: u64 = 0;
pub const PATCH_SEC_ENC_TYPE_AES: u64 = 1;
pub const PATCH_SEC_ENC_TYPE_SCRAMBLE: u64 = 2;
pub const PATCH_SEC_ENC_SCRAMBLE_INFO_MASK: u64 = 0xffff;
pub const PATCH_SEC_ENC_AES_KEY_MASK: u64 = 255;
pub const SEC_TYPE_SUBSYS_MASK: u64 = 0xff_0000;
pub const SEC_TYPE_SUBSYS_CBMCU: u64 = 0x1_0000;
pub const SEC_TYPE_SUBSYS_SEC_MASK: u64 = 0xffff;
pub const SEC_TYPE_SUBSYS_SEC_IMG_SIGN: u64 = 5;
#[must_use]
#[rustfmt::skip]
pub const fn MCU_PQ_ID(p: u64, q: u64) -> u64 { (shl(p, 15u64)) | (shl(q, 10u64)) }

pub const MCU_PKT_ID: u64 = 160;
pub const FW_TYPE_DEFAULT: u64 = 0;
pub const FW_TYPE_CLC: u64 = 2;
pub const FW_TYPE_MAX_NUM: u64 = 255;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac2_mcu_txd {
    pub txd: [Le32; 8],
    pub len: Le16,
    pub pq_id: Le16,
    pub cid: u8,
    pub pkt_type: u8,
    pub set_query: u8,
    pub seq: u8,
    pub uc_d2b0_rev: u8,
    pub ext_cid: u8,
    pub s2d_index: u8,
    pub ext_cid_ack: u8,
    pub rsv: [u32; 5],
}

pub const EXTRA_INFO_VER: u64 = 1;
pub const EXTRA_INFO_NEW: u64 = 2;
pub const RA_LEGACY_OFDM: u64 = 0x3fc0;
pub const RA_LEGACY_CCK: u64 = 15;
pub const HT_MCS_MASK_NUM: u64 = 10;
pub const STA_TYPE_STA: u64 = 1;
pub const STA_TYPE_AP: u64 = 2;
pub const STA_TYPE_ADHOC: u64 = 4;
pub const STA_TYPE_WDS: u64 = 16;
pub const STA_TYPE_BC: u64 = 32;
pub const NETWORK_INFRA: u64 = 0x1_0000;
pub const NETWORK_P2P: u64 = 0x2_0000;
pub const NETWORK_IBSS: u64 = 0x4_0000;
pub const NETWORK_WDS: u64 = 0x20_0000;
pub const NETWORK_NAN: u64 = 0x40_0000;
pub const SCAN_FUNC_RANDOM_MAC: u64 = 1;
pub const SCAN_FUNC_RNR_SCAN: u64 = 8;
pub const SCAN_FUNC_SPLIT_SCAN: u64 = 32;
pub const CONNECTION_INFRA_STA: u64 = 0x1_0001;
pub const CONNECTION_INFRA_AP: u64 = 0x1_0002;
pub const CONNECTION_P2P_GC: u64 = 0x2_0001;
pub const CONNECTION_P2P_GO: u64 = 0x2_0002;
pub const CONNECTION_IBSS_ADHOC: u64 = 0x4_0004;
pub const CONNECTION_WDS: u64 = 0x20_0010;
pub const CONNECTION_INFRA_BC: u64 = 0x1_0020;
pub const CONNECTION_NAN: u64 = 0x40_0000;
pub const CONN_STATE_DISCONNECT: u64 = 0;
pub const CONN_STATE_CONNECT: u64 = 1;
pub const CONN_STATE_PORT_SECURE: u64 = 2;
pub const STA_REC_HE_CAP_HTC: u64 = 1;
pub const STA_REC_HE_CAP_BQR: u64 = 2;
pub const STA_REC_HE_CAP_BSR: u64 = 4;
pub const STA_REC_HE_CAP_OM: u64 = 8;
pub const STA_REC_HE_CAP_AMSDU_IN_AMPDU: u64 = 16;
pub const STA_REC_HE_CAP_DUAL_BAND: u64 = 32;
pub const STA_REC_HE_CAP_LDPC: u64 = 64;
pub const STA_REC_HE_CAP_TRIG_CQI_FK: u64 = 128;
pub const STA_REC_HE_CAP_PARTIAL_BW_EXT_RANGE: u64 = 256;
pub const STA_REC_HE_CAP_LE_EQ_80M_TX_STBC: u64 = 512;
pub const STA_REC_HE_CAP_LE_EQ_80M_RX_STBC: u64 = 0x400;
pub const STA_REC_HE_CAP_GT_80M_TX_STBC: u64 = 0x800;
pub const STA_REC_HE_CAP_GT_80M_RX_STBC: u64 = 0x1000;
pub const STA_REC_HE_CAP_SU_PPDU_1LTF_8US_GI: u64 = 0x2000;
pub const STA_REC_HE_CAP_SU_MU_PPDU_4LTF_8US_GI: u64 = 0x4000;
pub const STA_REC_HE_CAP_ER_SU_PPDU_1LTF_8US_GI: u64 = 0x8000;
pub const STA_REC_HE_CAP_ER_SU_PPDU_4LTF_8US_GI: u64 = 0x1_0000;
pub const STA_REC_HE_CAP_NDP_4LTF_3DOT2MS_GI: u64 = 0x2_0000;
pub const STA_REC_HE_CAP_BW20_RU242_SUPPORT: u64 = 0x4_0000;
pub const STA_REC_HE_CAP_TX_1024QAM_UNDER_RU242: u64 = 0x8_0000;
pub const STA_REC_HE_CAP_RX_1024QAM_UNDER_RU242: u64 = 0x10_0000;
pub const PHY_MODE_A: u64 = 1;
pub const PHY_MODE_B: u64 = 2;
pub const PHY_MODE_G: u64 = 4;
pub const PHY_MODE_GN: u64 = 8;
pub const PHY_MODE_AN: u64 = 16;
pub const PHY_MODE_AC: u64 = 32;
pub const PHY_MODE_AX_24G: u64 = 64;
pub const PHY_MODE_AX_5G: u64 = 128;
pub const PHY_MODE_AX_6G: u64 = 1;
pub const PHY_MODE_BE_24G: u64 = 2;
pub const PHY_MODE_BE_5G: u64 = 4;
pub const PHY_MODE_BE_6G: u64 = 8;
pub const MODE_CCK: u64 = 1;
pub const MODE_OFDM: u64 = 2;
pub const MODE_HT: u64 = 4;
pub const MODE_VHT: u64 = 8;
pub const MODE_HE: u64 = 16;
pub const MODE_EHT: u64 = 32;
pub const STA_CAP_WMM: u64 = 1;
pub const STA_CAP_SGI_20: u64 = 16;
pub const STA_CAP_SGI_40: u64 = 32;
pub const STA_CAP_TX_STBC: u64 = 64;
pub const STA_CAP_RX_STBC: u64 = 128;
pub const STA_CAP_VHT_SGI_80: u64 = 0x1_0000;
pub const STA_CAP_VHT_SGI_160: u64 = 0x2_0000;
pub const STA_CAP_VHT_TX_STBC: u64 = 0x4_0000;
pub const STA_CAP_VHT_RX_STBC: u64 = 0x8_0000;
pub const STA_CAP_VHT_LDPC: u64 = 0x80_0000;
pub const STA_CAP_LDPC: u64 = 0x100_0000;
pub const STA_CAP_HT: u64 = 0x400_0000;
pub const STA_CAP_VHT: u64 = 0x800_0000;
pub const STA_CAP_HE: u64 = 0x1000_0000;
pub const HR_DSSS_ERP_BASIC_RATE: u64 = 15;
pub const OFDM_BASIC_RATE: u64 = 0x540;
pub const MT_WTBL_RATE_TX_MODE: u64 = 960;
pub const MT_WTBL_RATE_MCS: u64 = 63;
pub const MT_WTBL_RATE_NSS: u64 = 0x1c00;
pub const MT_WTBL_RATE_HE_GI: u64 = 240;
pub const MT_WTBL_RATE_GI: u64 = 15;
pub const MT_WTBL_W5_CHANGE_BW_RATE: u64 = 224;
pub const MT_WTBL_W5_SHORT_GI_20: u64 = 256;
pub const MT_WTBL_W5_SHORT_GI_40: u64 = 512;
pub const MT_WTBL_W5_SHORT_GI_80: u64 = 0x400;
pub const MT_WTBL_W5_SHORT_GI_160: u64 = 0x800;
pub const MT_WTBL_W5_BW_CAP: u64 = 0x3000;
pub const MT_WTBL_W5_MPDU_FAIL_COUNT: u64 = 0x380_0000;
pub const MT_WTBL_W5_MPDU_OK_COUNT: u64 = 0x1c00_0000;
pub const MT_WTBL_W5_RATE_IDX: u64 = 0xe000_0000;
pub const MCU_UNI_CMD_EVENT: u64 = 2;
pub const MCU_UNI_CMD_UNSOLICITED_EVENT: u64 = 4;
pub const MCU_CMD_ACK: u64 = 1;
pub const MCU_CMD_UNI: u64 = 2;
pub const MCU_CMD_SET: u64 = 4;
pub const MCU_CMD_UNI_EXT_ACK: u64 = 7;
pub const MCU_CMD_UNI_QUERY_ACK: u64 = 3;
pub const __MCU_CMD_FIELD_ID: u64 = 255;
pub const __MCU_CMD_FIELD_EXT_ID: u64 = 0xff00;
pub const __MCU_CMD_FIELD_QUERY: u64 = 0x1_0000;
pub const __MCU_CMD_FIELD_UNI: u64 = 0x2_0000;
pub const __MCU_CMD_FIELD_CE: u64 = 0x4_0000;
pub const __MCU_CMD_FIELD_WA: u64 = 0x8_0000;
pub const __MCU_CMD_FIELD_WM: u64 = 0x10_0000;
pub const UNI_WOW_DETECT_TYPE_MAGIC: u64 = 1;
pub const UNI_WOW_DETECT_TYPE_ANY: u64 = 2;
pub const UNI_WOW_DETECT_TYPE_DISCONNECT: u64 = 4;
pub const UNI_WOW_DETECT_TYPE_GTK_REKEY_FAIL: u64 = 8;
pub const UNI_WOW_DETECT_TYPE_BCN_LOST: u64 = 16;
pub const UNI_WOW_DETECT_TYPE_SCH_SCAN_HIT: u64 = 32;
pub const UNI_WOW_DETECT_TYPE_BITMAP: u64 = 64;
pub const MT76_CONNAC_SCAN_DONE_EVENT_MAX_CHANNEL_NUM: u64 = 64;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac2_mcu_uni_txd {
    pub txd: [Le32; 8],
    pub len: Le16,
    pub cid: Le16,
    pub rsv: u8,
    pub pkt_type: u8,
    pub frag_n: u8,
    pub seq: u8,
    pub checksum: Le16,
    pub s2d_index: u8,
    pub option: u8,
    pub rsv1: [u8; 4],
}

pub const MT76_CONNAC_WOW_MASK_MAX_LEN: u64 = 16;
pub const MT76_CONNAC_WOW_PATTEN_MAX_LEN: u64 = 128;
pub const MT_SKU_POWER_LIMIT: u64 = 161;
pub const MT76_CONNAC_MCU_STATUS_WLAN_FAILURE: u64 = 0xc000_0001;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac2_patch_hdr_desc {
    pub patch_ver: Be32,
    pub subsys: Be32,
    pub feature: Be32,
    pub n_region: Be32,
    pub crc: Be32,
    pub rsv: [u32; 11],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac2_patch_hdr {
    pub build_date: [u8; 16],
    pub platform: [u8; 4],
    pub hw_sw_ver: Be32,
    pub patch_ver: Be32,
    pub checksum: Be16,
    pub rsv: u16,
    pub desc: mt76_connac2_patch_hdr_desc,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac2_patch_sec_anon_3_info {
    pub addr: Be32,
    pub len: Be32,
    pub sec_key_idx: Be32,
    pub align_len: Be32,
    pub rsv: [u32; 9],
}

#[allow(missing_debug_implementations)] // union fields share one store
#[repr(C)]
#[derive(Clone, Copy)]
pub union mt76_connac2_patch_sec_anon_3 {
    pub spec: [Be32; 13],
    pub info: mt76_connac2_patch_sec_anon_3_info,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac2_patch_sec {
    pub r#type: Be32,
    pub offs: Be32,
    pub size: Be32,
    pub mt76_connac2_patch_sec_anon_3: mt76_connac2_patch_sec_anon_3,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac2_fw_trailer {
    pub chip_id: u8,
    pub eco_code: u8,
    pub n_region: u8,
    pub format_ver: u8,
    pub format_flag: u8,
    pub rsv: [u8; 2],
    pub fw_ver: [u8; 10],
    pub build_date: [u8; 15],
    pub crc: Le32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac2_fw_region {
    pub decomp_crc: Le32,
    pub decomp_len: Le32,
    pub decomp_blk_sz: Le32,
    pub rsv: [u8; 4],
    pub addr: Le32,
    pub len: Le32,
    pub feature_set: u8,
    pub r#type: u8,
    pub rsv1: [u8; 14],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac3_multi_header_v2_sec_raw_format {
    pub r#type: Le32,
    pub offset: Le32,
    pub size: Le32,
    pub spec: [u8; 52],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac3_multi_header_v2_raw_format {
    pub pack_time: [u8; 20],
    pub chip_id_eco_ver: [u8; 8],
    pub patch_ver: Le32,
    pub global_desc_head: [u8; 4],
    pub global_subsys: Le32,
    pub rsv2: [u8; 4],
    pub sec_num: Le32,
    pub rsv3: [u8; 48],
}
pub const mt76_connac3_multi_header_v2_raw_format_FLEX: usize =
    core::mem::size_of::<mt76_connac3_multi_header_v2_sec_raw_format>();

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct tlv {
    pub tag: Le16,
    pub len: Le16,
}
pub const tlv_FLEX: usize = core::mem::size_of::<u8>();

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_info_omac {
    pub tag: Le16,
    pub len: Le16,
    pub hw_bss_idx: u8,
    pub omac_idx: u8,
    pub band_idx: u8,
    pub rsv0: u8,
    pub conn_type: Le32,
    pub rsv1: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_info_rf_ch {
    pub tag: Le16,
    pub len: Le16,
    pub pri_ch: u8,
    pub center_ch0: u8,
    pub center_ch1: u8,
    pub bw: u8,
    pub he_ru26_block: u8,
    pub he_all_disable: u8,
    pub rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_info_ext_bss {
    pub tag: Le16,
    pub len: Le16,
    pub mbss_tsf_offset: Le32,
    pub rsv: [u8; 8],
}

pub const BSS_INFO_OMAC: u64 = 0;
pub const BSS_INFO_BASIC: u64 = 1;
pub const BSS_INFO_RF_CH: u64 = 2;
pub const BSS_INFO_PM: u64 = 3;
pub const BSS_INFO_UAPSD: u64 = 4;
pub const BSS_INFO_ROAM_DETECT: u64 = 5;
pub const BSS_INFO_LQ_RM: u64 = 6;
pub const BSS_INFO_EXT_BSS: u64 = 7;
pub const BSS_INFO_BMC_RATE: u64 = 8;
pub const BSS_INFO_SYNC_MODE: u64 = 9;
pub const BSS_INFO_RA: u64 = 10;
pub const BSS_INFO_HW_AMSDU: u64 = 11;
pub const BSS_INFO_BSS_COLOR: u64 = 12;
pub const BSS_INFO_HE_BASIC: u64 = 13;
pub const BSS_INFO_PROTECT_INFO: u64 = 14;
pub const BSS_INFO_OFFLOAD: u64 = 15;
pub const BSS_INFO_11V_MBSSID: u64 = 16;
pub const BSS_INFO_MAX_NUM: u64 = 17;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_ntlv_hdr {
    pub rsv: [u8; 2],
    pub tlv_num: Le16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_req_hdr {
    pub bss_idx: u8,
    pub wlan_idx_lo: u8,
    pub tlv_num: Le16,
    pub is_tlv_append: u8,
    pub muar_idx: u8,
    pub wlan_idx_hi: u8,
    pub rsv: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_ht {
    pub tag: Le16,
    pub len: Le16,
    pub ht_cap: Le16,
    pub rsv: u16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_vht {
    pub tag: Le16,
    pub len: Le16,
    pub vht_cap: Le32,
    pub vht_rx_mcs_map: Le16,
    pub vht_tx_mcs_map: Le16,
    pub rts_bw_sig: u8,
    pub rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_uapsd {
    pub tag: Le16,
    pub len: Le16,
    pub dac_map: u8,
    pub tac_map: u8,
    pub max_sp: u8,
    pub rsv0: u8,
    pub listen_interval: Le16,
    pub rsv1: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_ba {
    pub tag: Le16,
    pub len: Le16,
    pub tid: u8,
    pub ba_type: u8,
    pub amsdu: u8,
    pub ba_en: u8,
    pub ssn: Le16,
    pub winsize: Le16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_he {
    pub tag: Le16,
    pub len: Le16,
    pub he_cap: Le32,
    pub t_frame_dur: u8,
    pub max_ampdu_exp: u8,
    pub bw_set: u8,
    pub device_class: u8,
    pub dcm_tx_mode: u8,
    pub dcm_tx_max_nss: u8,
    pub dcm_rx_mode: u8,
    pub dcm_rx_max_nss: u8,
    pub dcm_max_ru: u8,
    pub punc_pream_rx: u8,
    pub pkt_ext: u8,
    pub rsv1: u8,
    pub max_nss_mcs: [Le16; 3],
    pub rsv2: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_he_v2 {
    pub tag: Le16,
    pub len: Le16,
    pub he_mac_cap: [u8; 6],
    pub he_phy_cap: [u8; 11],
    pub pkt_ext: u8,
    pub max_nss_mcs: [Le16; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_amsdu {
    pub tag: Le16,
    pub len: Le16,
    pub max_amsdu_num: u8,
    pub max_mpdu_size: u8,
    pub amsdu_en: u8,
    pub rsv: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_state {
    pub tag: Le16,
    pub len: Le16,
    pub flags: Le32,
    pub state: u8,
    pub vht_opmode: u8,
    pub action: u8,
    pub rsv: [u8; 1],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_ra_info {
    pub tag: Le16,
    pub len: Le16,
    pub legacy: Le16,
    pub rx_mcs_bitmask: [u8; 10],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_phy {
    pub tag: Le16,
    pub len: Le16,
    pub basic_rate: Le16,
    pub phy_type: u8,
    pub ampdu: u8,
    pub rts_policy: u8,
    pub rcpi: u8,
    pub max_ampdu_len: u8,
    pub rsv: [u8; 1],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_he_6g_capa {
    pub tag: Le16,
    pub len: Le16,
    pub capa: Le16,
    pub rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_pn_info {
    pub tag: Le16,
    pub len: Le16,
    pub pn: [u8; 6],
    pub tsc_type: u8,
    pub rsv: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sec_key {
    pub cipher_id: u8,
    pub cipher_len: u8,
    pub key_id: u8,
    pub key_len: u8,
    pub key: [u8; 32],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_sec {
    pub tag: Le16,
    pub len: Le16,
    pub add: u8,
    pub n_cipher: u8,
    pub rsv: [u8; 2],
    pub key: [sec_key; 2],
}

pub const sta_rec_bf_mem_col_MASK: u64 = 63;
pub const sta_rec_bf_mem_col_SHIFT: u64 = 0;
pub const sta_rec_bf_mem_row_msb_MASK: u64 = 192;
pub const sta_rec_bf_mem_row_msb_SHIFT: u64 = 6;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sta_rec_bf_mem {
    pub row: u8,
    pub bitfield_0: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_bf {
    pub tag: Le16,
    pub len: Le16,
    pub pfmu: Le16,
    pub su_mu: u8,
    pub bf_cap: u8,
    pub sounding_phy: u8,
    pub ndpa_rate: u8,
    pub ndp_rate: u8,
    pub rept_poll_rate: u8,
    pub tx_mode: u8,
    pub ncol: u8,
    pub nrow: u8,
    pub bw: u8,
    pub mem_total: u8,
    pub mem_20m: u8,
    pub mem: [sta_rec_bf_mem; 4],
    pub smart_ant: Le16,
    pub se_idx: u8,
    pub auto_sounding: u8,
    pub ibf_timeout: u8,
    pub ibf_dbw: u8,
    pub ibf_ncol: u8,
    pub ibf_nrow: u8,
    pub nrow_gt_bw80: u8,
    pub ncol_gt_bw80: u8,
    pub ru_start_idx: u8,
    pub ru_end_idx: u8,
    pub trigger_su: u8,
    pub trigger_mu: u8,
    pub ng16_su: u8,
    pub ng16_mu: u8,
    pub codebook42_su: u8,
    pub codebook75_mu: u8,
    pub he_ltf: u8,
    pub rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_bfee {
    pub tag: Le16,
    pub len: Le16,
    pub fb_identity_matrix: u8,
    pub ignore_feedback: u8,
    pub rsv: [u8; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sta_rec_muru_cfg {
    pub ofdma_dl_en: u8,
    pub ofdma_ul_en: u8,
    pub mimo_dl_en: u8,
    pub mimo_ul_en: u8,
    pub rsv: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sta_rec_muru_ofdma_dl {
    pub punc_pream_rx: u8,
    pub he_20m_in_40m_2g: u8,
    pub he_20m_in_160m: u8,
    pub he_80m_in_160m: u8,
    pub lt16_sigb: u8,
    pub rx_su_comp_sigb: u8,
    pub rx_su_non_comp_sigb: u8,
    pub rsv: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sta_rec_muru_ofdma_ul {
    pub t_frame_dur: u8,
    pub mu_cascading: u8,
    pub uo_ra: u8,
    pub he_2x996_tone: u8,
    pub rx_t_frame_11ac: u8,
    pub rx_ctrl_frame_to_mbss: u8,
    pub rsv: [u8; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sta_rec_muru_mimo_dl {
    pub vht_mu_bfee: u8,
    pub partial_bw_dl_mimo: u8,
    pub rsv: [u8; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sta_rec_muru_mimo_ul {
    pub full_ul_mimo: u8,
    pub partial_ul_mimo: u8,
    pub rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_muru {
    pub tag: Le16,
    pub len: Le16,
    pub cfg: sta_rec_muru_cfg,
    pub ofdma_dl: sta_rec_muru_ofdma_dl,
    pub ofdma_ul: sta_rec_muru_ofdma_ul,
    pub mimo_dl: sta_rec_muru_mimo_dl,
    pub mimo_ul: sta_rec_muru_mimo_ul,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_remove {
    pub tag: Le16,
    pub len: Le16,
    pub action: u8,
    pub pad: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sta_phy {
    pub r#type: u8,
    pub flag: u8,
    pub stbc: u8,
    pub sgi: u8,
    pub bw: u8,
    pub ldpc: u8,
    pub mcs: u8,
    pub nss: u8,
    pub he_ltf: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_ra {
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
    pub phy: sta_phy,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_ra_fixed {
    pub tag: Le16,
    pub len: Le16,
    pub field: Le32,
    pub op_mode: u8,
    pub op_vht_chan_width: u8,
    pub op_vht_rx_nss: u8,
    pub op_vht_rx_nss_type: u8,
    pub phy: sta_phy,
    pub spe_idx: u8,
    pub short_preamble: u8,
    pub is_5g: u8,
    pub mmps_mode: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_tx_proc {
    pub tag: Le16,
    pub len: Le16,
    pub flag: Le32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct sta_rec_eml_op {
    pub tag: Le16,
    pub len: Le16,
    pub link_bitmap: u8,
    pub link_ant_num: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct wtbl_req_hdr {
    pub wlan_idx_lo: u8,
    pub operation: u8,
    pub tlv_num: Le16,
    pub wlan_idx_hi: u8,
    pub rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct wtbl_rx {
    pub tag: Le16,
    pub len: Le16,
    pub rcid: u8,
    pub rca1: u8,
    pub rca2: u8,
    pub rv: u8,
    pub rsv: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct wtbl_ht {
    pub tag: Le16,
    pub len: Le16,
    pub ht: u8,
    pub ldpc: u8,
    pub af: u8,
    pub mm: u8,
    pub rsv: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct wtbl_vht {
    pub tag: Le16,
    pub len: Le16,
    pub ldpc: u8,
    pub dyn_bw: u8,
    pub vht: u8,
    pub txop_ps: u8,
    pub rsv: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct wtbl_tx_ps {
    pub tag: Le16,
    pub len: Le16,
    pub txps: u8,
    pub rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct wtbl_hdr_trans {
    pub tag: Le16,
    pub len: Le16,
    pub to_ds: u8,
    pub from_ds: u8,
    pub no_rx_trans: u8,
    pub rsv: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct wtbl_smps {
    pub tag: Le16,
    pub len: Le16,
    pub smps: u8,
    pub rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct wtbl_bf {
    pub tag: Le16,
    pub len: Le16,
    pub ibf: u8,
    pub ebf: u8,
    pub ibf_vht: u8,
    pub ebf_vht: u8,
    pub gid: u8,
    pub pfmu_idx: u8,
    pub rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct wtbl_pn {
    pub tag: Le16,
    pub len: Le16,
    pub pn: [u8; 6],
    pub rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct wtbl_spe {
    pub tag: Le16,
    pub len: Le16,
    pub spe_idx: u8,
    pub rsv: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct wtbl_raw {
    pub tag: Le16,
    pub len: Le16,
    pub wtbl_idx: u8,
    pub dw: u8,
    pub rsv: [u8; 2],
    pub msk: Le32,
    pub val: Le32,
}

pub const STA_REC_BASIC: u64 = 0;
pub const STA_REC_RA: u64 = 1;
pub const STA_REC_RA_CMM_INFO: u64 = 2;
pub const STA_REC_RA_UPDATE: u64 = 3;
pub const STA_REC_BF: u64 = 4;
pub const STA_REC_AMSDU: u64 = 5;
pub const STA_REC_BA: u64 = 6;
pub const STA_REC_STATE: u64 = 7;
pub const STA_REC_TX_PROC: u64 = 8;
pub const STA_REC_HT: u64 = 9;
pub const STA_REC_VHT: u64 = 10;
pub const STA_REC_APPS: u64 = 11;
pub const STA_REC_KEY: u64 = 12;
pub const STA_REC_WTBL: u64 = 13;
pub const STA_REC_HE: u64 = 14;
pub const STA_REC_HW_AMSDU: u64 = 15;
pub const STA_REC_WTBL_AADOM: u64 = 16;
pub const STA_REC_KEY_V2: u64 = 17;
pub const STA_REC_MURU: u64 = 18;
pub const STA_REC_MUEDCA: u64 = 19;
pub const STA_REC_BFEE: u64 = 20;
pub const STA_REC_PHY: u64 = 21;
pub const STA_REC_HE_6G: u64 = 23;
pub const STA_REC_HE_V2: u64 = 25;
pub const STA_REC_MLD: u64 = 32;
pub const STA_REC_EHT_MLD: u64 = 33;
pub const STA_REC_EHT: u64 = 34;
pub const STA_REC_MLD_OFF: u64 = 35;
pub const STA_REC_REMOVE: u64 = 37;
pub const STA_REC_PN_INFO: u64 = 38;
pub const STA_REC_KEY_V3: u64 = 39;
pub const STA_REC_HDRT: u64 = 40;
pub const STA_REC_EML_OP: u64 = 41;
pub const STA_REC_HDR_TRANS: u64 = 43;
pub const STA_REC_PS_LEAVE: u64 = 69;
pub const STA_REC_MAX_NUM: u64 = 70;
pub const WTBL_GENERIC: u64 = 0;
pub const WTBL_RX: u64 = 1;
pub const WTBL_HT: u64 = 2;
pub const WTBL_VHT: u64 = 3;
pub const WTBL_PEER_PS: u64 = 4;
pub const WTBL_TX_PS: u64 = 5;
pub const WTBL_HDR_TRANS: u64 = 6;
pub const WTBL_SEC_KEY: u64 = 7;
pub const WTBL_BA: u64 = 8;
pub const WTBL_RDG: u64 = 9;
pub const WTBL_PROTECT: u64 = 10;
pub const WTBL_CLEAR: u64 = 11;
pub const WTBL_BF: u64 = 12;
pub const WTBL_SMPS: u64 = 13;
pub const WTBL_RAW_DATA: u64 = 14;
pub const WTBL_PN: u64 = 15;
pub const WTBL_SPE: u64 = 16;
pub const WTBL_MAX_NUM: u64 = 17;
pub const PHY_TYPE_HR_DSSS_INDEX: u64 = 0;
pub const PHY_TYPE_ERP_INDEX: u64 = 1;
pub const PHY_TYPE_ERP_P2P_INDEX: u64 = 2;
pub const PHY_TYPE_OFDM_INDEX: u64 = 3;
pub const PHY_TYPE_HT_INDEX: u64 = 4;
pub const PHY_TYPE_VHT_INDEX: u64 = 5;
pub const PHY_TYPE_HE_INDEX: u64 = 6;
pub const PHY_TYPE_BE_INDEX: u64 = 7;
pub const PHY_TYPE_INDEX_NUM: u64 = 8;
pub const WTBL_RESET_AND_SET: u64 = 1;
pub const WTBL_SET: u64 = 2;
pub const WTBL_QUERY: u64 = 3;
pub const WTBL_RESET_ALL: u64 = 4;
pub const MT_BA_TYPE_INVALID: u64 = 0;
pub const MT_BA_TYPE_ORIGINATOR: u64 = 1;
pub const MT_BA_TYPE_RECIPIENT: u64 = 2;
pub const RST_BA_MAC_TID_MATCH: u64 = 0;
pub const RST_BA_MAC_MATCH: u64 = 1;
pub const RST_BA_NO_MATCH: u64 = 2;
pub const DEV_INFO_ACTIVE: u64 = 0;
pub const DEV_INFO_MAX_NUM: u64 = 1;
pub const MCU_EVENT_TARGET_ADDRESS_LEN: u64 = 1;
pub const MCU_EVENT_FW_START: u64 = 1;
pub const MCU_EVENT_GENERIC: u64 = 1;
pub const MCU_EVENT_ACCESS_REG: u64 = 2;
pub const MCU_EVENT_MT_PATCH_SEM: u64 = 4;
pub const MCU_EVENT_REG_ACCESS: u64 = 5;
pub const MCU_EVENT_LP_INFO: u64 = 7;
pub const MCU_EVENT_SCAN_DONE: u64 = 13;
pub const MCU_EVENT_TX_DONE: u64 = 15;
pub const MCU_EVENT_ROC: u64 = 16;
pub const MCU_EVENT_BSS_ABSENCE: u64 = 17;
pub const MCU_EVENT_BSS_BEACON_LOSS: u64 = 19;
pub const MCU_EVENT_CH_PRIVILEGE: u64 = 24;
pub const MCU_EVENT_SCHED_SCAN_DONE: u64 = 35;
pub const MCU_EVENT_DBG_MSG: u64 = 39;
pub const MCU_EVENT_RSSI_NOTIFY: u64 = 150;
pub const MCU_EVENT_TXPWR: u64 = 208;
pub const MCU_EVENT_EXT: u64 = 237;
pub const MCU_EVENT_RESTART_DL: u64 = 239;
pub const MCU_EVENT_COREDUMP: u64 = 240;
pub const MCU_EXT_EVENT_PS_SYNC: u64 = 5;
pub const MCU_EXT_EVENT_FW_LOG_2_HOST: u64 = 19;
pub const MCU_EXT_EVENT_THERMAL_PROTECT: u64 = 34;
pub const MCU_EXT_EVENT_ASSERT_DUMP: u64 = 35;
pub const MCU_EXT_EVENT_RDD_REPORT: u64 = 58;
pub const MCU_EXT_EVENT_CSA_NOTIFY: u64 = 79;
pub const MCU_EXT_EVENT_WA_TX_STAT: u64 = 116;
pub const MCU_EXT_EVENT_BCC_NOTIFY: u64 = 117;
pub const MCU_EXT_EVENT_WF_RF_PIN_CTRL: u64 = 154;
pub const MCU_EXT_EVENT_MURU_CTRL: u64 = 159;
pub const MCU_UNI_EVENT_RESULT: u64 = 1;
pub const MCU_UNI_EVENT_HIF_CTRL: u64 = 3;
pub const MCU_UNI_EVENT_FW_LOG_2_HOST: u64 = 4;
pub const MCU_UNI_EVENT_ACCESS_REG: u64 = 6;
pub const MCU_UNI_EVENT_IE_COUNTDOWN: u64 = 9;
pub const MCU_UNI_EVENT_COREDUMP: u64 = 10;
pub const MCU_UNI_EVENT_BSS_BEACON_LOSS: u64 = 12;
pub const MCU_UNI_EVENT_PS_SYNC: u64 = 13;
pub const MCU_UNI_EVENT_SCAN_DONE: u64 = 14;
pub const MCU_UNI_EVENT_RDD_REPORT: u64 = 17;
pub const MCU_UNI_EVENT_ROC: u64 = 39;
pub const MCU_UNI_EVENT_MBMC: u64 = 40;
pub const MCU_UNI_EVENT_TX_DONE: u64 = 45;
pub const MCU_UNI_EVENT_THERMAL: u64 = 53;
pub const MCU_UNI_EVENT_RSSI_MONITOR: u64 = 65;
pub const MCU_UNI_EVENT_NIC_CAPAB: u64 = 67;
pub const MCU_UNI_EVENT_NAN: u64 = 86;
pub const MCU_UNI_EVENT_WED_RRO: u64 = 87;
pub const MCU_UNI_EVENT_PER_STA_INFO: u64 = 109;
pub const MCU_UNI_EVENT_ALL_STA_INFO: u64 = 110;
pub const MCU_UNI_EVENT_SDO: u64 = 131;
pub const MCU_Q_QUERY: u64 = 0;
pub const MCU_Q_SET: u64 = 1;
pub const MCU_Q_RESERVED: u64 = 2;
pub const MCU_Q_NA: u64 = 3;
pub const MCU_S2D_H2N: u64 = 0;
pub const MCU_S2D_C2N: u64 = 1;
pub const MCU_S2D_H2C: u64 = 2;
pub const MCU_S2D_H2CN: u64 = 3;
pub const PATCH_NOT_DL_SEM_FAIL: u64 = 0;
pub const PATCH_IS_DL: u64 = 1;
pub const PATCH_NOT_DL_SEM_SUCCESS: u64 = 2;
pub const PATCH_REL_SEM_SUCCESS: u64 = 3;
pub const CB_PATCH_GET_SEM_NEED_PATCH: u64 = 0;
pub const CB_PATCH_IS_DL: u64 = 1;
pub const CB_PATCH_NO_SEM_NEED_PATCH: u64 = 2;
pub const CB_PATCH_REL_SEM_SUCCESS: u64 = 3;
pub const FW_STATE_INITIAL: u64 = 0;
pub const FW_STATE_FW_DOWNLOAD: u64 = 1;
pub const FW_STATE_NORMAL_OPERATION: u64 = 2;
pub const FW_STATE_NORMAL_TRX: u64 = 3;
pub const FW_STATE_RDY: u64 = 7;
pub const CH_SWITCH_NORMAL: u64 = 0;
pub const CH_SWITCH_SCAN: u64 = 3;
pub const CH_SWITCH_MCC: u64 = 4;
pub const CH_SWITCH_DFS: u64 = 5;
pub const CH_SWITCH_BACKGROUND_SCAN_START: u64 = 6;
pub const CH_SWITCH_BACKGROUND_SCAN_RUNNING: u64 = 7;
pub const CH_SWITCH_BACKGROUND_SCAN_STOP: u64 = 8;
pub const CH_SWITCH_SCAN_BYPASS_DPD: u64 = 9;
pub const THERMAL_SENSOR_TEMP_QUERY: u64 = 0;
pub const THERMAL_SENSOR_MANUAL_CTRL: u64 = 1;
pub const THERMAL_SENSOR_INFO_QUERY: u64 = 2;
pub const THERMAL_SENSOR_TASK_CTRL: u64 = 3;
pub const MCU_CIPHER_NONE: u64 = 0;
pub const MCU_CIPHER_WEP40: u64 = 1;
pub const MCU_CIPHER_WEP104: u64 = 2;
pub const MCU_CIPHER_WEP128: u64 = 3;
pub const MCU_CIPHER_TKIP: u64 = 4;
pub const MCU_CIPHER_AES_CCMP: u64 = 5;
pub const MCU_CIPHER_CCMP_256: u64 = 6;
pub const MCU_CIPHER_GCMP: u64 = 7;
pub const MCU_CIPHER_GCMP_256: u64 = 8;
pub const MCU_CIPHER_WAPI: u64 = 9;
pub const MCU_CIPHER_BIP_CMAC_128: u64 = 10;
pub const MCU_CIPHER_BIP_CMAC_256: u64 = 11;
pub const MCU_CIPHER_BCN_PROT_CMAC_128: u64 = 12;
pub const MCU_CIPHER_BCN_PROT_CMAC_256: u64 = 13;
pub const MCU_CIPHER_BCN_PROT_GMAC_128: u64 = 14;
pub const MCU_CIPHER_BCN_PROT_GMAC_256: u64 = 15;
pub const MCU_CIPHER_BIP_GMAC_128: u64 = 16;
pub const MCU_CIPHER_BIP_GMAC_256: u64 = 17;
pub const EE_MODE_EFUSE: u64 = 0;
pub const EE_MODE_BUFFER: u64 = 1;
pub const EE_FORMAT_BIN: u64 = 0;
pub const EE_FORMAT_WHOLE: u64 = 1;
pub const EE_FORMAT_MULTIPLE: u64 = 2;
pub const MCU_PHY_STATE_TX_RATE: u64 = 0;
pub const MCU_PHY_STATE_RX_RATE: u64 = 1;
pub const MCU_PHY_STATE_RSSI: u64 = 2;
pub const MCU_PHY_STATE_CONTENTION_RX_RATE: u64 = 3;
pub const MCU_PHY_STATE_OFDMLQ_CNINFO: u64 = 4;
pub const MCU_EXT_CMD_EFUSE_ACCESS: u64 = 1;
pub const MCU_EXT_CMD_RF_REG_ACCESS: u64 = 2;
pub const MCU_EXT_CMD_RF_TEST: u64 = 4;
pub const MCU_EXT_CMD_ID_RADIO_ON_OFF_CTRL: u64 = 5;
pub const MCU_EXT_CMD_PM_STATE_CTRL: u64 = 7;
pub const MCU_EXT_CMD_CHANNEL_SWITCH: u64 = 8;
pub const MCU_EXT_CMD_SET_TX_POWER_CTRL: u64 = 17;
pub const MCU_EXT_CMD_FW_LOG_2_HOST: u64 = 19;
pub const MCU_EXT_CMD_TXBF_ACTION: u64 = 30;
pub const MCU_EXT_CMD_EFUSE_BUFFER_MODE: u64 = 33;
pub const MCU_EXT_CMD_THERMAL_PROT: u64 = 35;
pub const MCU_EXT_CMD_STA_REC_UPDATE: u64 = 37;
pub const MCU_EXT_CMD_BSS_INFO_UPDATE: u64 = 38;
pub const MCU_EXT_CMD_EDCA_UPDATE: u64 = 39;
pub const MCU_EXT_CMD_DEV_INFO_UPDATE: u64 = 42;
pub const MCU_EXT_CMD_THERMAL_CTRL: u64 = 44;
pub const MCU_EXT_CMD_WTBL_UPDATE: u64 = 50;
pub const MCU_EXT_CMD_SET_DRR_CTRL: u64 = 54;
pub const MCU_EXT_CMD_SET_RDD_CTRL: u64 = 58;
pub const MCU_EXT_CMD_ATE_CTRL: u64 = 61;
pub const MCU_EXT_CMD_PROTECT_CTRL: u64 = 62;
pub const MCU_EXT_CMD_DBDC_CTRL: u64 = 69;
pub const MCU_EXT_CMD_MAC_INIT_CTRL: u64 = 70;
pub const MCU_EXT_CMD_RX_HDR_TRANS: u64 = 71;
pub const MCU_EXT_CMD_MUAR_UPDATE: u64 = 72;
pub const MCU_EXT_CMD_BCN_OFFLOAD: u64 = 73;
pub const MCU_EXT_CMD_RX_AIRTIME_CTRL: u64 = 74;
pub const MCU_EXT_CMD_SET_RX_PATH: u64 = 78;
pub const MCU_EXT_CMD_EFUSE_FREE_BLOCK: u64 = 79;
pub const MCU_EXT_CMD_TX_POWER_FEATURE_CTRL: u64 = 88;
pub const MCU_EXT_CMD_RXDCOC_CAL: u64 = 89;
pub const MCU_EXT_CMD_GET_MIB_INFO: u64 = 90;
pub const MCU_EXT_CMD_TXDPD_CAL: u64 = 96;
pub const MCU_EXT_CMD_CAL_CACHE: u64 = 103;
pub const MCU_EXT_CMD_RED_ENABLE: u64 = 104;
pub const MCU_EXT_CMD_CP_SUPPORT: u64 = 117;
pub const MCU_EXT_CMD_SET_RADAR_TH: u64 = 124;
pub const MCU_EXT_CMD_SET_RDD_PATTERN: u64 = 125;
pub const MCU_EXT_CMD_MWDS_SUPPORT: u64 = 128;
pub const MCU_EXT_CMD_SET_SER_TRIGGER: u64 = 129;
pub const MCU_EXT_CMD_TWT_AGRT_UPDATE: u64 = 148;
pub const MCU_EXT_CMD_FW_DBG_CTRL: u64 = 149;
pub const MCU_EXT_CMD_OFFCH_SCAN_CTRL: u64 = 154;
pub const MCU_EXT_CMD_SET_RDD_TH: u64 = 157;
pub const MCU_EXT_CMD_MURU_CTRL: u64 = 159;
pub const MCU_EXT_CMD_SET_SPR: u64 = 168;
pub const MCU_EXT_CMD_GROUP_PRE_CAL_INFO: u64 = 171;
pub const MCU_EXT_CMD_DPD_PRE_CAL_INFO: u64 = 172;
pub const MCU_EXT_CMD_PHY_STAT_INFO: u64 = 173;
pub const MCU_EXT_CMD_WF_RF_PIN_CTRL: u64 = 189;
pub const MCU_UNI_CMD_DEV_INFO_UPDATE: u64 = 1;
pub const MCU_UNI_CMD_BSS_INFO_UPDATE: u64 = 2;
pub const MCU_UNI_CMD_STA_REC_UPDATE: u64 = 3;
pub const MCU_UNI_CMD_EDCA_UPDATE: u64 = 4;
pub const MCU_UNI_CMD_SUSPEND: u64 = 5;
pub const MCU_UNI_CMD_OFFLOAD: u64 = 6;
pub const MCU_UNI_CMD_HIF_CTRL: u64 = 7;
pub const MCU_UNI_CMD_BAND_CONFIG: u64 = 8;
pub const MCU_UNI_CMD_REPT_MUAR: u64 = 9;
pub const MCU_UNI_CMD_WSYS_CONFIG: u64 = 11;
pub const MCU_UNI_CMD_REG_ACCESS: u64 = 13;
pub const MCU_UNI_CMD_CHIP_CONFIG: u64 = 14;
pub const MCU_UNI_CMD_POWER_CTRL: u64 = 15;
pub const MCU_UNI_CMD_RX_HDR_TRANS: u64 = 18;
pub const MCU_UNI_CMD_SER: u64 = 19;
pub const MCU_UNI_CMD_TWT: u64 = 20;
pub const MCU_UNI_CMD_SET_DOMAIN_INFO: u64 = 21;
pub const MCU_UNI_CMD_SCAN_REQ: u64 = 22;
pub const MCU_UNI_CMD_RDD_CTRL: u64 = 25;
pub const MCU_UNI_CMD_GET_MIB_INFO: u64 = 34;
pub const MCU_UNI_CMD_GET_STAT_INFO: u64 = 35;
pub const MCU_UNI_CMD_SNIFFER: u64 = 36;
pub const MCU_UNI_CMD_SR: u64 = 37;
pub const MCU_UNI_CMD_ROC: u64 = 39;
pub const MCU_UNI_CMD_SET_DBDC_PARMS: u64 = 40;
pub const MCU_UNI_CMD_TXPOWER: u64 = 43;
pub const MCU_UNI_CMD_SET_POWER_LIMIT: u64 = 44;
pub const MCU_UNI_CMD_EFUSE_CTRL: u64 = 45;
pub const MCU_UNI_CMD_RA: u64 = 47;
pub const MCU_UNI_CMD_MURU: u64 = 49;
pub const MCU_UNI_CMD_TESTMODE_RX_STAT: u64 = 50;
pub const MCU_UNI_CMD_BF: u64 = 51;
pub const MCU_UNI_CMD_CHANNEL_SWITCH: u64 = 52;
pub const MCU_UNI_CMD_THERMAL: u64 = 53;
pub const MCU_UNI_CMD_VOW: u64 = 55;
pub const MCU_UNI_CMD_FIXED_RATE_TABLE: u64 = 64;
pub const MCU_UNI_CMD_RSSI_MONITOR: u64 = 65;
pub const MCU_UNI_CMD_TESTMODE_CTRL: u64 = 70;
pub const MCU_UNI_CMD_NAN: u64 = 86;
pub const MCU_UNI_CMD_RRO: u64 = 87;
pub const MCU_UNI_CMD_OFFCH_SCAN_CTRL: u64 = 88;
pub const MCU_UNI_CMD_PER_STA_INFO: u64 = 109;
pub const MCU_UNI_CMD_ALL_STA_INFO: u64 = 110;
pub const MCU_UNI_CMD_ASSERT_DUMP: u64 = 111;
pub const MCU_UNI_CMD_EXT_EEPROM_CTRL: u64 = 116;
pub const MCU_UNI_CMD_RADIO_STATUS: u64 = 128;
pub const MCU_UNI_CMD_MLD: u64 = 130;
pub const MCU_UNI_CMD_SDO: u64 = 136;
pub const MCU_CMD_TARGET_ADDRESS_LEN_REQ: u64 = 1;
pub const MCU_CMD_FW_START_REQ: u64 = 2;
pub const MCU_CMD_INIT_ACCESS_REG: u64 = 3;
pub const MCU_CMD_NIC_POWER_CTRL: u64 = 4;
pub const MCU_CMD_PATCH_START_REQ: u64 = 5;
pub const MCU_CMD_PATCH_FINISH_REQ: u64 = 7;
pub const MCU_CMD_PATCH_SEM_CONTROL: u64 = 16;
pub const MCU_CMD_CB_PATCH_SEM_CONTROL: u64 = 48;
pub const MCU_CMD_CB_PATCH_START_REQ: u64 = 49;
pub const MCU_CMD_CB_PATCH_FINISH_REQ: u64 = 50;
pub const MCU_CMD_WA_PARAM: u64 = 196;
pub const MCU_CMD_EXT_CID: u64 = 237;
pub const MCU_CMD_FW_SCATTER: u64 = 238;
pub const MCU_CMD_RESTART_DL_REQ: u64 = 239;
pub const MCU_CE_CMD_TEST_CTRL: u64 = 1;
pub const MCU_CE_CMD_START_HW_SCAN: u64 = 3;
pub const MCU_CE_CMD_SET_PS_PROFILE: u64 = 5;
pub const MCU_CE_CMD_SET_RX_FILTER: u64 = 10;
pub const MCU_CE_CMD_SET_CHAN_DOMAIN: u64 = 15;
pub const MCU_CE_CMD_SET_BSS_CONNECTED: u64 = 22;
pub const MCU_CE_CMD_SET_BSS_ABORT: u64 = 23;
pub const MCU_CE_CMD_CANCEL_HW_SCAN: u64 = 27;
pub const MCU_CE_CMD_SET_ROC: u64 = 28;
pub const MCU_CE_CMD_SET_EDCA_PARMS: u64 = 29;
pub const MCU_CE_CMD_SET_P2P_OPPPS: u64 = 51;
pub const MCU_CE_CMD_SET_CLC: u64 = 92;
pub const MCU_CE_CMD_SET_RATE_TX_POWER: u64 = 93;
pub const MCU_CE_CMD_SCHED_SCAN_ENABLE: u64 = 97;
pub const MCU_CE_CMD_SCHED_SCAN_REQ: u64 = 98;
pub const MCU_CE_CMD_GET_NIC_CAPAB: u64 = 138;
pub const MCU_CE_CMD_RSSI_MONITOR: u64 = 161;
pub const MCU_CE_CMD_SET_MU_EDCA_PARMS: u64 = 176;
pub const MCU_CE_CMD_REG_WRITE: u64 = 192;
pub const MCU_CE_CMD_REG_READ: u64 = 192;
pub const MCU_CE_CMD_CHIP_CONFIG: u64 = 202;
pub const MCU_CE_CMD_FWLOG_2_HOST: u64 = 197;
pub const MCU_CE_CMD_GET_WTBL: u64 = 205;
pub const MCU_CE_CMD_GET_TXPWR: u64 = 208;
pub const MCU_CE_CMD_SET_REGD_CH: u64 = 209;
pub const PATCH_SEM_RELEASE: u64 = 0;
pub const PATCH_SEM_GET: u64 = 1;
pub const UNI_BSS_INFO_BASIC: u64 = 0;
pub const UNI_BSS_INFO_RA: u64 = 1;
pub const UNI_BSS_INFO_RLM: u64 = 2;
pub const UNI_BSS_INFO_PROTECT_INFO: u64 = 3;
pub const UNI_BSS_INFO_BSS_COLOR: u64 = 4;
pub const UNI_BSS_INFO_HE_BASIC: u64 = 5;
pub const UNI_BSS_INFO_11V_MBSSID: u64 = 6;
pub const UNI_BSS_INFO_BCN_CONTENT: u64 = 7;
pub const UNI_BSS_INFO_BCN_CSA: u64 = 8;
pub const UNI_BSS_INFO_BCN_BCC: u64 = 9;
pub const UNI_BSS_INFO_BCN_MBSSID: u64 = 10;
pub const UNI_BSS_INFO_RATE: u64 = 11;
pub const UNI_BSS_INFO_QBSS: u64 = 15;
pub const UNI_BSS_INFO_SEC: u64 = 16;
pub const UNI_BSS_INFO_BCN_PROT: u64 = 17;
pub const UNI_BSS_INFO_TXCMD: u64 = 18;
pub const UNI_BSS_INFO_UAPSD: u64 = 19;
pub const UNI_BSS_INFO_PS: u64 = 21;
pub const UNI_BSS_INFO_BCNFT: u64 = 22;
pub const UNI_BSS_INFO_IFS_TIME: u64 = 23;
pub const UNI_BSS_INFO_OFFLOAD: u64 = 25;
pub const UNI_BSS_INFO_MLD: u64 = 26;
pub const UNI_BSS_INFO_PM_DISABLE: u64 = 27;
pub const UNI_BSS_INFO_EHT: u64 = 30;
pub const UNI_BSS_INFO_MLD_LINK_OP: u64 = 36;
pub const UNI_OFFLOAD_OFFLOAD_ARP: u64 = 0;
pub const UNI_OFFLOAD_OFFLOAD_ND: u64 = 1;
pub const UNI_OFFLOAD_OFFLOAD_GTK_REKEY: u64 = 2;
pub const UNI_OFFLOAD_OFFLOAD_BMC_RPY_DETECT: u64 = 3;
pub const UNI_ALL_STA_TXRX_RATE: u64 = 0;
pub const UNI_ALL_STA_TX_STAT: u64 = 1;
pub const UNI_ALL_STA_TXRX_ADM_STAT: u64 = 2;
pub const UNI_ALL_STA_TXRX_AIR_TIME: u64 = 3;
pub const UNI_ALL_STA_DATA_TX_RETRY_COUNT: u64 = 4;
pub const UNI_ALL_STA_GI_MODE: u64 = 5;
pub const UNI_ALL_STA_TXRX_MSDU_COUNT: u64 = 6;
pub const UNI_ALL_STA_MAX_NUM: u64 = 7;
pub const MT_NIC_CAP_TX_RESOURCE: u64 = 0;
pub const MT_NIC_CAP_TX_EFUSE_ADDR: u64 = 1;
pub const MT_NIC_CAP_COEX: u64 = 2;
pub const MT_NIC_CAP_SINGLE_SKU: u64 = 3;
pub const MT_NIC_CAP_CSUM_OFFLOAD: u64 = 4;
pub const MT_NIC_CAP_HW_VER: u64 = 5;
pub const MT_NIC_CAP_SW_VER: u64 = 6;
pub const MT_NIC_CAP_MAC_ADDR: u64 = 7;
pub const MT_NIC_CAP_PHY: u64 = 8;
pub const MT_NIC_CAP_MAC: u64 = 9;
pub const MT_NIC_CAP_FRAME_BUF: u64 = 10;
pub const MT_NIC_CAP_BEAM_FORM: u64 = 11;
pub const MT_NIC_CAP_LOCATION: u64 = 12;
pub const MT_NIC_CAP_MUMIMO: u64 = 13;
pub const MT_NIC_CAP_BUFFER_MODE_INFO: u64 = 14;
pub const MT_NIC_CAP_HW_ADIE_VERSION: u64 = 20;
pub const MT_NIC_CAP_ANTSWP: u64 = 22;
pub const MT_NIC_CAP_WFDMA_REALLOC: u64 = 23;
pub const MT_NIC_CAP_6G: u64 = 24;
pub const MT_NIC_CAP_CHIP_CAP: u64 = 32;
pub const MT_NIC_CAP_EML_CAP: u64 = 34;
pub const UNI_SUSPEND_MODE_SETTING: u64 = 0;
pub const UNI_SUSPEND_WOW_CTRL: u64 = 1;
pub const UNI_SUSPEND_WOW_GPIO_PARAM: u64 = 2;
pub const UNI_SUSPEND_WOW_WAKEUP_PORT: u64 = 3;
pub const UNI_SUSPEND_WOW_PATTERN: u64 = 4;
pub const WOW_USB: u64 = 1;
pub const WOW_PCIE: u64 = 2;
pub const WOW_GPIO: u64 = 3;
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_bss_qos_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub qos: u8,
    pub pad: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac_beacon_loss_event {
    pub bss_idx: u8,
    pub reason: u8,
    pub pad: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_rssi_notify_event {
    pub rssi: [Le32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac_mcu_bss_event {
    pub bss_idx: u8,
    pub is_absent: u8,
    pub free_quota: u8,
    pub pad: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac_mcu_scan_channel {
    pub band: u8,
    pub channel_num: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_hw_scan_done {
    pub seq_num: u8,
    pub sparse_channel_num: u8,
    pub sparse_channel: mt76_connac_mcu_scan_channel,
    pub complete_channel_num: u8,
    pub current_state: u8,
    pub version: u8,
    pub pad: u8,
    pub beacon_scan_num: Le32,
    pub pno_enabled: u8,
    pub pad2: [u8; 3],
    pub sparse_channel_valid_num: u8,
    pub alpha2: [u8; 3],
    pub channel_num: [u8; 64],
    pub channel_idle_time: [Le16; 64],
    pub beacon_probe_num: [u8; 64],
    pub mdrdy_count: [u8; 64],
    pub beacon_2g_num: Le32,
    pub beacon_5g_num: Le32,
    pub channel_scan_time: [Le16; 64],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_sched_scan_done {
    pub seq_num: u8,
    pub status: u8,
    pub pad: Le16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_info_uni_bss_color {
    pub tag: Le16,
    pub len: Le16,
    pub enable: u8,
    pub bss_color: u8,
    pub rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_info_uni_he {
    pub tag: Le16,
    pub len: Le16,
    pub he_rts_thres: Le16,
    pub he_pe_duration: u8,
    pub su_disable: u8,
    pub max_nss_mcs: [Le16; 3],
    pub rsv: [u8; 2],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct bss_info_uni_mbssid {
    pub tag: Le16,
    pub len: Le16,
    pub max_indicator: u8,
    pub mbss_idx: u8,
    pub tx_bss_omac_idx: u8,
    pub rsv: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_wow_pattern_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub index: u8,
    pub enable: u8,
    pub data_len: u8,
    pub pad: u8,
    pub mask: [u8; 16],
    pub pattern: [u8; 128],
    pub rsv: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_wow_ctrl_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub cmd: u8,
    pub trigger: u8,
    pub wakeup_hif: u8,
    pub pad: u8,
    pub rsv: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_wow_gpio_param_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub gpio_pin: u8,
    pub trigger_lvl: u8,
    pub pad: [u8; 2],
    pub gpio_interval: Le32,
    pub rsv: [u8; 4],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_arpns_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub mode: u8,
    pub ips_num: u8,
    pub option: u8,
    pub pad: [u8; 1],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_suspend_tlv {
    pub tag: Le16,
    pub len: Le16,
    pub enable: u8,
    pub mdtim: u8,
    pub wow_suspend: u8,
    pub pad: [u8; 5],
}

pub const MT76_STA_INFO_STATE_NONE: u64 = 0;
pub const MT76_STA_INFO_STATE_AUTH: u64 = 1;
pub const MT76_STA_INFO_STATE_ASSOC: u64 = 2;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_connac_sku_tlv {
    pub channel: u8,
    pub pwr_limit: [i8; 161],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_tx_power_limit_tlv {
    pub ver: u8,
    pub pad0: u8,
    pub len: Le16,
    pub n_chan: u8,
    pub band: u8,
    pub last_msg: u8,
    pub pad1: u8,
    pub alpha2: [u8; 4],
    pub pad2: [u8; 32],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_config {
    pub id: Le16,
    pub r#type: u8,
    pub resp_type: u8,
    pub data_size: Le16,
    pub resv: Le16,
    pub data: [u8; 320],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_mcu_uni_event {
    pub cid: u8,
    pub pad: [u8; 3],
    pub status: Le32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct mt76_connac_mcu_reg_event {
    pub reg: Le32,
    pub val: Le32,
}
