/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7915_MCU_H
#define MT79_MT7915_MCU_H
#include "../mt79_types.h"

#define MCU_ATE_SET_TRX 1ull
#define MCU_ATE_SET_FREQ_OFFSET 10ull
#define MCU_ATE_SET_SLOT_TIME 19ull
#define MCU_ATE_CLEAN_TXQUEUE 28ull
struct mt7915_mcu_thermal_ctrl_anon_2_type {
    u8 protect_type;
    u8 trigger_type;
};

struct mt7915_mcu_thermal_ctrl_anon_2_duty {
    u8 duty_level;
    u8 duty_cycle;
};

union mt7915_mcu_thermal_ctrl_anon_2 {
    struct mt7915_mcu_thermal_ctrl_anon_2_type type;
    struct mt7915_mcu_thermal_ctrl_anon_2_duty duty;
};

struct mt7915_mcu_thermal_ctrl {
    u8 ctrl_id;
    u8 band_idx;
    union mt7915_mcu_thermal_ctrl_anon_2 mt7915_mcu_thermal_ctrl_anon_2;
};

#define WMM_AIFS_SET 1ull
#define WMM_CW_MIN_SET 2ull
#define WMM_CW_MAX_SET 4ull
#define WMM_TXOP_SET 8ull
#define WMM_PARAM_SET 15ull
#define MAX_BEACON_NUM 32ull
#define RATE_CFG_MCS 15ull
#define RATE_CFG_NSS 240ull
#define RATE_CFG_GI 3840ull
#define RATE_CFG_BW 61440ull
#define RATE_CFG_STBC 983040ull
#define RATE_CFG_LDPC 15728640ull
#define RATE_CFG_PHY_TYPE 251658240ull
#define RATE_CFG_HE_LTF 4026531840ull
#define MT7915_MAX_BEACON_SIZE 1308ull
struct mt7915_mcu_background_chain_ctrl {
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
};

struct mt7915_mcu_sr_ctrl {
    u8 action;
    u8 argnum;
    u8 band_idx;
    u8 status;
    u8 drop_ta_idx;
    u8 sta_idx;
    u8 rsv[2];
    __le32 val;
} __attribute__((packed));

struct mt7915_mcu_eeprom {
    u8 buffer_mode;
    u8 format;
    __le16 len;
} __attribute__((packed));

struct mt7915_mcu_eeprom_info {
    __le32 addr;
    __le32 valid;
    u8 data[16];
} __attribute__((packed));

struct mt7915_mcu_phy_rx_info {
    u8 category;
    u8 rate;
    u8 mode;
    u8 nsts;
    u8 gi;
    u8 coding;
    u8 stbc;
    u8 bw;
};

struct mt7915_mcu_mib {
    __le32 band;
    __le32 offs;
    __le64 data;
} __attribute__((packed));

#define MIB_TX_TIME 81ull
#define MIB_RX_TIME 82ull
#define MIB_OBSS_AIRTIME 86ull
#define MIB_NON_WIFI_TIME 87ull
#define MIB_TXOP_INIT_COUNT 88ull
#define MIB_TX_TIME_V2 6ull
#define MIB_RX_TIME_V2 8ull
#define MIB_OBSS_AIRTIME_V2 490ull
#define MIB_NON_WIFI_TIME_V2 491ull
struct mt7915_mcu_txpower_sku {
    u8 format_id;
    u8 limit_type;
    u8 band_idx;
    s8 txpower_sku[161];
};

struct edca {
    u8 queue;
    u8 set;
    u8 aifs;
    u8 cw_min;
    __le16 cw_max;
    __le16 txop;
};

struct mt7915_mcu_muru_stats_dl {
    __le32 cck_cnt;
    __le32 ofdm_cnt;
    __le32 htmix_cnt;
    __le32 htgf_cnt;
    __le32 vht_su_cnt;
    __le32 vht_2mu_cnt;
    __le32 vht_3mu_cnt;
    __le32 vht_4mu_cnt;
    __le32 he_su_cnt;
    __le32 he_ext_su_cnt;
    __le32 he_2ru_cnt;
    __le32 he_2mu_cnt;
    __le32 he_3ru_cnt;
    __le32 he_3mu_cnt;
    __le32 he_4ru_cnt;
    __le32 he_4mu_cnt;
    __le32 he_5to8ru_cnt;
    __le32 he_9to16ru_cnt;
    __le32 he_gtr16ru_cnt;
};

