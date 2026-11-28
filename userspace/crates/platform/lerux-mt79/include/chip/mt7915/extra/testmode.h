/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7915_TESTMODE_H
#define MT79_MT7915_TESTMODE_H
#include "mt79_types.h"

struct mt7915_tm_trx {
    u8 type;
    u8 enable;
    u8 band;
    u8 rsv;
};

struct mt7915_tm_freq_offset {
    u8 band;
    u8 pad_0[3];
    __le32 freq_offset;
};

struct mt7915_tm_slot_time {
    u8 slot_time;
    u8 sifs;
    u8 rifs;
    u8 _rsv;
    __le16 eifs;
    u8 band;
    u8 _rsv1[5];
};

struct mt7915_tm_clean_txq {
    bool sta_pause;
    u8 wcid;
    u8 band;
    u8 rsv;
};

union mt7915_tm_cmd_param {
    __le32 data;
    struct mt7915_tm_trx trx;
    struct mt7915_tm_freq_offset freq;
    struct mt7915_tm_slot_time slot;
    struct mt7915_tm_clean_txq clean;
    u8 test[72];
};

struct mt7915_tm_cmd {
    u8 testmode_en;
    u8 param_idx;
    u8 _rsv[2];
    union mt7915_tm_cmd_param param;
} __attribute__((packed));

#define TM_MAC_TX 1ull
#define TM_MAC_RX 2ull
#define TM_MAC_TXRX 3ull
#define TM_MAC_TXRX_RXV 4ull
#define TM_MAC_RXV 5ull
#define TM_MAC_RX_RXV 6ull
struct tm_tx_cont {
    u8 control_ch;
    u8 center_ch;
    u8 bw;
    u8 tx_ant;
    __le16 rateval;
    u8 band;
    u8 txfd_mode;
};

union mt7915_tm_rf_test_op_rf_param {
    __le32 func_data;
    __le32 cal_dump;
    struct tm_tx_cont tx_cont;
    u8 _pad[80];
};

struct mt7915_tm_rf_test_op_rf {
    __le32 func_idx;
    union mt7915_tm_rf_test_op_rf_param param;
};

union mt7915_tm_rf_test_op {
    __le32 op_mode;
    __le32 freq;
    struct mt7915_tm_rf_test_op_rf rf;
};

struct mt7915_tm_rf_test {
    u8 action;
    u8 icap_len;
    u8 _rsv[2];
    union mt7915_tm_rf_test_op op;
} __attribute__((packed));

#define RF_OPER_NORMAL 0ull
#define RF_OPER_RF_TEST 1ull
#define RF_OPER_ICAP 2ull
#define RF_OPER_ICAP_OVERLAP 3ull
#define RF_OPER_WIFI_SPECTRUM 4ull
#define TAM_ARB_OP_MODE_NORMAL 1ull
#define TAM_ARB_OP_MODE_TEST 2ull
#define TAM_ARB_OP_MODE_FORCE_SU 5ull
#endif /* MT79_MT7915_TESTMODE_H */
