/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT76_CONNAC_MCU_H
#define MT79_MT76_CONNAC_MCU_H
#include "mt79_types.h"

#define FW_FEATURE_SET_ENCRYPT 1ull
#define FW_FEATURE_SET_KEY_IDX 6ull
#define FW_FEATURE_ENCRY_MODE 16ull
#define FW_FEATURE_OVERRIDE_ADDR 32ull
#define FW_FEATURE_NON_DL 64ull
#define DL_MODE_ENCRYPT 1ull
#define DL_MODE_KEY_IDX 6ull
#define DL_MODE_RESET_SEC_IV 8ull
#define DL_MODE_WORKING_PDA_CR4 16ull
#define DL_MODE_VALID_RAM_ENTRY 32ull
#define DL_CONFIG_ENCRY_MODE_SEL 64ull
#define DL_MODE_NEED_RSP 2147483648ull
#define FW_START_OVERRIDE 1ull
#define FW_START_WORKING_PDA_CR4 4ull
#define FW_START_WORKING_PDA_DSP 8ull
#define PATCH_SEC_NOT_SUPPORT 4294967295ull
#define PATCH_SEC_TYPE_MASK 65535ull
#define PATCH_SEC_TYPE_INFO 2ull
#define PATCH_SEC_ENC_TYPE_MASK 4278190080ull
#define PATCH_SEC_ENC_TYPE_PLAIN 0ull
#define PATCH_SEC_ENC_TYPE_AES 1ull
#define PATCH_SEC_ENC_TYPE_SCRAMBLE 2ull
#define PATCH_SEC_ENC_SCRAMBLE_INFO_MASK 65535ull
#define PATCH_SEC_ENC_AES_KEY_MASK 255ull
#define SEC_TYPE_SUBSYS_MASK 16711680ull
#define SEC_TYPE_SUBSYS_CBMCU 65536ull
#define SEC_TYPE_SUBSYS_SEC_MASK 65535ull
#define SEC_TYPE_SUBSYS_SEC_IMG_SIGN 5ull
#define MCU_PQ_ID(p, q) ((p << 15ull) | (q << 10ull))
#define MCU_PKT_ID 160ull
#define FW_TYPE_DEFAULT 0ull
#define FW_TYPE_CLC 2ull
#define FW_TYPE_MAX_NUM 255ull
struct mt76_connac2_mcu_txd {
    __le32 txd[8];
    __le16 len;
    __le16 pq_id;
    u8 cid;
    u8 pkt_type;
    u8 set_query;
    u8 seq;
    u8 uc_d2b0_rev;
    u8 ext_cid;
    u8 s2d_index;
    u8 ext_cid_ack;
    u32 rsv[5];
};

#define EXTRA_INFO_VER 1ull
#define EXTRA_INFO_NEW 2ull
#define RA_LEGACY_OFDM 16320ull
#define RA_LEGACY_CCK 15ull
#define HT_MCS_MASK_NUM 10ull
#define STA_TYPE_STA 1ull
#define STA_TYPE_AP 2ull
#define STA_TYPE_ADHOC 4ull
#define STA_TYPE_WDS 16ull
#define STA_TYPE_BC 32ull
#define NETWORK_INFRA 65536ull
#define NETWORK_P2P 131072ull
#define NETWORK_IBSS 262144ull
#define NETWORK_WDS 2097152ull
#define NETWORK_NAN 4194304ull
#define SCAN_FUNC_RANDOM_MAC 1ull
#define SCAN_FUNC_RNR_SCAN 8ull
#define SCAN_FUNC_SPLIT_SCAN 32ull
#define CONNECTION_INFRA_STA 65537ull
#define CONNECTION_INFRA_AP 65538ull
#define CONNECTION_P2P_GC 131073ull
#define CONNECTION_P2P_GO 131074ull
#define CONNECTION_IBSS_ADHOC 262148ull
#define CONNECTION_WDS 2097168ull
#define CONNECTION_INFRA_BC 65568ull
#define CONNECTION_NAN 4194304ull
#define CONN_STATE_DISCONNECT 0ull
#define CONN_STATE_CONNECT 1ull
#define CONN_STATE_PORT_SECURE 2ull
#define STA_REC_HE_CAP_HTC 1ull
#define STA_REC_HE_CAP_BQR 2ull
#define STA_REC_HE_CAP_BSR 4ull
#define STA_REC_HE_CAP_OM 8ull
#define STA_REC_HE_CAP_AMSDU_IN_AMPDU 16ull
#define STA_REC_HE_CAP_DUAL_BAND 32ull
#define STA_REC_HE_CAP_LDPC 64ull
#define STA_REC_HE_CAP_TRIG_CQI_FK 128ull
#define STA_REC_HE_CAP_PARTIAL_BW_EXT_RANGE 256ull
#define STA_REC_HE_CAP_LE_EQ_80M_TX_STBC 512ull
#define STA_REC_HE_CAP_LE_EQ_80M_RX_STBC 1024ull
#define STA_REC_HE_CAP_GT_80M_TX_STBC 2048ull
#define STA_REC_HE_CAP_GT_80M_RX_STBC 4096ull
#define STA_REC_HE_CAP_SU_PPDU_1LTF_8US_GI 8192ull
#define STA_REC_HE_CAP_SU_MU_PPDU_4LTF_8US_GI 16384ull
#define STA_REC_HE_CAP_ER_SU_PPDU_1LTF_8US_GI 32768ull
#define STA_REC_HE_CAP_ER_SU_PPDU_4LTF_8US_GI 65536ull
#define STA_REC_HE_CAP_NDP_4LTF_3DOT2MS_GI 131072ull
#define STA_REC_HE_CAP_BW20_RU242_SUPPORT 262144ull
#define STA_REC_HE_CAP_TX_1024QAM_UNDER_RU242 524288ull
#define STA_REC_HE_CAP_RX_1024QAM_UNDER_RU242 1048576ull
#define PHY_MODE_A 1ull
#define PHY_MODE_B 2ull
#define PHY_MODE_G 4ull
#define PHY_MODE_GN 8ull
#define PHY_MODE_AN 16ull
#define PHY_MODE_AC 32ull
#define PHY_MODE_AX_24G 64ull
#define PHY_MODE_AX_5G 128ull
#define PHY_MODE_AX_6G 1ull
#define PHY_MODE_BE_24G 2ull
#define PHY_MODE_BE_5G 4ull
#define PHY_MODE_BE_6G 8ull
#define MODE_CCK 1ull
#define MODE_OFDM 2ull
#define MODE_HT 4ull
#define MODE_VHT 8ull
#define MODE_HE 16ull
#define MODE_EHT 32ull
#define STA_CAP_WMM 1ull
#define STA_CAP_SGI_20 16ull
#define STA_CAP_SGI_40 32ull
#define STA_CAP_TX_STBC 64ull
#define STA_CAP_RX_STBC 128ull
#define STA_CAP_VHT_SGI_80 65536ull
#define STA_CAP_VHT_SGI_160 131072ull
#define STA_CAP_VHT_TX_STBC 262144ull
#define STA_CAP_VHT_RX_STBC 524288ull
#define STA_CAP_VHT_LDPC 8388608ull
#define STA_CAP_LDPC 16777216ull
#define STA_CAP_HT 67108864ull
#define STA_CAP_VHT 134217728ull
#define STA_CAP_HE 268435456ull
#define HR_DSSS_ERP_BASIC_RATE 15ull
#define OFDM_BASIC_RATE 1344ull
#define MT_WTBL_RATE_TX_MODE 960ull
#define MT_WTBL_RATE_MCS 63ull
#define MT_WTBL_RATE_NSS 7168ull
#define MT_WTBL_RATE_HE_GI 240ull
#define MT_WTBL_RATE_GI 15ull
#define MT_WTBL_W5_CHANGE_BW_RATE 224ull
#define MT_WTBL_W5_SHORT_GI_20 256ull
#define MT_WTBL_W5_SHORT_GI_40 512ull
#define MT_WTBL_W5_SHORT_GI_80 1024ull
#define MT_WTBL_W5_SHORT_GI_160 2048ull
#define MT_WTBL_W5_BW_CAP 12288ull
#define MT_WTBL_W5_MPDU_FAIL_COUNT 58720256ull
#define MT_WTBL_W5_MPDU_OK_COUNT 469762048ull
#define MT_WTBL_W5_RATE_IDX 3758096384ull
#define MCU_UNI_CMD_EVENT 2ull
#define MCU_UNI_CMD_UNSOLICITED_EVENT 4ull
#define MCU_CMD_ACK 1ull
#define MCU_CMD_UNI 2ull
#define MCU_CMD_SET 4ull
#define MCU_CMD_UNI_EXT_ACK 7ull
#define MCU_CMD_UNI_QUERY_ACK 3ull
#define __MCU_CMD_FIELD_ID 255ull
#define __MCU_CMD_FIELD_EXT_ID 65280ull
#define __MCU_CMD_FIELD_QUERY 65536ull
#define __MCU_CMD_FIELD_UNI 131072ull
#define __MCU_CMD_FIELD_CE 262144ull
#define __MCU_CMD_FIELD_WA 524288ull
#define __MCU_CMD_FIELD_WM 1048576ull
#define UNI_WOW_DETECT_TYPE_MAGIC 1ull
#define UNI_WOW_DETECT_TYPE_ANY 2ull
#define UNI_WOW_DETECT_TYPE_DISCONNECT 4ull
#define UNI_WOW_DETECT_TYPE_GTK_REKEY_FAIL 8ull
#define UNI_WOW_DETECT_TYPE_BCN_LOST 16ull
#define UNI_WOW_DETECT_TYPE_SCH_SCAN_HIT 32ull
#define UNI_WOW_DETECT_TYPE_BITMAP 64ull
#define MT76_CONNAC_SCAN_DONE_EVENT_MAX_CHANNEL_NUM 64ull
struct mt76_connac2_mcu_uni_txd {
    __le32 txd[8];
    __le16 len;
    __le16 cid;
    u8 rsv;
    u8 pkt_type;
    u8 frag_n;
    u8 seq;
    __le16 checksum;
    u8 s2d_index;
    u8 option;
    u8 rsv1[4];
};

