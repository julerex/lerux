/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7921_MCU_H
#define MT79_MT7921_MCU_H
#include "../mt79_types.h"

#define MT_RA_RATE_NSS 448ull
#define MT_RA_RATE_MCS 15ull
#define MT_RA_RATE_TX_MODE 7680ull
#define MT_RA_RATE_DCM_EN 16ull
#define MT_RA_RATE_BW 24576ull
struct mt7921_mcu_tx_done_event {
    u8 pid;
    u8 status;
    __le16 seq;
    u8 wlan_idx;
    u8 tx_cnt;
    __le16 tx_rate;
    u8 flag;
    u8 tid;
    u8 rsp_rate;
    u8 mcs;
    u8 bw;
    u8 tx_pwr;
    u8 reason;
    u8 rsv0[1];
    __le32 delay;
    __le32 timestamp;
    __le32 applied_flag;
    u8 txs[28];
    u8 rsv1[32];
} __attribute__((packed));

#define MCU_EXT_EVENT_RATE_REPORT 135ull
#define MT_EBF 1ull
#define MT_IBF 2ull
struct mt7921_mcu_ant_id_config {
    u8 ant_id[4];
};

struct mt7921_txpwr_req {
    u8 ver;
    u8 action;
    __le16 len;
    u8 dbdc_idx;
    u8 rsv[3];
} __attribute__((packed));

struct mt7921_wf_rf_pin_ctrl_event {
    u8 result;
    u8 value;
};

#define TM_SWITCH_MODE 0ull
#define TM_SET_AT_CMD 1ull
#define TM_QUERY_AT_CMD 2ull
#define MT7921_TM_NORMAL 0ull
#define MT7921_TM_TESTMODE 1ull
#define MT7921_TM_ICAP 2ull
#define MT7921_TM_ICAP_OVERLAP 3ull
#define MT7921_TM_WIFISPECTRUM 4ull
struct mt7921_rftest_cmd {
    u8 action;
    u8 rsv[3];
    __le32 param0;
    __le32 param1;
} __attribute__((packed));

struct mt7921_rftest_evt {
    __le32 param0;
    __le32 param1;
} __attribute__((packed));

struct mt7921_clc_info_tlv {
    __le16 tag;
    __le16 len;
    u8 chan_conf;
    u8 rsv[63];
} __attribute__((packed));

#endif /* MT79_MT7921_MCU_H */
