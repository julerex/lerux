/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7996_MCU_H
#define MT79_MT7996_MCU_H
#include "mt79_types.h"

struct mt7996_mcu_rxd {
    __le32 rxd[8];
    __le16 len;
    __le16 pkt_type_id;
    u8 eid;
    u8 seq;
    u8 option;
    u8 __rsv;
    u8 ext_eid;
    u8 __rsv1[2];
    u8 s2d_index;
};

struct mt7996_mcu_uni_event {
    __le16 cid;
    u8 __rsv[2];
    __le32 status;
} __attribute__((packed));

#define MT7996_PS_MULTI_WCID 2047ull
#define MT7996_PS_MULTI_PS_BIT 32768ull
#define MCU_PQ_ID(p, q) ((p << 15ull) | (q << 10ull))
#define MCU_PKT_ID 160ull
struct mt7996_mcu_thermal_ctrl_anon_2_type {
    u8 protect_type;
    u8 trigger_type;
};

struct mt7996_mcu_thermal_ctrl_anon_2_duty {
    u8 duty_level;
    u8 duty_cycle;
};

union mt7996_mcu_thermal_ctrl_anon_2 {
    struct mt7996_mcu_thermal_ctrl_anon_2_type type;
    struct mt7996_mcu_thermal_ctrl_anon_2_duty duty;
};

struct mt7996_mcu_thermal_ctrl {
    u8 ctrl_id;
    u8 band_idx;
    union mt7996_mcu_thermal_ctrl_anon_2 mt7996_mcu_thermal_ctrl_anon_2;
};

#define MAX_BEACON_NUM 32ull
struct mt7996_mcu_thermal_enable {
    __le32 trigger_temp;
    __le32 restore_temp;
    __le16 sustain_time;
    u8 rsv[2];
} __attribute__((packed));

struct mt7996_mcu_countdown_notify {
    u8 omac_idx;
    u8 count;
    u8 csa_failure_reason;
    u8 rsv;
};

#define MT7996_MAX_BSS_OFFLOAD_SIZE 2048ull
struct mt7996_mcu_rdd_report_long_pulse {
    __le32 start;
    __le16 pulse_width;
    __le16 pulse_power;
    u8 mdrdy_flag;
    u8 rsv[3];
};

struct mt7996_mcu_rdd_report_periodic_pulse {
    __le32 start;
    __le16 pulse_width;
    __le16 pulse_power;
    u8 mdrdy_flag;
    u8 rsv[3];
};

struct mt7996_mcu_rdd_report_hw_pulse {
    __le32 start;
    __le16 pulse_width;
    __le16 pulse_power;
    u8 sc_pass;
    u8 sw_reset;
    u8 mdrdy_flag;
    u8 tx_active;
};

struct mt7996_mcu_rdd_report {
    struct mt7996_mcu_rxd rxd;
    u8 __rsv1[4];
    __le16 tag;
    __le16 len;
    u8 rdd_idx;
    u8 long_detected;
    u8 constant_prf_detected;
    u8 staggered_prf_detected;
    u8 radar_type_idx;
    u8 periodic_pulse_num;
    u8 long_pulse_num;
    u8 hw_pulse_num;
    u8 out_lpn;
    u8 out_spn;
    u8 out_crpn;
    u8 out_crpw;
    u8 out_crbn;
    u8 out_stgpn;
    u8 out_stgpw;
    u8 __rsv2;
    __le32 out_pri_const;
    __le32 out_pri_stg[3];
    __le32 out_pri_stg_dmin;
    struct mt7996_mcu_rdd_report_long_pulse long_pulse[32];
    struct mt7996_mcu_rdd_report_periodic_pulse periodic_pulse[32];
    struct mt7996_mcu_rdd_report_hw_pulse hw_pulse[32];
} __attribute__((packed));

#define MT7996_PATCH_SEC 4278190080ull
#define MT7996_PATCH_SCRAMBLE_KEY 65280ull
#define MT7996_PATCH_AES_KEY 255ull
#define MT7996_SEC_ENCRYPT 1ull
#define MT7996_SEC_KEY_IDX 6ull
#define MT7996_SEC_IV 8ull
struct mt7996_mcu_background_chain_ctrl {
    u8 _rsv[4];
    __le16 tag;
    __le16 len;
    u8 chan;
    u8 central_chan;
    u8 bw;
    u8 tx_stream;
    u8 rx_stream;
    u8 monitor_chan;
    u8 monitor_central_chan;
    u8 monitor_bw;
    u8 monitor_tx_stream;
    u8 monitor_rx_stream;
    u8 scan_mode;
    u8 band_idx;
    u8 monitor_scan_type;
    u8 band;
    u8 rsv[2];
} __attribute__((packed));

