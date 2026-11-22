/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7925_NAN_H
#define MT79_MT7925_NAN_H
#include "../mt79_types.h"

#define NAN_MAX_SOCIAL_CHANNELS 3ull
#define NAN_ANCHOR_MASTER_RANK_NUM 8ull
#define NAN_5G_LOW_DISC_CHANNEL 44ull
#define NAN_5G_HIGH_DISC_CHANNEL 149ull
#define NAN_MAX_MASTER_PREFERENCE 255ull
#define NAN_DEFAULT_DW_INTERVAL 1ull
#define NAN_DEFAULT_DISC_BCN_INTERVAL 100ull
#define NAN_TOTAL_DW 16ull
#define NAN_SUPPORTED_2G_FAW_CH_NUM 4ull
#define NAN_SUPPORTED_5G_FAW_CH_NUM 4ull
#define NAN_TIMELINE_MGMT_SIZE 2ull
#define NAN_TIMELINE_MGMT_CHNL_LIST_NUM 4ull
#define NAN_NUM_AVAIL_DB 2ull
#define NAN_NDC_ATTRIBUTE_ID_LENGTH 6ull
#define NAN_MAX_CONN_CFG 8ull
#define NAN_MAX_NDP_CXT 4ull
#define NAN_AVAIL_ATTR_ID_OFFSET 0ull
#define NAN_AVAIL_ATTR_LEN_OFFSET 1ull
#define NAN_AVAIL_SEQ_ID_OFFSET 3ull
#define NAN_AVAIL_ATTR_CTRL_OFFSET 4ull
#define NAN_AVAIL_CTRL_MAPID 15ull
#define NAN_AVAIL_CTRL_COMMIT_CHANGED 16ull
#define NAN_AVAIL_CTRL_POTN_CHANGED 32ull
#define NAN_AVAIL_CTRL_PUBLIC_AVAIL_CHANGED 64ull
#define NAN_AVAIL_CTRL_NDC_CHANGED 128ull
#define NAN_AVAIL_CTRL_CHECK_FOR_CHANGED 240ull
#define UNII1_LOWER_BOUND 36ull
#define UNII1_UPPER_BOUND 50ull
#define UNII3_LOWER_BOUND 149ull
#define UNII3_UPPER_BOUND 165ull
#define NAN_CH_CTRL_OP_CLASS 65280ull
#define NAN_CH_CTRL_PRIMARY_CH 16711680ull
#define NAN_CRB_USE_DATA_PATH 1ull
#define NAN_CRB_AVAIL_6G_FORMAT 6ull
#define NAN_UNI_CMD_SET_MASTER_PREFERENCE 0ull
#define NAN_UNI_CMD_ENABLE_REQUEST 7ull
#define NAN_UNI_CMD_DISABLE_REQUEST 8ull
#define NAN_UNI_CMD_UPDATE_AVAILABILITY 9ull
#define NAN_UNI_CMD_UPDATE_CRB 10ull
#define NAN_UNI_CMD_MANAGE_PEER_SCH_RECORD 12ull
#define NAN_UNI_CMD_MAP_STA_RECORD 13ull
#define NAN_UNI_CMD_UPDATE_AVAILABILITY_CTRL 20ull
#define NAN_UNI_CMD_UPDATE_PEER_CAPABILITY 21ull
#define NAN_UNI_CMD_CHANGE_NMI_ADDRESS 24ull
#define NAN_UNI_CMD_SET_DW_INTERVAL 26ull
#define NAN_UNI_CMD_SET_SYNC_RSSI 39ull
#define NAN_UNI_CMD_SET_CLUSTER_ID 40ull
#define NAN_UNI_CMD_KEY_MANAGEMENT 53ull
#define NAN_UNI_EVENT_ID_DE_EVENT_IND 19ull
#define NAN_UNI_EVENT_REPORT_DW_END 60ull
#define NAN_EVENT_ID_DISC_MAC_ADDR 0ull
#define NAN_EVENT_ID_JOINED_CLUSTER 2ull
#define NAN_SUPPORTED_BAND_ID_2P4G 2ull
#define NAN_SUPPORTED_BAND_ID_5G 4ull
#define NAN_PROPRIETARY_BAND_ID_6G 6ull
#define NAN_SUPPORTED_BAND_ID_6G 7ull
#define NAN_SUPPORTED_BN_2G 0ull
#define NAN_SUPPORTED_BN_5G_LOW 1ull
#define NAN_SUPPORTED_BN_5G_HIGH 2ull
#define NAN_SUPPORTED_BN_6G 3ull
#define NAN_SUPPORTED_BN_NUM 4ull
struct mt7925_nan_social_ch_scan_params {
    u8 dwell_time[3];
    __le16 scan_period[3];
} __attribute__((packed));