struct mt7915_mcu_muru_stats_ul {
    __le32 hetrig_su_cnt;
    __le32 hetrig_2ru_cnt;
    __le32 hetrig_3ru_cnt;
    __le32 hetrig_4ru_cnt;
    __le32 hetrig_5to8ru_cnt;
    __le32 hetrig_9to16ru_cnt;
    __le32 hetrig_gtr16ru_cnt;
    __le32 hetrig_2mu_cnt;
    __le32 hetrig_3mu_cnt;
    __le32 hetrig_4mu_cnt;
};

struct mt7915_mcu_muru_stats {
    __le32 event_id;
    struct mt7915_mcu_muru_stats_dl dl;
    struct mt7915_mcu_muru_stats_ul ul;
};

#define MCU_FW_LOG_WM 0ull
#define MCU_FW_LOG_WA 1ull
#define MCU_FW_LOG_TO_HOST 2ull
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
#define MCU_WA_PARAM_RED_SETTING 64ull
#define MCU_MMPS_STATIC 0ull
#define MCU_MMPS_DYNAMIC 1ull
#define MCU_MMPS_RSV 2ull
#define MCU_MMPS_DISABLE 3ull
struct bss_info_bmc_rate {
    __le16 tag;
    __le16 len;
    __le16 bc_trans;
    __le16 mc_trans;
    u8 short_preamble;
    u8 rsv[7];
} __attribute__((packed));

struct bss_info_ra {
    __le16 tag;
    __le16 len;
    u8 op_mode;
    u8 adhoc_en;
    u8 short_preamble;
    u8 tx_streams;
    u8 rx_streams;
    u8 algo;
    u8 force_sgi;
    u8 force_gf;
    u8 ht_mode;
    u8 has_20_sta;
    u8 bss_width_trigger_events;
    u8 vht_nss_cap;
    u8 vht_bw_signal;
    u8 vht_force_sgi;
    u8 se_off;
    u8 antenna_idx;
    u8 train_up_rule;
    u8 rsv[3];
    __le16 train_up_high_thres;
    __le16 train_up_rule_rssi;
    __le16 low_traffic_thres;
    __le16 max_phyrate;
    __le32 phy_cap;
    __le32 interval;
    __le32 fast_interval;
} __attribute__((packed));

struct bss_info_hw_amsdu {
    __le16 tag;
    __le16 len;
    __le32 cmp_bitmap_0;
    __le32 cmp_bitmap_1;
    __le16 trig_thres;
    u8 enable;
    u8 rsv;
} __attribute__((packed));

struct bss_info_color {
    __le16 tag;
    __le16 len;
    u8 disable;
    u8 color;
    u8 rsv[2];
} __attribute__((packed));

struct bss_info_he {
    __le16 tag;
    __le16 len;
    u8 he_pe_duration;
    u8 vht_op_info_present;
    __le16 he_rts_thres;
    __le16 max_nss_mcs[3];
    u8 rsv[6];
} __attribute__((packed));

struct bss_info_bcn {
    __le16 tag;
    __le16 len;
    u8 ver;
    u8 enable;
    __le16 sub_ntlv;
} __attribute__((aligned(4)));

struct bss_info_bcn_cntdwn {
    __le16 tag;
    __le16 len;
    u8 cnt;
    u8 rsv[3];
} __attribute__((aligned(4)));

struct bss_info_bcn_mbss {
    __le16 tag;
    __le16 len;
    __le32 bitmap;
    __le16 offset[32];
    u8 rsv[8];
};

struct bss_info_bcn_cont {
    __le16 tag;
    __le16 len;
    __le16 tim_ofs;
    __le16 csa_ofs;
    __le16 bcc_ofs;
    __le16 pkt_len;
} __attribute__((aligned(4)));