struct mt7996_mcu_eeprom_update {
    u8 _rsv[4];
    __le16 tag;
    __le16 len;
    u8 buffer_mode;
    u8 format;
    __le16 buf_len;
} __attribute__((packed));

struct mt7996_mcu_eeprom_info {
    u8 _rsv[4];
    __le16 tag;
    __le16 len;
    __le32 addr;
    __le32 valid;
} __attribute__((packed));

struct mt7996_mcu_phy_rx_info {
    u8 category;
    u8 rate;
    u8 mode;
    u8 nsts;
    u8 gi;
    u8 coding;
    u8 stbc;
    u8 bw;
};

struct mt7996_mcu_mib {
    __le16 tag;
    __le16 len;
    __le32 offs;
    __le64 data;
} __attribute__((packed));

struct all_sta_trx_rate {
    __le16 wlan_idx;
    u8 __rsv1[2];
    u8 tx_mode;
    u8 flags;
    u8 tx_stbc;
    u8 tx_gi;
    u8 tx_bw;
    u8 tx_ldpc;
    u8 tx_mcs;
    u8 tx_nss;
    u8 rx_rate;
    u8 rx_mode;
    u8 rx_nsts;
    u8 rx_gi;
    u8 rx_coding;
    u8 rx_stbc;
    u8 rx_bw;
    u8 __rsv2;
} __attribute__((packed));

struct mt7996_mcu_wed_rro_event {
    struct mt7996_mcu_rxd rxd;
    u8 __rsv1[4];
    __le16 tag;
    __le16 len;
} __attribute__((packed));

struct mt7996_mcu_wed_rro_ba_event {
    __le16 tag;
    __le16 len;
    __le16 wlan_id;
    u8 tid;
    u8 __rsv1;
    __le32 status;
    __le16 id;
    u8 __rsv2[2];
} __attribute__((packed));

struct mt7996_mcu_wed_rro_ba_delete_event {
    __le16 tag;
    __le16 len;
    __le16 session_id;
    __le16 wlan_id;
    u8 tid;
    u8 __rsv1;
    u8 __rsv2[2];
} __attribute__((packed));

#define UNI_WED_RRO_BA_SESSION_STATUS 0ull
#define UNI_WED_RRO_BA_SESSION_TBL 1ull
#define UNI_WED_RRO_BA_SESSION_DELETE 2ull
struct mt7996_mcu_ps_sync_event {
    struct mt7996_mcu_rxd rxd;
    u8 bss_idx;
    u8 __rsv[3];
} __attribute__((packed));

struct mt7996_mcu_ps_client_info {
    __le16 tag;
    __le16 len;
    u8 ps_bit;
    u8 __rsv;
    __le16 wlan_idx;
    u8 buffer_size;
    u8 __rsv2[3];
} __attribute__((packed));

struct mt7996_mcu_ps_multi_client_info {
    __le16 tag;
    __le16 len;
    __le16 sta_cnt;
} __attribute__((packed));

#define UNI_PS_CLIENT_INFO 0ull
#define UNI_PS_MULTI_CLIENT_INFO 1ull
#define UNI_PS_MULTI_CLIENT_INFO_BITMAP 2ull
struct mt7996_mcu_thermal_notify {
    struct mt7996_mcu_rxd rxd;
    u8 __rsv1[4];
    __le16 tag;
    __le16 len;
    u8 event_id;
    u8 band_idx;
    u8 level_idx;
    u8 duty_percent;
    __le32 restore_temp;
    u8 __rsv2[4];
} __attribute__((packed));

#define UNI_MIB_OBSS_AIRTIME 26ull
#define UNI_MIB_NON_WIFI_TIME 27ull
#define UNI_MIB_TX_TIME 28ull
#define UNI_MIB_RX_TIME 29ull
struct edca {
    __le16 tag;
    __le16 len;
    u8 queue;
    u8 set;
    u8 cw_min;
    u8 cw_max;
    __le16 txop;
    u8 aifs;
    u8 __rsv;
};