#define MT76_CONNAC_WOW_MASK_MAX_LEN 16ull
#define MT76_CONNAC_WOW_PATTEN_MAX_LEN 128ull
#define MT_SKU_POWER_LIMIT 161ull
#define MT76_CONNAC_MCU_STATUS_WLAN_FAILURE 3221225473ull
struct mt76_connac2_patch_hdr_desc {
    __be32 patch_ver;
    __be32 subsys;
    __be32 feature;
    __be32 n_region;
    __be32 crc;
    u32 rsv[11];
};

struct mt76_connac2_patch_hdr {
    u8 build_date[16];
    u8 platform[4];
    __be32 hw_sw_ver;
    __be32 patch_ver;
    __be16 checksum;
    u16 rsv;
    struct mt76_connac2_patch_hdr_desc desc;
} __attribute__((packed));

struct mt76_connac2_patch_sec_anon_3_info {
    __be32 addr;
    __be32 len;
    __be32 sec_key_idx;
    __be32 align_len;
    u32 rsv[9];
};

union mt76_connac2_patch_sec_anon_3 {
    __be32 spec[13];
    struct mt76_connac2_patch_sec_anon_3_info info;
};

struct mt76_connac2_patch_sec {
    __be32 type;
    __be32 offs;
    __be32 size;
    union mt76_connac2_patch_sec_anon_3 mt76_connac2_patch_sec_anon_3;
} __attribute__((packed));

struct mt76_connac2_fw_trailer {
    u8 chip_id;
    u8 eco_code;
    u8 n_region;
    u8 format_ver;
    u8 format_flag;
    u8 rsv[2];
    u8 fw_ver[10];
    u8 build_date[15];
    __le32 crc;
} __attribute__((packed));

struct mt76_connac2_fw_region {
    __le32 decomp_crc;
    __le32 decomp_len;
    __le32 decomp_blk_sz;
    u8 rsv[4];
    __le32 addr;
    __le32 len;
    u8 feature_set;
    u8 type;
    u8 rsv1[14];
} __attribute__((packed));

struct mt76_connac3_multi_header_v2_sec_raw_format {
    __le32 type;
    __le32 offset;
    __le32 size;
    u8 spec[52];
} __attribute__((packed));

struct mt76_connac3_multi_header_v2_raw_format {
    u8 pack_time[20];
    u8 chip_id_eco_ver[8];
    __le32 patch_ver;
    u8 global_desc_head[4];
    __le32 global_subsys;
    u8 rsv2[4];
    __le32 sec_num;
    u8 rsv3[48];
} __attribute__((packed));

struct tlv {
    __le16 tag;
    __le16 len;
} __attribute__((packed));

struct bss_info_omac {
    __le16 tag;
    __le16 len;
    u8 hw_bss_idx;
    u8 omac_idx;
    u8 band_idx;
    u8 rsv0;
    __le32 conn_type;
    u32 rsv1;
} __attribute__((packed));

struct bss_info_rf_ch {
    __le16 tag;
    __le16 len;
    u8 pri_ch;
    u8 center_ch0;
    u8 center_ch1;
    u8 bw;
    u8 he_ru26_block;
    u8 he_all_disable;
    u8 rsv[2];
} __attribute__((packed));

struct bss_info_ext_bss {
    __le16 tag;
    __le16 len;
    __le32 mbss_tsf_offset;
    u8 rsv[8];
} __attribute__((packed));

