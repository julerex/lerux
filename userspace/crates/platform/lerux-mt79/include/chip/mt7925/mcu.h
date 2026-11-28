/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7925_MCU_H
#define MT79_MT7925_MCU_H
#include "mt79_types.h"
#include "mt76_connac_mcu.h"
#include "mt7925.h"

#define MT_RA_RATE_NSS 448ull
#define MT_RA_RATE_MCS 15ull
#define MT_RA_RATE_TX_MODE 7680ull
#define MT_RA_RATE_DCM_EN 16ull
#define MT_RA_RATE_BW 24576ull
#define MCU_EXT_EVENT_RATE_REPORT 135ull
struct mt7925_mcu_eeprom_info {
    __le32 addr;
    __le32 valid;
    u8 data[16];
} __attribute__((packed));

#define MT7925_RNR_SCAN_MAX_BSSIDS 10ull
struct mt7925_mcu_rxd {
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

struct mt7925_mcu_uni_event {
    u8 cid;
    u8 pad[3];
    __le32 status;
} __attribute__((packed));

#define MT_CONNAC3_SKU_POWER_LIMIT 449ull
#define MT_EBF 1ull
#define MT_IBF 2ull
struct mt7925_mcu_reg_event {
    __le32 reg;
    __le32 val;
} __attribute__((packed));

struct mt7925_mcu_ant_id_config {
    u8 ant_id[4];
};

struct mt7925_txpwr_req {
    u8 _rsv[4];
    __le16 tag;
    __le16 len;
    u8 format_id;
    u8 catg;
    u8 band_idx;
    u8 _rsv1;
} __attribute__((packed));

struct mt7925_txpwr_event {
    u8 rsv[4];
    __le16 tag;
    __le16 len;
    u8 catg;
    u8 band_idx;
    u8 ch_band;
    u8 format;
    struct mt7925_txpwr txpwr;
    s8 pwr_max;
    s8 pwr_min;
    u8 rsv1;
} __attribute__((packed));

#define TM_SWITCH_MODE 0ull
#define TM_SET_AT_CMD 1ull
#define TM_QUERY_AT_CMD 2ull
#define MT7925_TM_NORMAL 0ull
#define MT7925_TM_TESTMODE 1ull
#define MT7925_TM_ICAP 2ull
#define MT7925_TM_ICAP_OVERLAP 3ull
#define MT7925_TM_WIFISPECTRUM 4ull
struct mt7925_rftest_evt {
    __le32 param0;
    __le32 param1;
} __attribute__((packed));

#define UNI_CHANNEL_SWITCH 0ull
#define UNI_CHANNEL_RX_PATH 1ull
#define UNI_CHIP_CONFIG_CHIP_CFG 2ull
#define UNI_CHIP_CONFIG_NIC_CAPA 3ull
#define UNI_BAND_CONFIG_RADIO_ENABLE 0ull
#define UNI_BAND_CONFIG_RTS_THRESHOLD 8ull
#define UNI_BAND_CONFIG_SET_MAC80211_RX_FILTER 12ull
#define UNI_WSYS_CONFIG_FW_LOG_CTRL 0ull
#define UNI_WSYS_CONFIG_FW_DBG_CTRL 1ull
#define UNI_EFUSE_ACCESS 1ull
#define UNI_EFUSE_BUFFER_MODE 2ull
#define UNI_EFUSE_FREE_BLOCK 3ull
#define UNI_EFUSE_BUFFER_RD 4ull
#define UNI_CMD_ACCESS_REG_BASIC 0ull
#define UNI_CMD_ACCESS_RF_REG_BASIC 1ull
#define UNI_MBMC_SETTING 0ull
#define UNI_MBMC_NO_RESP_SETTING 1ull
#define UNI_EVENT_SCAN_DONE_BASIC 0ull
#define UNI_EVENT_SCAN_DONE_CHNLINFO 2ull
#define UNI_EVENT_SCAN_DONE_NLO 3ull
#define UNI_CMD_RSSI_MONITOR_SET 0ull
#define UNI_EVENT_RSSI_MONITOR_INFO 0ull
#define CONNAC3_CIPHER_NONE 0ull
#define CONNAC3_CIPHER_WEP40 1ull
#define CONNAC3_CIPHER_TKIP 2ull
#define CONNAC3_CIPHER_AES_CCMP 4ull
#define CONNAC3_CIPHER_WEP104 5ull
#define CONNAC3_CIPHER_BIP_CMAC_128 6ull
#define CONNAC3_CIPHER_WEP128 7ull
#define CONNAC3_CIPHER_WAPI 8ull
#define CONNAC3_CIPHER_CCMP_256 10ull
#define CONNAC3_CIPHER_GCMP 11ull
#define CONNAC3_CIPHER_GCMP_256 12ull
#define DMASHDL_GROUP_0 0ull
#define DMASHDL_GROUP_1 1ull
#define DMASHDL_GROUP_2 2ull
#define DMASHDL_GROUP_3 3ull
#define DMASHDL_GROUP_4 4ull
#define DMASHDL_GROUP_5 5ull
#define DMASHDL_GROUP_6 6ull
#define DMASHDL_GROUP_7 7ull
#define DMASHDL_GROUP_8 8ull
#define DMASHDL_GROUP_9 9ull
#define DMASHDL_GROUP_10 10ull
#define DMASHDL_GROUP_11 11ull
#define DMASHDL_GROUP_12 12ull
#define DMASHDL_GROUP_13 13ull
#define DMASHDL_GROUP_14 14ull
#define DMASHDL_GROUP_15 15ull
#define DMASHDL_GROUP_NUM 16ull
#define DMASHDL_LITE_GROUP_NUM 64ull
struct mt7925_mcu_scan_chinfo_event {
    u8 nr_chan;
    u8 alpha2[3];
};

#define UNI_SCAN_REQ 1ull
#define UNI_SCAN_CANCEL 2ull
#define UNI_SCAN_SCHED_REQ 3ull
#define UNI_SCAN_SCHED_ENABLE 4ull
#define UNI_SCAN_SSID 10ull
#define UNI_SCAN_BSSID 11ull
#define UNI_SCAN_CHANNEL 12ull
#define UNI_SCAN_IE 13ull
#define UNI_SCAN_MISC 14ull
#define UNI_SCAN_SSID_MATCH_SETS 15ull
#define UNI_SNIFFER_ENABLE 0ull
#define UNI_SNIFFER_CONFIG 1ull
struct scan_hdr_tlv {
    u8 seq_num;
    u8 bss_idx;
    u8 pad[2];
};

struct scan_req_tlv {
    __le16 tag;
    __le16 len;
    u8 scan_type;
    u8 probe_req_num;
    u8 scan_func;
    u8 src_mask;
    __le16 channel_min_dwell_time;
    __le16 channel_dwell_time;
    __le16 timeout_value;
    __le16 probe_delay_time;
    __le32 func_mask_ext;
} __attribute__((packed));

struct scan_chan_info_tlv {
    __le16 tag;
    __le16 len;
    u8 channel_type;
    u8 channels_num;
    u8 pad[2];
    struct mt76_connac_mcu_scan_channel channels[64];
} __attribute__((packed));

struct scan_ie_tlv {
    __le16 tag;
    __le16 len;
    __le16 ies_len;
    u8 band;
    u8 pad;
};

struct scan_sched_req {
    __le16 tag;
    __le16 len;
    u8 version;
    u8 stop_on_match;
    u8 intervals_num;
    u8 scan_func;
    __le16 intervals[10];
};

struct scan_sched_enable {
    __le16 tag;
    __le16 len;
    u8 active;
    u8 rsv[3];
};

struct mbmc_set_req {
    u8 pad[4];
};

struct mbmc_conf_tlv {
    __le16 tag;
    __le16 len;
    u8 mbmc_en;
    u8 band;
    u8 pad[2];
} __attribute__((packed));

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

struct bss_req_hdr {
    u8 bss_idx;
    u8 __rsv[3];
};

struct bss_rate_tlv {
    __le16 tag;
    __le16 len;
    u8 __rsv1[2];
    __le16 basic_rate;
    __le16 bc_trans;
    __le16 mc_trans;
    u8 short_preamble;
    u8 bc_fixed_rate;
    u8 mc_fixed_rate;
    u8 __rsv2;
} __attribute__((packed));

struct bss_eht_tlv {
    __le16 tag;
    __le16 len;
    u8 is_eht_op_present;
    u8 is_eth_dscb_present;
    u8 eht_ctrl;
    u8 eht_ccfs0;
    u8 eht_ccfs1;
    u8 pad1;
    __le16 eht_dis_sub_chan_bitmap;
    u8 pad2[4];
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

struct sta_rec_sec_uni {
    __le16 tag;
    __le16 len;
    u8 add;
    u8 tx_key;
    u8 key_type;
    u8 is_authenticator;
    u8 peer_addr[6];
    u8 bss_idx;
    u8 cipher_id;
    u8 key_id;
    u8 key_len;
    u8 wlan_idx;
    u8 mgmt_prot;
    u8 key[32];
    u8 key_rsc[16];
} __attribute__((packed));

struct sta_rec_hdr_trans {
    __le16 tag;
    __le16 len;
    u8 from_ds;
    u8 to_ds;
    u8 dis_rx_hdr_tran;
    u8 rsv;
} __attribute__((packed));

struct sta_rec_eht_mld {
    __le16 tag;
    __le16 len;
    u8 nsep;
    u8 mld_type;
    u8 __rsv1[1];
    u8 str_cap[3];
    u8 eml_cap[3];
    u8 __rsv2[3];
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
    u8 pad[3];
} __attribute__((packed));

struct mt7925_sku_tlv {
    u8 channel;
    s8 pwr_limit[449];
};

struct mt7925_tx_power_limit_tlv {
    u8 rsv[4];
    __le16 tag;
    __le16 len;
    u8 ver;
    u8 pad0;
    __le16 rsv1;
    u8 n_chan;
    u8 band;
    u8 last_msg;
    u8 limit_type;
    u8 alpha2[4];
    u8 pad2[32];
} __attribute__((packed));

struct mt7925_arpns_tlv {
    __le16 tag;
    __le16 len;
    u8 enable;
    u8 ips_num;
    u8 rsv[2];
} __attribute__((packed));

struct mt7925_wow_pattern_tlv {
    __le16 tag;
    __le16 len;
    u8 bss_idx;
    u8 index;
    u8 enable;
    u8 data_len;
    u8 offset;
    u8 mask[16];
    u8 pattern[128];
    u8 rsv[4];
};

struct roc_acquire_tlv {
    __le16 tag;
    __le16 len;
    u8 bss_idx;
    u8 tokenid;
    u8 control_channel;
    u8 sco;
    u8 band;
    u8 bw;
    u8 center_chan;
    u8 center_chan2;
    u8 bw_from_ap;
    u8 center_chan_from_ap;
    u8 center_chan2_from_ap;
    u8 reqtype;
    __le32 maxinterval;
    u8 dbdcband;
    u8 rsv[3];
} __attribute__((packed));

#define CMD_TEST_CTRL_ACT_SWITCH_MODE 0ull
#define CMD_TEST_CTRL_ACT_SET_AT 1ull
#define CMD_TEST_CTRL_ACT_GET_AT 2ull
#define CMD_TEST_CTRL_ACT_SET_AT_ENG 3ull
#define CMD_TEST_CTRL_ACT_GET_AT_ENG 4ull
#define CMD_TEST_CTRL_ACT_NUM 5ull
#define CMD_TEST_CTRL_ACT_SWITCH_MODE_NORMAL 0ull
#define CMD_TEST_CTRL_ACT_SWITCH_MODE_RF_TEST 1ull
#define CMD_TEST_CTRL_ACT_SWITCH_MODE_ICAP 2ull
#define CMD_TEST_CTRL_ACT_SWITCH_MODE_NUM 3ull
union testmode_data {
    __le32 op_mode;
    __le32 channel_freq;
    u8 rf_at_info[84];
};

union testmode_evt {
    __le32 op_mode;
    __le32 channel_freq;
    u8 rf_at_info[1024];
};

struct uni_cmd_testmode_ctrl {
    u16 tag;
    u16 length;
    u8 action;
    u8 reserved[3];
    union testmode_data data;
} __attribute__((packed));

struct mt7925_rftest_cmd {
    u8 padding[4];
    struct uni_cmd_testmode_ctrl ctrl;
};

#endif /* MT79_MT7925_MCU_H */