#define MCU_FW_LOG_WM 0ull
#define MCU_FW_LOG_WA 1ull
#define MCU_FW_LOG_TO_HOST 2ull
#define MCU_FW_LOG_RELAY 16ull
#define MCU_TWT_AGRT_ADD 0ull
#define MCU_TWT_AGRT_MODIFY 1ull
#define MCU_TWT_AGRT_DELETE 2ull
#define MCU_TWT_AGRT_TEARDOWN 3ull
#define MCU_TWT_AGRT_GET_TSF 4ull
#define MCU_WA_PARAM_CMD_QUERY 0ull
#define MCU_WA_PARAM_CMD_SET 1ull
#define MCU_WA_PARAM_CMD_CAPABILITY 2ull
#define MCU_WA_PARAM_CMD_DEBUG 3ull
#define MCU_WA_PARAM_PDMA_RX 4ull
#define MCU_WA_PARAM_CPU_UTIL 11ull
#define MCU_WA_PARAM_RED 14ull
#define MCU_WA_PARAM_HW_PATH_HIF_VER 47ull
#define MCU_MMPS_STATIC 0ull
#define MCU_MMPS_DYNAMIC 1ull
#define MCU_MMPS_RSV 2ull
#define MCU_MMPS_DISABLE 3ull
struct bss_rate_tlv {
    __le16 tag;
    __le16 len;
    u8 __rsv1[4];
    __le16 bc_trans;
    __le16 mc_trans;
    u8 short_preamble;
    u8 bc_fixed_rate;
    u8 mc_fixed_rate;
    u8 __rsv2[9];
} __attribute__((packed));

#define BP_DISABLE 0ull
#define BP_SW_MODE 1ull
#define BP_HW_MODE 2ull
struct bss_ra_tlv {
    __le16 tag;
    __le16 len;
    u8 short_preamble;
    u8 force_sgi;
    u8 force_gf;
    u8 ht_mode;
    u8 se_off;
    u8 antenna_idx;
    __le16 max_phyrate;
    u8 force_tx_streams;
    u8 __rsv[3];
} __attribute__((packed));

struct bss_rlm_tlv {
    __le16 tag;
    __le16 len;
    u8 control_channel;
    u8 center_chan;
    u8 center_chan2;
    u8 bw;
    u8 tx_streams;
    u8 rx_streams;
    u8 ht_op_info;
    u8 sco;
    u8 band;
    u8 __rsv[3];
} __attribute__((packed));

struct bss_color_tlv {
    __le16 tag;
    __le16 len;
    u8 enable;
    u8 color;
    u8 rsv[2];
} __attribute__((packed));

struct bss_inband_discovery_tlv {
    __le16 tag;
    __le16 len;
    u8 tx_type;
    u8 tx_mode;
    u8 tx_interval;
    u8 enable;
    __le16 wcid;
    __le16 prob_rsp_len;
} __attribute__((packed));

struct bss_bcn_content_tlv {
    __le16 tag;
    __le16 len;
    __le16 tim_ie_pos;
    __le16 csa_ie_pos;
    __le16 bcc_ie_pos;
    u8 enable;
    u8 type;
    __le16 pkt_len;
} __attribute__((packed));

struct bss_bcn_cntdwn_tlv_anon_3_csa {
    bool static_pp;
    bool abort;
};

struct bss_bcn_cntdwn_tlv_anon_3_cca {
    bool abort;
};

union bss_bcn_cntdwn_tlv_anon_3 {
    struct bss_bcn_cntdwn_tlv_anon_3_csa csa;
    struct bss_bcn_cntdwn_tlv_anon_3_cca cca;
};

struct bss_bcn_cntdwn_tlv {
    __le16 tag;
    __le16 len;
    u8 cnt;
    union bss_bcn_cntdwn_tlv_anon_3 bss_bcn_cntdwn_tlv_anon_3;
    u8 rsv;
} __attribute__((packed));

struct bss_bcn_mbss_tlv {
    __le16 tag;
    __le16 len;
    __le32 bitmap;
    __le16 offset[32];
};

struct bss_txcmd_tlv {
    __le16 tag;
    __le16 len;
    u8 txcmd_mode;
    u8 __rsv[3];
} __attribute__((packed));