struct mt7925_nan_conf_dw {
    u8 config_2dot4g_dw_band;
    __le32 dw_2dot4g_interval_val;
    u8 config_5g_dw_band;
    __le32 dw_5g_interval_val;
} __attribute__((packed));

struct mt7925_nan_common_hdr {
    u8 reserved[4];
};

struct mt7925_nan_master_preference_tlv {
    __le16 tag;
    __le16 len;
    u8 master_preference;
    u8 reserved[3];
} __attribute__((aligned(4)));

struct mt7925_nan_dw_interval_tlv {
    __le16 tag;
    __le16 len;
    u8 dw_interval;
    u8 vendor_ioctl;
    __le16 disc_bcn_interval;
} __attribute__((aligned(4)));

struct mt7925_nan_sync_rssi_tlv {
    __le16 tag;
    __le16 len;
    s8 rssi_close_2g;
    s8 rssi_middle_2g;
    s8 rssi_close_5g;
    s8 rssi_middle_5g;
} __attribute__((aligned(4)));

struct mt7925_nan_avail_ctrl_tlv {
    __le16 tag;
    __le16 len;
    __le16 avail_ctrl;
    u8 seq_id;
    u8 reserved[1];
} __attribute__((aligned(4)));

struct mt7925_nan_ch_timeline {
    u8 is_valid;
    u8 reserved[3];
    __le32 ch_info;
    __le32 num;
    __le32 avail_map[16];
};

struct mt7925_nan_avail_entry_tlv {
    __le16 tag;
    __le16 len;
    u8 map_id;
    u8 is_cond_avail;
    u8 timeline_idx;
    u8 is_multi_map;
    struct mt7925_nan_ch_timeline ch_list[4];
};

struct mt7925_nan_sched_update_peer_cap_tlv {
    __le16 tag;
    __le16 len;
    __le32 sch_idx;
    u8 supported_bands;
    __le16 max_chnl_switch_time;
    u8 peer_supported_bands;
} __attribute__((packed, aligned(4)));

union mt7925_nan_sched_timeline_anon_3 {
    __le32 avail_map[16];
    u8 avail_block[64];
};

struct mt7925_nan_sched_timeline {
    u8 map_id;
    u8 local_map_id;
    u8 reserved[2];
    union mt7925_nan_sched_timeline_anon_3 mt7925_nan_sched_timeline_anon_3;
};

struct mt7925_nan_sched_faw_ndc_timeline {
    __le32 avail_map[16];
};

struct mt7925_nan_sched_ndc_ctrl {
    u8 is_valid;
    u8 ndc_id[6];
    u8 ndc_idx;
    struct mt7925_nan_sched_timeline timeline[2];
};

struct mt7925_nan_sched_update_crb_tlv {
    __le16 tag;
    __le16 len;
    __le32 sch_idx;
    u8 flags;
    u8 is_use_ranging;
    u8 reserved[2];
    struct mt7925_nan_sched_timeline comm_ranging_timeline[2];
    struct mt7925_nan_sched_timeline comm_faw_timeline[2];
    struct mt7925_nan_sched_ndc_ctrl comm_ndc_ctrl;
    struct mt7925_nan_sched_faw_ndc_timeline faw_ndc_timeline[2];
};

#endif /* MT79_MT7925_NAN_H */