#define BSS_INFO_OMAC 0ull
#define BSS_INFO_BASIC 1ull
#define BSS_INFO_RF_CH 2ull
#define BSS_INFO_PM 3ull
#define BSS_INFO_UAPSD 4ull
#define BSS_INFO_ROAM_DETECT 5ull
#define BSS_INFO_LQ_RM 6ull
#define BSS_INFO_EXT_BSS 7ull
#define BSS_INFO_BMC_RATE 8ull
#define BSS_INFO_SYNC_MODE 9ull
#define BSS_INFO_RA 10ull
#define BSS_INFO_HW_AMSDU 11ull
#define BSS_INFO_BSS_COLOR 12ull
#define BSS_INFO_HE_BASIC 13ull
#define BSS_INFO_PROTECT_INFO 14ull
#define BSS_INFO_OFFLOAD 15ull
#define BSS_INFO_11V_MBSSID 16ull
#define BSS_INFO_MAX_NUM 17ull
struct sta_ntlv_hdr {
    u8 rsv[2];
    __le16 tlv_num;
} __attribute__((packed));

struct sta_req_hdr {
    u8 bss_idx;
    u8 wlan_idx_lo;
    __le16 tlv_num;
    u8 is_tlv_append;
    u8 muar_idx;
    u8 wlan_idx_hi;
    u8 rsv;
} __attribute__((packed));

struct sta_rec_ht {
    __le16 tag;
    __le16 len;
    __le16 ht_cap;
    u16 rsv;
} __attribute__((packed));

struct sta_rec_vht {
    __le16 tag;
    __le16 len;
    __le32 vht_cap;
    __le16 vht_rx_mcs_map;
    __le16 vht_tx_mcs_map;
    u8 rts_bw_sig;
    u8 rsv[3];
} __attribute__((packed));

struct sta_rec_uapsd {
    __le16 tag;
    __le16 len;
    u8 dac_map;
    u8 tac_map;
    u8 max_sp;
    u8 rsv0;
    __le16 listen_interval;
    u8 rsv1[2];
} __attribute__((packed));

struct sta_rec_ba {
    __le16 tag;
    __le16 len;
    u8 tid;
    u8 ba_type;
    u8 amsdu;
    u8 ba_en;
    __le16 ssn;
    __le16 winsize;
} __attribute__((packed));

struct sta_rec_he {
    __le16 tag;
    __le16 len;
    __le32 he_cap;
    u8 t_frame_dur;
    u8 max_ampdu_exp;
    u8 bw_set;
    u8 device_class;
    u8 dcm_tx_mode;
    u8 dcm_tx_max_nss;
    u8 dcm_rx_mode;
    u8 dcm_rx_max_nss;
    u8 dcm_max_ru;
    u8 punc_pream_rx;
    u8 pkt_ext;
    u8 rsv1;
    __le16 max_nss_mcs[3];
    u8 rsv2[2];
} __attribute__((packed));

struct sta_rec_he_v2 {
    __le16 tag;
    __le16 len;
    u8 he_mac_cap[6];
    u8 he_phy_cap[11];
    u8 pkt_ext;
    __le16 max_nss_mcs[3];
} __attribute__((packed));

struct sta_rec_amsdu {
    __le16 tag;
    __le16 len;
    u8 max_amsdu_num;
    u8 max_mpdu_size;
    u8 amsdu_en;
    u8 rsv;
} __attribute__((packed));

struct sta_rec_state {
    __le16 tag;
    __le16 len;
    __le32 flags;
    u8 state;
    u8 vht_opmode;
    u8 action;
    u8 rsv[1];
} __attribute__((packed));

struct sta_rec_ra_info {
    __le16 tag;
    __le16 len;
    __le16 legacy;
    u8 rx_mcs_bitmask[10];
} __attribute__((packed));

struct sta_rec_phy {
    __le16 tag;
    __le16 len;
    __le16 basic_rate;
    u8 phy_type;
    u8 ampdu;
    u8 rts_policy;
    u8 rcpi;
    u8 max_ampdu_len;
    u8 rsv[1];
} __attribute__((packed));

struct sta_rec_he_6g_capa {
    __le16 tag;
    __le16 len;
    __le16 capa;
    u8 rsv[2];
} __attribute__((packed));

struct sta_rec_pn_info {
    __le16 tag;
    __le16 len;
    u8 pn[6];
    u8 tsc_type;
    u8 rsv;
} __attribute__((packed));

struct sec_key {
    u8 cipher_id;
    u8 cipher_len;
    u8 key_id;
    u8 key_len;
    u8 key[32];
};

struct sta_rec_sec {
    __le16 tag;
    __le16 len;
    u8 add;
    u8 n_cipher;
    u8 rsv[2];
    struct sec_key key[2];
} __attribute__((packed));

#define sta_rec_bf_mem_col_MASK 63ull
#define sta_rec_bf_mem_col_SHIFT 0ull
#define sta_rec_bf_mem_row_msb_MASK 192ull
#define sta_rec_bf_mem_row_msb_SHIFT 6ull
struct sta_rec_bf_mem {
    u8 row;
    u8 bitfield_0;
};

struct sta_rec_bf {
    __le16 tag;
    __le16 len;
    __le16 pfmu;
    bool su_mu;
    u8 bf_cap;
    u8 sounding_phy;
    u8 ndpa_rate;
    u8 ndp_rate;
    u8 rept_poll_rate;
    u8 tx_mode;
    u8 ncol;
    u8 nrow;
    u8 bw;
    u8 mem_total;
    u8 mem_20m;
    struct sta_rec_bf_mem mem[4];
    __le16 smart_ant;
    u8 se_idx;
    u8 auto_sounding;
    u8 ibf_timeout;
    u8 ibf_dbw;
    u8 ibf_ncol;
    u8 ibf_nrow;
    u8 nrow_gt_bw80;
    u8 ncol_gt_bw80;
    u8 ru_start_idx;
    u8 ru_end_idx;
    bool trigger_su;
    bool trigger_mu;
    bool ng16_su;
    bool ng16_mu;
    bool codebook42_su;
    bool codebook75_mu;
    u8 he_ltf;
    u8 rsv[3];
} __attribute__((packed));

struct sta_rec_bfee {
    __le16 tag;
    __le16 len;
    bool fb_identity_matrix;
    bool ignore_feedback;
    u8 rsv[2];
} __attribute__((packed));

struct sta_rec_muru_cfg {
    bool ofdma_dl_en;
    bool ofdma_ul_en;
    bool mimo_dl_en;
    bool mimo_ul_en;
    u8 rsv[4];
};

struct sta_rec_muru_ofdma_dl {
    u8 punc_pream_rx;
    bool he_20m_in_40m_2g;
    bool he_20m_in_160m;
    bool he_80m_in_160m;
    bool lt16_sigb;
    bool rx_su_comp_sigb;
    bool rx_su_non_comp_sigb;
    u8 rsv;
};