struct bss_sec_tlv {
    __le16 tag;
    __le16 len;
    u8 __rsv1[2];
    u8 cipher;
    u8 __rsv2[1];
} __attribute__((packed));

struct bss_ifs_time_tlv {
    __le16 tag;
    __le16 len;
    u8 slot_valid;
    u8 sifs_valid;
    u8 rifs_valid;
    u8 eifs_valid;
    __le16 slot_time;
    __le16 sifs_time;
    __le16 rifs_time;
    __le16 eifs_time;
    u8 eifs_cck_valid;
    u8 rsv;
    __le16 eifs_cck_time;
} __attribute__((packed));

struct bss_power_save {
    __le16 tag;
    __le16 len;
    u8 profile;
    u8 _rsv[3];
} __attribute__((packed));

struct bss_prot_tlv {
    __le16 tag;
    __le16 len;
    __le32 prot_mode;
} __attribute__((packed));

struct sta_rec_ht_uni {
    __le16 tag;
    __le16 len;
    __le16 ht_cap;
    __le16 ht_cap_ext;
    u8 ampdu_param;
    u8 _rsv[3];
} __attribute__((packed));

struct sta_rec_ba_uni {
    __le16 tag;
    __le16 len;
    u8 tid;
    u8 ba_type;
    u8 amsdu;
    u8 ba_en;
    __le16 ssn;
    __le16 winsize;
    u8 ba_rdd_rro;
    u8 __rsv[3];
} __attribute__((packed));

struct sta_rec_eht {
    __le16 tag;
    __le16 len;
    u8 tid_bitmap;
    u8 _rsv;
    __le16 mac_cap;
    __le64 phy_cap;
    __le64 phy_cap_ext;
    u8 mcs_map_bw20[4];
    u8 mcs_map_bw80[3];
    u8 mcs_map_bw160[3];
    u8 mcs_map_bw320[3];
    u8 _rsv2[3];
} __attribute__((packed));

struct sec_key_uni {
    __le16 wlan_idx;
    u8 mgmt_prot;
    u8 cipher_id;
    u8 cipher_len;
    u8 key_id;
    u8 key_len;
    u8 need_resp;
    u8 key[32];
    u8 pn[6];
    u8 bcn_mode;
    u8 _rsv;
} __attribute__((packed));

struct sta_rec_sec_uni {
    __le16 tag;
    __le16 len;
    u8 add;
    u8 n_cipher;
    u8 rsv[2];
    struct sec_key_uni key[2];
} __attribute__((packed));

struct sta_phy_uni {
    u8 type;
    u8 flag;
    u8 stbc;
    u8 sgi;
    u8 bw;
    u8 ldpc;
    u8 mcs;
    u8 nss;
    u8 he_ltf;
    u8 rsv[3];
};

struct sta_rec_ra_uni {
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
    struct sta_phy_uni phy;
    u8 rx_rcpi[4];
} __attribute__((packed));

struct sta_rec_ra_fixed_uni {
    __le16 tag;
    __le16 len;
    __le32 field;
    u8 op_mode;
    u8 op_vht_chan_width;
    u8 op_vht_rx_nss;
    u8 op_vht_rx_nss_type;
    struct sta_phy_uni phy;
    u8 spe_idx;
    u8 short_preamble;
    u8 is_5g;
    u8 mmps_mode;
} __attribute__((packed));

struct sta_rec_hdrt {
    __le16 tag;
    __le16 len;
    u8 hdrt_mode;
    u8 rsv[3];
} __attribute__((packed));

struct sta_rec_hdr_trans {
    __le16 tag;
    __le16 len;
    u8 from_ds;
    u8 to_ds;
    u8 dis_rx_hdr_tran;
    u8 mesh;
} __attribute__((packed));

struct sta_rec_ps_leave {
    __le16 tag;
    __le16 len;
    u8 __rsv[4];
} __attribute__((packed));

struct sta_rec_eht_mld {
    __le16 tag;
    __le16 len;
    u8 nsep;
    u8 __rsv1[2];
    u8 str_cap[3];
    __le16 eml_cap;
    u8 __rsv2[4];
} __attribute__((packed));

struct mld_setup_link {
    __le16 wcid;
    u8 bss_idx;
    u8 __rsv;
} __attribute__((packed));