struct bss_info_inband_discovery {
    __le16 tag;
    __le16 len;
    u8 tx_type;
    u8 tx_mode;
    u8 tx_interval;
    u8 enable;
    __le16 rsv;
    __le16 prob_rsp_len;
} __attribute__((aligned(4)));

struct bss_info_prot {
    __le16 tag;
    __le16 len;
    __le32 prot_type;
    __le32 prot_mode;
    __le32 rts_len_thres;
    __le16 he_rts_thres;
    u8 rts_pkt_thres;
    u8 rsv[5];
} __attribute__((packed));

#define BSS_INFO_BCN_CSA 0ull
#define BSS_INFO_BCN_BCC 1ull
#define BSS_INFO_BCN_MBSSID 2ull
#define BSS_INFO_BCN_CONTENT 3ull
#define BSS_INFO_BCN_DISCOV 4ull
#define BSS_INFO_BCN_MAX 5ull
#define RATE_PARAM_FIXED 3ull
#define RATE_PARAM_MMPS_UPDATE 5ull
#define RATE_PARAM_FIXED_HE_LTF 7ull
#define RATE_PARAM_FIXED_MCS 8ull
#define RATE_PARAM_FIXED_GI 11ull
#define RATE_PARAM_AUTO 20ull
#define RATE_PARAM_SPE_UPDATE 22ull
#define TX_POWER_LIMIT_ENABLE 0ull
#define TX_POWER_LIMIT_PATH_ENABLE 3ull
#define TX_POWER_LIMIT_TABLE 4ull
#define TX_POWER_LIMIT_INFO 7ull
#define TX_POWER_LIMIT_FRAME 17ull
#define TX_POWER_LIMIT_FRAME_MIN 18ull
#define TX_POWER_INFO_PATH 1ull
#define TX_POWER_INFO_RATE 2ull
#define SPR_ENABLE 1ull
#define SPR_ENABLE_SD 3ull
#define SPR_ENABLE_MODE 5ull
#define SPR_ENABLE_DPD 35ull
#define SPR_ENABLE_TX 37ull
#define SPR_SET_SRG_BITMAP 128ull
#define SPR_SET_PARAM 194ull
#define SPR_SET_SIGA 220ull
#define THERMAL_PROTECT_PARAMETER_CTRL 0ull
#define THERMAL_PROTECT_BASIC_INFO 1ull
#define THERMAL_PROTECT_ENABLE 2ull
#define THERMAL_PROTECT_DISABLE 3ull
#define THERMAL_PROTECT_DUTY_CONFIG 4ull
#define THERMAL_PROTECT_MECH_INFO 5ull
#define THERMAL_PROTECT_DUTY_INFO 6ull
#define THERMAL_PROTECT_STATE_ACT 7ull
#define MT_BF_SOUNDING_ON 1ull
#define MT_BF_TYPE_UPDATE 20ull
#define MT_BF_MODULE_UPDATE 25ull
#define MURU_SET_ARB_OP_MODE 14ull
#define MURU_SET_PLATFORM_TYPE 25ull
#define MURU_PLATFORM_TYPE_PERF_LEVEL_1 1ull
#define MURU_PLATFORM_TYPE_PERF_LEVEL_2 2ull
#define MURU_SET_TXC_TX_STATS_EN 150ull
#define MURU_GET_TXC_TX_STATS 151ull
#define SER_QUERY 0ull
#define SER_SET_RECOVER_L1 1ull
#define SER_SET_RECOVER_L2 2ull
#define SER_SET_RECOVER_L3_RX_ABORT 3ull
#define SER_SET_RECOVER_L3_TX_ABORT 4ull
#define SER_SET_RECOVER_L3_TX_DISABLE 5ull
#define SER_SET_RECOVER_L3_BF 6ull
#define SER_SET_RECOVER_FULL 7ull
#define SER_SET_SYSTEM_ASSERT 8ull
#define SER_ENABLE 2ull
#define SER_RECOVER 3ull
#endif /* MT79_MT7915_MCU_H */