struct sta_rec_muru_ofdma_ul {
    u8 t_frame_dur;
    u8 mu_cascading;
    u8 uo_ra;
    u8 he_2x996_tone;
    u8 rx_t_frame_11ac;
    u8 rx_ctrl_frame_to_mbss;
    u8 rsv[2];
};

struct sta_rec_muru_mimo_dl {
    bool vht_mu_bfee;
    bool partial_bw_dl_mimo;
    u8 rsv[2];
};

struct sta_rec_muru_mimo_ul {
    bool full_ul_mimo;
    bool partial_ul_mimo;
    u8 rsv[2];
};

struct sta_rec_muru {
    __le16 tag;
    __le16 len;
    struct sta_rec_muru_cfg cfg;
    struct sta_rec_muru_ofdma_dl ofdma_dl;
    struct sta_rec_muru_ofdma_ul ofdma_ul;
    struct sta_rec_muru_mimo_dl mimo_dl;
    struct sta_rec_muru_mimo_ul mimo_ul;
} __attribute__((packed));

struct sta_rec_remove {
    __le16 tag;
    __le16 len;
    u8 action;
    u8 pad[3];
} __attribute__((packed));

struct sta_phy {
    u8 type;
    u8 flag;
    u8 stbc;
    u8 sgi;
    u8 bw;
    u8 ldpc;
    u8 mcs;
    u8 nss;
    u8 he_ltf;
};

struct sta_rec_ra {
    __le16 tag;
    __le16 len;
    u8 valid;
    u8 auto_rate;
    u8 phy_mode;
    u8 channel;
    u8 bw;
    u8 disable_cck;
    u8 ht_mcs32;
    u8 ht_gf;
    u8 ht_mcs[4];
    u8 mmps_mode;
    u8 gband_256;
    u8 af;
    u8 auth_wapi_mode;
    u8 rate_len;
    u8 supp_mode;
    u8 supp_cck_rate;
    u8 supp_ofdm_rate;
    __le32 supp_ht_mcs;
    __le16 supp_vht_mcs[4];
    u8 op_mode;
    u8 op_vht_chan_width;
    u8 op_vht_rx_nss;
    u8 op_vht_rx_nss_type;
    __le32 sta_cap;
    struct sta_phy phy;
} __attribute__((packed));

struct sta_rec_ra_fixed {
    __le16 tag;
    __le16 len;
    __le32 field;
    u8 op_mode;
    u8 op_vht_chan_width;
    u8 op_vht_rx_nss;
    u8 op_vht_rx_nss_type;
    struct sta_phy phy;
    u8 spe_idx;
    u8 short_preamble;
    u8 is_5g;
    u8 mmps_mode;
} __attribute__((packed));

struct sta_rec_tx_proc {
    __le16 tag;
    __le16 len;
    __le32 flag;
} __attribute__((packed));

struct sta_rec_eml_op {
    __le16 tag;
    __le16 len;
    u8 link_bitmap;
    u8 link_ant_num[3];
} __attribute__((packed));

struct wtbl_req_hdr {
    u8 wlan_idx_lo;
    u8 operation;
    __le16 tlv_num;
    u8 wlan_idx_hi;
    u8 rsv[3];
} __attribute__((packed));

struct wtbl_rx {
    __le16 tag;
    __le16 len;
    u8 rcid;
    u8 rca1;
    u8 rca2;
    u8 rv;
    u8 rsv[4];
} __attribute__((packed));

struct wtbl_ht {
    __le16 tag;
    __le16 len;
    u8 ht;
    u8 ldpc;
    u8 af;
    u8 mm;
    u8 rsv[4];
} __attribute__((packed));

struct wtbl_vht {
    __le16 tag;
    __le16 len;
    u8 ldpc;
    u8 dyn_bw;
    u8 vht;
    u8 txop_ps;
    u8 rsv[4];
} __attribute__((packed));

struct wtbl_tx_ps {
    __le16 tag;
    __le16 len;
    u8 txps;
    u8 rsv[3];
} __attribute__((packed));

struct wtbl_hdr_trans {
    __le16 tag;
    __le16 len;
    u8 to_ds;
    u8 from_ds;
    u8 no_rx_trans;
    u8 rsv;
} __attribute__((packed));

struct wtbl_smps {
    __le16 tag;
    __le16 len;
    u8 smps;
    u8 rsv[3];
} __attribute__((packed));

struct wtbl_bf {
    __le16 tag;
    __le16 len;
    u8 ibf;
    u8 ebf;
    u8 ibf_vht;
    u8 ebf_vht;
    u8 gid;
    u8 pfmu_idx;
    u8 rsv[2];
} __attribute__((packed));

struct wtbl_pn {
    __le16 tag;
    __le16 len;
    u8 pn[6];
    u8 rsv[2];
} __attribute__((packed));

struct wtbl_spe {
    __le16 tag;
    __le16 len;
    u8 spe_idx;
    u8 rsv[3];
} __attribute__((packed));

struct wtbl_raw {
    __le16 tag;
    __le16 len;
    u8 wtbl_idx;
    u8 dw;
    u8 rsv[2];
    __le32 msk;
    __le32 val;
} __attribute__((packed));