struct mld_reconf_stop_link {
    __le16 tag;
    __le16 len;
    __le16 link_bitmap;
    u8 rsv[2];
    u8 bss_idx[16];
} __attribute__((packed));

#define UNI_CMD_MLD_RECONF_AP_REM_TIMER 3ull
#define UNI_CMD_MLD_RECONF_STOP_LINK 4ull
struct hdr_trans_en {
    __le16 tag;
    __le16 len;
    u8 enable;
    u8 check_bssid;
    u8 mode;
    u8 __rsv;
} __attribute__((packed));

struct hdr_trans_vlan {
    __le16 tag;
    __le16 len;
    u8 insert_vlan;
    u8 remove_vlan;
    u8 tid;
    u8 __rsv;
} __attribute__((packed));

struct hdr_trans_blacklist {
    __le16 tag;
    __le16 len;
    u8 idx;
    u8 enable;
    __le16 type;
} __attribute__((packed));

struct uni_header {
    u8 __rsv[4];
};

struct vow_rx_airtime {
    __le16 tag;
    __le16 len;
    u8 enable;
    u8 band;
    u8 __rsv[2];
} __attribute__((packed));

struct bf_sounding_on {
    __le16 tag;
    __le16 len;
    u8 snd_mode;
    u8 sta_num;
    u8 __rsv[2];
    __le16 wlan_id[4];
    __le32 snd_period;
} __attribute__((packed));

struct bf_hw_en_status_update {
    __le16 tag;
    __le16 len;
    bool ebf;
    bool ibf;
    u8 __rsv[2];
} __attribute__((packed));

struct bf_mod_en_ctrl {
    __le16 tag;
    __le16 len;
    u8 bf_num;
    u8 bf_bitmap;
    u8 bf_sel[8];
    u8 __rsv[2];
} __attribute__((packed));

union bf_tag_tlv {
    struct bf_sounding_on bf_snd;
    struct bf_hw_en_status_update bf_hw_en;
    struct bf_mod_en_ctrl bf_mod_en;
};

struct ra_rate {
    __le16 wlan_idx;
    u8 mode;
    u8 stbc;
    __le16 gi;
    u8 bw;
    u8 ldpc;
    u8 mcs;
    u8 nss;
    __le16 ltf;
    u8 spe;
    u8 preamble;
    u8 __rsv[2];
} __attribute__((packed));

struct ra_fixed_rate {
    __le16 tag;
    __le16 len;
    __le16 version;
    struct ra_rate rate;
} __attribute__((packed));

#define UNI_RA_FIXED_RATE 15ull
#define UNI_HDR_TRANS_EN 0ull
#define UNI_HDR_TRANS_VLAN 1ull
#define UNI_HDR_TRANS_BLACKLIST 2ull
#define RATE_PARAM_FIXED 3ull
#define RATE_PARAM_MMPS_UPDATE 5ull
#define RATE_PARAM_FIXED_HE_LTF 7ull
#define RATE_PARAM_FIXED_MCS 8ull
#define RATE_PARAM_FIXED_GI 11ull
#define RATE_PARAM_AUTO 20ull
#define BF_SOUNDING_ON 1ull
#define BF_HW_EN_UPDATE 17ull
#define BF_MOD_EN_CTRL 20ull
#define CMD_BAND_NONE 0ull
#define CMD_BAND_24G 1ull
#define CMD_BAND_5G 2ull
#define CMD_BAND_6G 3ull
struct bss_req_hdr {
    u8 bss_idx;
    u8 __rsv[3];
};