#define STA_REC_BASIC 0ull
#define STA_REC_RA 1ull
#define STA_REC_RA_CMM_INFO 2ull
#define STA_REC_RA_UPDATE 3ull
#define STA_REC_BF 4ull
#define STA_REC_AMSDU 5ull
#define STA_REC_BA 6ull
#define STA_REC_STATE 7ull
#define STA_REC_TX_PROC 8ull
#define STA_REC_HT 9ull
#define STA_REC_VHT 10ull
#define STA_REC_APPS 11ull
#define STA_REC_KEY 12ull
#define STA_REC_WTBL 13ull
#define STA_REC_HE 14ull
#define STA_REC_HW_AMSDU 15ull
#define STA_REC_WTBL_AADOM 16ull
#define STA_REC_KEY_V2 17ull
#define STA_REC_MURU 18ull
#define STA_REC_MUEDCA 19ull
#define STA_REC_BFEE 20ull
#define STA_REC_PHY 21ull
#define STA_REC_HE_6G 23ull
#define STA_REC_HE_V2 25ull
#define STA_REC_MLD 32ull
#define STA_REC_EHT_MLD 33ull
#define STA_REC_EHT 34ull
#define STA_REC_MLD_OFF 35ull
#define STA_REC_REMOVE 37ull
#define STA_REC_PN_INFO 38ull
#define STA_REC_KEY_V3 39ull
#define STA_REC_HDRT 40ull
#define STA_REC_EML_OP 41ull
#define STA_REC_HDR_TRANS 43ull
#define STA_REC_PS_LEAVE 69ull
#define STA_REC_MAX_NUM 70ull
#define WTBL_GENERIC 0ull
#define WTBL_RX 1ull
#define WTBL_HT 2ull
#define WTBL_VHT 3ull
#define WTBL_PEER_PS 4ull
#define WTBL_TX_PS 5ull
#define WTBL_HDR_TRANS 6ull
#define WTBL_SEC_KEY 7ull
#define WTBL_BA 8ull
#define WTBL_RDG 9ull
#define WTBL_PROTECT 10ull
#define WTBL_CLEAR 11ull
#define WTBL_BF 12ull
#define WTBL_SMPS 13ull
#define WTBL_RAW_DATA 14ull
#define WTBL_PN 15ull
#define WTBL_SPE 16ull
#define WTBL_MAX_NUM 17ull
#define PHY_TYPE_HR_DSSS_INDEX 0ull
#define PHY_TYPE_ERP_INDEX 1ull
#define PHY_TYPE_ERP_P2P_INDEX 2ull
#define PHY_TYPE_OFDM_INDEX 3ull
#define PHY_TYPE_HT_INDEX 4ull
#define PHY_TYPE_VHT_INDEX 5ull
#define PHY_TYPE_HE_INDEX 6ull
#define PHY_TYPE_BE_INDEX 7ull
#define PHY_TYPE_INDEX_NUM 8ull
#define WTBL_RESET_AND_SET 1ull
#define WTBL_SET 2ull
#define WTBL_QUERY 3ull
#define WTBL_RESET_ALL 4ull
#define MT_BA_TYPE_INVALID 0ull
#define MT_BA_TYPE_ORIGINATOR 1ull
#define MT_BA_TYPE_RECIPIENT 2ull
#define RST_BA_MAC_TID_MATCH 0ull
#define RST_BA_MAC_MATCH 1ull
#define RST_BA_NO_MATCH 2ull
#define DEV_INFO_ACTIVE 0ull
#define DEV_INFO_MAX_NUM 1ull
#define MCU_EVENT_TARGET_ADDRESS_LEN 1ull
#define MCU_EVENT_FW_START 1ull
#define MCU_EVENT_GENERIC 1ull
#define MCU_EVENT_ACCESS_REG 2ull
#define MCU_EVENT_MT_PATCH_SEM 4ull
#define MCU_EVENT_REG_ACCESS 5ull
#define MCU_EVENT_LP_INFO 7ull
#define MCU_EVENT_SCAN_DONE 13ull
#define MCU_EVENT_TX_DONE 15ull
#define MCU_EVENT_ROC 16ull
#define MCU_EVENT_BSS_ABSENCE 17ull
#define MCU_EVENT_BSS_BEACON_LOSS 19ull
#define MCU_EVENT_CH_PRIVILEGE 24ull
#define MCU_EVENT_SCHED_SCAN_DONE 35ull
#define MCU_EVENT_DBG_MSG 39ull
#define MCU_EVENT_RSSI_NOTIFY 150ull
#define MCU_EVENT_TXPWR 208ull
#define MCU_EVENT_EXT 237ull
#define MCU_EVENT_RESTART_DL 239ull
#define MCU_EVENT_COREDUMP 240ull
#define MCU_EXT_EVENT_PS_SYNC 5ull
#define MCU_EXT_EVENT_FW_LOG_2_HOST 19ull
#define MCU_EXT_EVENT_THERMAL_PROTECT 34ull
#define MCU_EXT_EVENT_ASSERT_DUMP 35ull
#define MCU_EXT_EVENT_RDD_REPORT 58ull
#define MCU_EXT_EVENT_CSA_NOTIFY 79ull
#define MCU_EXT_EVENT_WA_TX_STAT 116ull
#define MCU_EXT_EVENT_BCC_NOTIFY 117ull
#define MCU_EXT_EVENT_WF_RF_PIN_CTRL 154ull
#define MCU_EXT_EVENT_MURU_CTRL 159ull
#define MCU_UNI_EVENT_RESULT 1ull
#define MCU_UNI_EVENT_HIF_CTRL 3ull
#define MCU_UNI_EVENT_FW_LOG_2_HOST 4ull
#define MCU_UNI_EVENT_ACCESS_REG 6ull
#define MCU_UNI_EVENT_IE_COUNTDOWN 9ull
#define MCU_UNI_EVENT_COREDUMP 10ull
#define MCU_UNI_EVENT_BSS_BEACON_LOSS 12ull
#define MCU_UNI_EVENT_PS_SYNC 13ull
#define MCU_UNI_EVENT_SCAN_DONE 14ull
#define MCU_UNI_EVENT_RDD_REPORT 17ull
#define MCU_UNI_EVENT_ROC 39ull
#define MCU_UNI_EVENT_MBMC 40ull
#define MCU_UNI_EVENT_TX_DONE 45ull
#define MCU_UNI_EVENT_THERMAL 53ull
#define MCU_UNI_EVENT_RSSI_MONITOR 65ull
#define MCU_UNI_EVENT_NIC_CAPAB 67ull
#define MCU_UNI_EVENT_NAN 86ull
#define MCU_UNI_EVENT_WED_RRO 87ull
#define MCU_UNI_EVENT_PER_STA_INFO 109ull
#define MCU_UNI_EVENT_ALL_STA_INFO 110ull
#define MCU_UNI_EVENT_SDO 131ull
#define MCU_Q_QUERY 0ull
#define MCU_Q_SET 1ull
#define MCU_Q_RESERVED 2ull
#define MCU_Q_NA 3ull
#define MCU_S2D_H2N 0ull
#define MCU_S2D_C2N 1ull
#define MCU_S2D_H2C 2ull
#define MCU_S2D_H2CN 3ull
#define PATCH_NOT_DL_SEM_FAIL 0ull
#define PATCH_IS_DL 1ull
#define PATCH_NOT_DL_SEM_SUCCESS 2ull
#define PATCH_REL_SEM_SUCCESS 3ull
#define CB_PATCH_GET_SEM_NEED_PATCH 0ull
#define CB_PATCH_IS_DL 1ull
#define CB_PATCH_NO_SEM_NEED_PATCH 2ull
#define CB_PATCH_REL_SEM_SUCCESS 3ull
#define FW_STATE_INITIAL 0ull
#define FW_STATE_FW_DOWNLOAD 1ull
#define FW_STATE_NORMAL_OPERATION 2ull
#define FW_STATE_NORMAL_TRX 3ull
#define FW_STATE_RDY 7ull
#define CH_SWITCH_NORMAL 0ull
#define CH_SWITCH_SCAN 3ull
#define CH_SWITCH_MCC 4ull
#define CH_SWITCH_DFS 5ull
#define CH_SWITCH_BACKGROUND_SCAN_START 6ull
#define CH_SWITCH_BACKGROUND_SCAN_RUNNING 7ull
#define CH_SWITCH_BACKGROUND_SCAN_STOP 8ull
#define CH_SWITCH_SCAN_BYPASS_DPD 9ull
#define THERMAL_SENSOR_TEMP_QUERY 0ull
#define THERMAL_SENSOR_MANUAL_CTRL 1ull
#define THERMAL_SENSOR_INFO_QUERY 2ull
#define THERMAL_SENSOR_TASK_CTRL 3ull
#define MCU_CIPHER_NONE 0ull
#define MCU_CIPHER_WEP40 1ull
#define MCU_CIPHER_WEP104 2ull
#define MCU_CIPHER_WEP128 3ull
#define MCU_CIPHER_TKIP 4ull
#define MCU_CIPHER_AES_CCMP 5ull
#define MCU_CIPHER_CCMP_256 6ull
#define MCU_CIPHER_GCMP 7ull
#define MCU_CIPHER_GCMP_256 8ull
#define MCU_CIPHER_WAPI 9ull
#define MCU_CIPHER_BIP_CMAC_128 10ull
#define MCU_CIPHER_BIP_CMAC_256 11ull
#define MCU_CIPHER_BCN_PROT_CMAC_128 12ull
#define MCU_CIPHER_BCN_PROT_CMAC_256 13ull
#define MCU_CIPHER_BCN_PROT_GMAC_128 14ull
#define MCU_CIPHER_BCN_PROT_GMAC_256 15ull
#define MCU_CIPHER_BIP_GMAC_128 16ull
#define MCU_CIPHER_BIP_GMAC_256 17ull
#define EE_MODE_EFUSE 0ull
#define EE_MODE_BUFFER 1ull
#define EE_FORMAT_BIN 0ull
#define EE_FORMAT_WHOLE 1ull
#define EE_FORMAT_MULTIPLE 2ull
#define MCU_PHY_STATE_TX_RATE 0ull
#define MCU_PHY_STATE_RX_RATE 1ull
#define MCU_PHY_STATE_RSSI 2ull
#define MCU_PHY_STATE_CONTENTION_RX_RATE 3ull
#define MCU_PHY_STATE_OFDMLQ_CNINFO 4ull
#define MCU_EXT_CMD_EFUSE_ACCESS 1ull
#define MCU_EXT_CMD_RF_REG_ACCESS 2ull
#define MCU_EXT_CMD_RF_TEST 4ull
#define MCU_EXT_CMD_ID_RADIO_ON_OFF_CTRL 5ull
#define MCU_EXT_CMD_PM_STATE_CTRL 7ull
#define MCU_EXT_CMD_CHANNEL_SWITCH 8ull
#define MCU_EXT_CMD_SET_TX_POWER_CTRL 17ull
#define MCU_EXT_CMD_FW_LOG_2_HOST 19ull
#define MCU_EXT_CMD_TXBF_ACTION 30ull
#define MCU_EXT_CMD_EFUSE_BUFFER_MODE 33ull
#define MCU_EXT_CMD_THERMAL_PROT 35ull
#define MCU_EXT_CMD_STA_REC_UPDATE 37ull
#define MCU_EXT_CMD_BSS_INFO_UPDATE 38ull
#define MCU_EXT_CMD_EDCA_UPDATE 39ull
#define MCU_EXT_CMD_DEV_INFO_UPDATE 42ull
#define MCU_EXT_CMD_THERMAL_CTRL 44ull
#define MCU_EXT_CMD_WTBL_UPDATE 50ull
#define MCU_EXT_CMD_SET_DRR_CTRL 54ull
#define MCU_EXT_CMD_SET_RDD_CTRL 58ull
#define MCU_EXT_CMD_ATE_CTRL 61ull
#define MCU_EXT_CMD_PROTECT_CTRL 62ull
#define MCU_EXT_CMD_DBDC_CTRL 69ull
#define MCU_EXT_CMD_MAC_INIT_CTRL 70ull
#define MCU_EXT_CMD_RX_HDR_TRANS 71ull
#define MCU_EXT_CMD_MUAR_UPDATE 72ull
#define MCU_EXT_CMD_BCN_OFFLOAD 73ull
#define MCU_EXT_CMD_RX_AIRTIME_CTRL 74ull
#define MCU_EXT_CMD_SET_RX_PATH 78ull
#define MCU_EXT_CMD_EFUSE_FREE_BLOCK 79ull
#define MCU_EXT_CMD_TX_POWER_FEATURE_CTRL 88ull
#define MCU_EXT_CMD_RXDCOC_CAL 89ull
#define MCU_EXT_CMD_GET_MIB_INFO 90ull
#define MCU_EXT_CMD_TXDPD_CAL 96ull
#define MCU_EXT_CMD_CAL_CACHE 103ull
#define MCU_EXT_CMD_RED_ENABLE 104ull
#define MCU_EXT_CMD_CP_SUPPORT 117ull
#define MCU_EXT_CMD_SET_RADAR_TH 124ull
#define MCU_EXT_CMD_SET_RDD_PATTERN 125ull
#define MCU_EXT_CMD_MWDS_SUPPORT 128ull
#define MCU_EXT_CMD_SET_SER_TRIGGER 129ull
#define MCU_EXT_CMD_TWT_AGRT_UPDATE 148ull
#define MCU_EXT_CMD_FW_DBG_CTRL 149ull
#define MCU_EXT_CMD_OFFCH_SCAN_CTRL 154ull
#define MCU_EXT_CMD_SET_RDD_TH 157ull
#define MCU_EXT_CMD_MURU_CTRL 159ull
#define MCU_EXT_CMD_SET_SPR 168ull
#define MCU_EXT_CMD_GROUP_PRE_CAL_INFO 171ull
#define MCU_EXT_CMD_DPD_PRE_CAL_INFO 172ull
#define MCU_EXT_CMD_PHY_STAT_INFO 173ull
#define MCU_EXT_CMD_WF_RF_PIN_CTRL 189ull
#define MCU_UNI_CMD_DEV_INFO_UPDATE 1ull
#define MCU_UNI_CMD_BSS_INFO_UPDATE 2ull
#define MCU_UNI_CMD_STA_REC_UPDATE 3ull
#define MCU_UNI_CMD_EDCA_UPDATE 4ull
#define MCU_UNI_CMD_SUSPEND 5ull
#define MCU_UNI_CMD_OFFLOAD 6ull
#define MCU_UNI_CMD_HIF_CTRL 7ull
#define MCU_UNI_CMD_BAND_CONFIG 8ull
#define MCU_UNI_CMD_REPT_MUAR 9ull
#define MCU_UNI_CMD_WSYS_CONFIG 11ull
#define MCU_UNI_CMD_REG_ACCESS 13ull
#define MCU_UNI_CMD_CHIP_CONFIG 14ull
#define MCU_UNI_CMD_POWER_CTRL 15ull
#define MCU_UNI_CMD_RX_HDR_TRANS 18ull
#define MCU_UNI_CMD_SER 19ull
#define MCU_UNI_CMD_TWT 20ull
#define MCU_UNI_CMD_SET_DOMAIN_INFO 21ull
#define MCU_UNI_CMD_SCAN_REQ 22ull
#define MCU_UNI_CMD_RDD_CTRL 25ull
#define MCU_UNI_CMD_GET_MIB_INFO 34ull
#define MCU_UNI_CMD_GET_STAT_INFO 35ull
#define MCU_UNI_CMD_SNIFFER 36ull
#define MCU_UNI_CMD_SR 37ull
#define MCU_UNI_CMD_ROC 39ull
#define MCU_UNI_CMD_SET_DBDC_PARMS 40ull
#define MCU_UNI_CMD_TXPOWER 43ull
#define MCU_UNI_CMD_SET_POWER_LIMIT 44ull
#define MCU_UNI_CMD_EFUSE_CTRL 45ull
#define MCU_UNI_CMD_RA 47ull
#define MCU_UNI_CMD_MURU 49ull
#define MCU_UNI_CMD_TESTMODE_RX_STAT 50ull
#define MCU_UNI_CMD_BF 51ull
#define MCU_UNI_CMD_CHANNEL_SWITCH 52ull
#define MCU_UNI_CMD_THERMAL 53ull
#define MCU_UNI_CMD_VOW 55ull
#define MCU_UNI_CMD_FIXED_RATE_TABLE 64ull
#define MCU_UNI_CMD_RSSI_MONITOR 65ull
#define MCU_UNI_CMD_TESTMODE_CTRL 70ull
#define MCU_UNI_CMD_NAN 86ull
#define MCU_UNI_CMD_RRO 87ull
#define MCU_UNI_CMD_OFFCH_SCAN_CTRL 88ull
#define MCU_UNI_CMD_PER_STA_INFO 109ull
#define MCU_UNI_CMD_ALL_STA_INFO 110ull
#define MCU_UNI_CMD_ASSERT_DUMP 111ull
#define MCU_UNI_CMD_EXT_EEPROM_CTRL 116ull
#define MCU_UNI_CMD_RADIO_STATUS 128ull
#define MCU_UNI_CMD_MLD 130ull
#define MCU_UNI_CMD_SDO 136ull
#define MCU_CMD_TARGET_ADDRESS_LEN_REQ 1ull
#define MCU_CMD_FW_START_REQ 2ull
#define MCU_CMD_INIT_ACCESS_REG 3ull
#define MCU_CMD_NIC_POWER_CTRL 4ull
#define MCU_CMD_PATCH_START_REQ 5ull
#define MCU_CMD_PATCH_FINISH_REQ 7ull
#define MCU_CMD_PATCH_SEM_CONTROL 16ull
#define MCU_CMD_CB_PATCH_SEM_CONTROL 48ull
#define MCU_CMD_CB_PATCH_START_REQ 49ull
#define MCU_CMD_CB_PATCH_FINISH_REQ 50ull
#define MCU_CMD_WA_PARAM 196ull
#define MCU_CMD_EXT_CID 237ull
#define MCU_CMD_FW_SCATTER 238ull
#define MCU_CMD_RESTART_DL_REQ 239ull
#define MCU_CE_CMD_TEST_CTRL 1ull
#define MCU_CE_CMD_START_HW_SCAN 3ull
#define MCU_CE_CMD_SET_PS_PROFILE 5ull
#define MCU_CE_CMD_SET_RX_FILTER 10ull
#define MCU_CE_CMD_SET_CHAN_DOMAIN 15ull
#define MCU_CE_CMD_SET_BSS_CONNECTED 22ull
#define MCU_CE_CMD_SET_BSS_ABORT 23ull
#define MCU_CE_CMD_CANCEL_HW_SCAN 27ull
#define MCU_CE_CMD_SET_ROC 28ull
#define MCU_CE_CMD_SET_EDCA_PARMS 29ull
#define MCU_CE_CMD_SET_P2P_OPPPS 51ull
#define MCU_CE_CMD_SET_CLC 92ull
#define MCU_CE_CMD_SET_RATE_TX_POWER 93ull
#define MCU_CE_CMD_SCHED_SCAN_ENABLE 97ull
#define MCU_CE_CMD_SCHED_SCAN_REQ 98ull
#define MCU_CE_CMD_GET_NIC_CAPAB 138ull
#define MCU_CE_CMD_RSSI_MONITOR 161ull
#define MCU_CE_CMD_SET_MU_EDCA_PARMS 176ull
#define MCU_CE_CMD_REG_WRITE 192ull
#define MCU_CE_CMD_REG_READ 192ull
#define MCU_CE_CMD_CHIP_CONFIG 202ull
#define MCU_CE_CMD_FWLOG_2_HOST 197ull
#define MCU_CE_CMD_GET_WTBL 205ull
#define MCU_CE_CMD_GET_TXPWR 208ull
#define MCU_CE_CMD_SET_REGD_CH 209ull
#define PATCH_SEM_RELEASE 0ull
#define PATCH_SEM_GET 1ull
#define UNI_BSS_INFO_BASIC 0ull
#define UNI_BSS_INFO_RA 1ull
#define UNI_BSS_INFO_RLM 2ull
#define UNI_BSS_INFO_PROTECT_INFO 3ull
#define UNI_BSS_INFO_BSS_COLOR 4ull
#define UNI_BSS_INFO_HE_BASIC 5ull
#define UNI_BSS_INFO_11V_MBSSID 6ull
#define UNI_BSS_INFO_BCN_CONTENT 7ull
#define UNI_BSS_INFO_BCN_CSA 8ull
#define UNI_BSS_INFO_BCN_BCC 9ull
#define UNI_BSS_INFO_BCN_MBSSID 10ull
#define UNI_BSS_INFO_RATE 11ull
#define UNI_BSS_INFO_QBSS 15ull
#define UNI_BSS_INFO_SEC 16ull
#define UNI_BSS_INFO_BCN_PROT 17ull
#define UNI_BSS_INFO_TXCMD 18ull
#define UNI_BSS_INFO_UAPSD 19ull
#define UNI_BSS_INFO_PS 21ull
#define UNI_BSS_INFO_BCNFT 22ull
#define UNI_BSS_INFO_IFS_TIME 23ull
#define UNI_BSS_INFO_OFFLOAD 25ull
#define UNI_BSS_INFO_MLD 26ull
#define UNI_BSS_INFO_PM_DISABLE 27ull
#define UNI_BSS_INFO_EHT 30ull
#define UNI_BSS_INFO_MLD_LINK_OP 36ull
#define UNI_OFFLOAD_OFFLOAD_ARP 0ull
#define UNI_OFFLOAD_OFFLOAD_ND 1ull
#define UNI_OFFLOAD_OFFLOAD_GTK_REKEY 2ull
#define UNI_OFFLOAD_OFFLOAD_BMC_RPY_DETECT 3ull
#define UNI_ALL_STA_TXRX_RATE 0ull
#define UNI_ALL_STA_TX_STAT 1ull
#define UNI_ALL_STA_TXRX_ADM_STAT 2ull
#define UNI_ALL_STA_TXRX_AIR_TIME 3ull
#define UNI_ALL_STA_DATA_TX_RETRY_COUNT 4ull
#define UNI_ALL_STA_GI_MODE 5ull
#define UNI_ALL_STA_TXRX_MSDU_COUNT 6ull
#define UNI_ALL_STA_MAX_NUM 7ull
#define MT_NIC_CAP_TX_RESOURCE 0ull
#define MT_NIC_CAP_TX_EFUSE_ADDR 1ull
#define MT_NIC_CAP_COEX 2ull
#define MT_NIC_CAP_SINGLE_SKU 3ull
#define MT_NIC_CAP_CSUM_OFFLOAD 4ull
#define MT_NIC_CAP_HW_VER 5ull
#define MT_NIC_CAP_SW_VER 6ull
#define MT_NIC_CAP_MAC_ADDR 7ull
#define MT_NIC_CAP_PHY 8ull
#define MT_NIC_CAP_MAC 9ull
#define MT_NIC_CAP_FRAME_BUF 10ull
#define MT_NIC_CAP_BEAM_FORM 11ull
#define MT_NIC_CAP_LOCATION 12ull
#define MT_NIC_CAP_MUMIMO 13ull
#define MT_NIC_CAP_BUFFER_MODE_INFO 14ull
#define MT_NIC_CAP_HW_ADIE_VERSION 20ull
#define MT_NIC_CAP_ANTSWP 22ull
#define MT_NIC_CAP_WFDMA_REALLOC 23ull
#define MT_NIC_CAP_6G 24ull
#define MT_NIC_CAP_CHIP_CAP 32ull
#define MT_NIC_CAP_EML_CAP 34ull
#define UNI_SUSPEND_MODE_SETTING 0ull
#define UNI_SUSPEND_WOW_CTRL 1ull
#define UNI_SUSPEND_WOW_GPIO_PARAM 2ull
#define UNI_SUSPEND_WOW_WAKEUP_PORT 3ull
#define UNI_SUSPEND_WOW_PATTERN 4ull
#define WOW_USB 1ull
#define WOW_PCIE 2ull
#define WOW_GPIO 3ull
struct mt76_connac_bss_qos_tlv {
    __le16 tag;
    __le16 len;
    u8 qos;
    u8 pad[3];
} __attribute__((packed));

struct mt76_connac_beacon_loss_event {
    u8 bss_idx;
    u8 reason;
    u8 pad[2];
};

struct mt76_connac_rssi_notify_event {
    __le32 rssi[4];
} __attribute__((packed));

struct mt76_connac_mcu_bss_event {
    u8 bss_idx;
    u8 is_absent;
    u8 free_quota;
    u8 pad;
};

struct mt76_connac_mcu_scan_channel {
    u8 band;
    u8 channel_num;
};

struct mt76_connac_hw_scan_done {
    u8 seq_num;
    u8 sparse_channel_num;
    struct mt76_connac_mcu_scan_channel sparse_channel;
    u8 complete_channel_num;
    u8 current_state;
    u8 version;
    u8 pad;
    __le32 beacon_scan_num;
    u8 pno_enabled;
    u8 pad2[3];
    u8 sparse_channel_valid_num;
    u8 alpha2[3];
    u8 channel_num[64];
    __le16 channel_idle_time[64];
    u8 beacon_probe_num[64];
    u8 mdrdy_count[64];
    __le32 beacon_2g_num;
    __le32 beacon_5g_num;
    __le16 channel_scan_time[64];
} __attribute__((packed));

struct mt76_connac_sched_scan_done {
    u8 seq_num;
    u8 status;
    __le16 pad;
} __attribute__((packed));