#define UNI_CHANNEL_SWITCH 0ull
#define UNI_CHANNEL_RX_PATH 1ull
#define UNI_CHIP_CONFIG_NIC_CAPA 3ull
#define UNI_CHIP_CONFIG_DUP_WTBL 4ull
#define UNI_BAND_CONFIG_RADIO_ENABLE 0ull
#define UNI_BAND_CONFIG_RTS_THRESHOLD 8ull
#define UNI_BAND_CONFIG_MAC_ENABLE_CTRL 12ull
#define UNI_BAND_CONFIG_BSSID_MAPPING_ADDR 18ull
#define UNI_WSYS_CONFIG_FW_LOG_CTRL 0ull
#define UNI_WSYS_CONFIG_FW_DBG_CTRL 1ull
#define UNI_RDD_CTRL_PARM 0ull
#define UNI_RDD_CTRL_SET_TH 3ull
#define UNI_EFUSE_ACCESS 1ull
#define UNI_EFUSE_BUFFER_MODE 2ull
#define UNI_EFUSE_FREE_BLOCK 3ull
#define UNI_EFUSE_BUFFER_RD 4ull
#define UNI_EXT_EEPROM_ACCESS 1ull
#define UNI_VOW_DRR_CTRL 0ull
#define UNI_VOW_RX_AT_AIRTIME_EN 11ull
#define UNI_VOW_RX_AT_AIRTIME_CLR_EN 14ull
#define UNI_CMD_MIB_DATA 0ull
#define UNI_POWER_OFF 0ull
#define UNI_CMD_TWT_ARGT_UPDATE 0ull
#define UNI_CMD_TWT_MGMT_OFFLOAD 1ull
#define UNI_RRO_DEL_ENTRY 1ull
#define UNI_RRO_SET_PLATFORM_TYPE 2ull
#define UNI_RRO_GET_BA_SESSION_TABLE 3ull
#define UNI_RRO_SET_BYPASS_MODE 4ull
#define UNI_RRO_SET_TXFREE_PATH 5ull
#define UNI_RRO_DEL_BA_SESSION 6ull
#define UNI_RRO_SET_FLUSH_TIMEOUT 7ull
#define UNI_CMD_SR_ENABLE 1ull
#define UNI_CMD_SR_ENABLE_SD 2ull
#define UNI_CMD_SR_ENABLE_MODE 3ull
#define UNI_CMD_SR_ENABLE_DPD 18ull
#define UNI_CMD_SR_ENABLE_TX 19ull
#define UNI_CMD_SR_SET_SRG_BITMAP 128ull
#define UNI_CMD_SR_SET_PARAM 193ull
#define UNI_CMD_SR_SET_SIGA 208ull
#define UNI_CMD_THERMAL_PROTECT_ENABLE 6ull
#define UNI_CMD_THERMAL_PROTECT_DISABLE 7ull
#define UNI_CMD_THERMAL_PROTECT_DUTY_CONFIG 8ull
#define UNI_TXPOWER_POWER_LIMIT_TABLE_CTRL 4ull
#define UNI_CMD_ACCESS_REG_BASIC 0ull
#define UNI_CMD_ACCESS_RF_REG_BASIC 1ull
#define UNI_CMD_SER_QUERY 0ull
#define UNI_CMD_SER_SET_RECOVER_L1 1ull
#define UNI_CMD_SER_SET_RECOVER_L2 2ull
#define UNI_CMD_SER_SET_RECOVER_L3_RX_ABORT 3ull
#define UNI_CMD_SER_SET_RECOVER_L3_TX_ABORT 4ull
#define UNI_CMD_SER_SET_RECOVER_L3_TX_DISABLE 5ull
#define UNI_CMD_SER_SET_RECOVER_L3_BF 6ull
#define UNI_CMD_SER_SET_RECOVER_L4_MDP 7ull
#define UNI_CMD_SER_SET_RECOVER_FROM_ETH 8ull
#define UNI_CMD_SER_SET_RECOVER_FULL 8ull
#define UNI_CMD_SER_SET_SYSTEM_ASSERT 9ull
#define UNI_CMD_SER_ENABLE 1ull
#define UNI_CMD_SER_SET 2ull
#define UNI_CMD_SER_TRIGGER 3ull
#define UNI_CMD_SDO_SET 1ull
#define UNI_CMD_SDO_QUERY 2ull
#define UNI_CMD_SDO_CP_MODE 6ull
#define MT7996_SEC_MODE_PLAIN 0ull
#define MT7996_SEC_MODE_AES 1ull
#define MT7996_SEC_MODE_SCRAMBLE 2ull
#define MT7996_SEC_MODE_MAX 3ull
struct fixed_rate_table_ctrl {
    u8 _rsv[4];
    __le16 tag;
    __le16 len;
    u8 table_idx;
    u8 antenna_idx;
    __le16 rate_idx;
    u8 spe_idx_sel;
    u8 spe_idx;
    u8 gi;
    u8 he_ltf;
    bool ldpc;
    bool txbf;
    bool dynamic_bw;
    u8 _rsv2;
} __attribute__((packed));

#endif /* MT79_MT7996_MCU_H */