struct bss_info_uni_bss_color {
    __le16 tag;
    __le16 len;
    u8 enable;
    u8 bss_color;
    u8 rsv[2];
} __attribute__((packed));

struct bss_info_uni_he {
    __le16 tag;
    __le16 len;
    __le16 he_rts_thres;
    u8 he_pe_duration;
    u8 su_disable;
    __le16 max_nss_mcs[3];
    u8 rsv[2];
} __attribute__((packed));

struct bss_info_uni_mbssid {
    __le16 tag;
    __le16 len;
    u8 max_indicator;
    u8 mbss_idx;
    u8 tx_bss_omac_idx;
    u8 rsv;
} __attribute__((packed));

struct mt76_connac_wow_pattern_tlv {
    __le16 tag;
    __le16 len;
    u8 index;
    u8 enable;
    u8 data_len;
    u8 pad;
    u8 mask[16];
    u8 pattern[128];
    u8 rsv[4];
} __attribute__((packed));

struct mt76_connac_wow_ctrl_tlv {
    __le16 tag;
    __le16 len;
    u8 cmd;
    u8 trigger;
    u8 wakeup_hif;
    u8 pad;
    u8 rsv[4];
} __attribute__((packed));

struct mt76_connac_wow_gpio_param_tlv {
    __le16 tag;
    __le16 len;
    u8 gpio_pin;
    u8 trigger_lvl;
    u8 pad[2];
    __le32 gpio_interval;
    u8 rsv[4];
} __attribute__((packed));

struct mt76_connac_arpns_tlv {
    __le16 tag;
    __le16 len;
    u8 mode;
    u8 ips_num;
    u8 option;
    u8 pad[1];
} __attribute__((packed));

struct mt76_connac_suspend_tlv {
    __le16 tag;
    __le16 len;
    u8 enable;
    u8 mdtim;
    u8 wow_suspend;
    u8 pad[5];
} __attribute__((packed));

#define MT76_STA_INFO_STATE_NONE 0ull
#define MT76_STA_INFO_STATE_AUTH 1ull
#define MT76_STA_INFO_STATE_ASSOC 2ull
struct mt76_connac_sku_tlv {
    u8 channel;
    s8 pwr_limit[161];
};

struct mt76_connac_tx_power_limit_tlv {
    u8 ver;
    u8 pad0;
    __le16 len;
    u8 n_chan;
    u8 band;
    u8 last_msg;
    u8 pad1;
    u8 alpha2[4];
    u8 pad2[32];
} __attribute__((packed));

struct mt76_connac_config {
    __le16 id;
    u8 type;
    u8 resp_type;
    __le16 data_size;
    __le16 resv;
    u8 data[320];
} __attribute__((packed));

struct mt76_connac_mcu_uni_event {
    u8 cid;
    u8 pad[3];
    __le32 status;
} __attribute__((packed));

struct mt76_connac_mcu_reg_event {
    __le32 reg;
    __le32 val;
} __attribute__((packed));

#endif /* MT79_MT76_CONNAC_MCU_H */
