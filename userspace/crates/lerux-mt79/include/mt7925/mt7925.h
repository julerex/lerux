/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7925_MT7925_H
#define MT79_MT7925_MT7925_H
#include "../mt79_types.h"

#define MT7925_BEACON_RATES_TBL 25ull
#define MT7925_TX_RING_SIZE 2048ull
#define MT7925_TX_MCU_RING_SIZE 256ull
#define MT7925_TX_FWDL_RING_SIZE 128ull
#define MT7925_RX_RING_SIZE 1536ull
#define MT7925_RX_MCU_RING_SIZE 512ull
#define MT7928_RX_MCU_WA_RING_SIZE 512ull
#define MT7925_EEPROM_SIZE 3584ull
#define MT7925_TOKEN_SIZE 8192ull
#define MT7925_EEPROM_BLOCK_SIZE 16ull
#define MT7925_SKU_RATE_NUM 161ull
#define MT7925_SKU_MAX_DELTA_IDX 161ull
#define MT7925_SKU_TABLE_SIZE 162ull
#define MCU_UNI_EVENT_ROC 39ull
#define HIF_TRAFFIC_IDLE 2ull
#define UNI_EVENT_HIF_CTRL_BASIC 0ull
#define to_rcpi(rssi) ((2ull * rssi) + 220ull)
#define UNI_EVENT_HIF_CTRL_TAG_NUM 1ull
struct mt7925_mcu_hif_ctrl_basic_tlv {
    __le16 tag;
    __le16 len;
    u8 cid;
    u8 pad[3];
    u32 status;
    u8 hif_type;
    u8 hif_tx_traffic_status;
    u8 hif_rx_traffic_status;
    u8 hifsuspend;
    u8 rsv[4];
} __attribute__((packed));

#define MT_EE_HW_TYPE_ENCAP 3ull
#define UNI_ROC_ACQUIRE 0ull
#define UNI_ROC_ABORT 1ull
#define UNI_ROC_SUB_LINK 3ull
#define UNI_ROC_NUM 4ull
#define MT7925_ROC_REQ_JOIN 0ull
#define MT7925_ROC_REQ_ROC 1ull
#define MT7925_ROC_REQ_SUB_LINK 2ull
#define MT7925_ROC_REQ_MLSR_AG 10ull
#define MT7925_ROC_REQ_MLSR_AA 11ull
#define MT7925_ROC_REQ_NUM 12ull
#define UNI_EVENT_ROC_GRANT 0ull
#define UNI_EVENT_ROC_GRANT_SUB_LINK 4ull
#define UNI_EVENT_ROC_TAG_NUM 5ull
struct mt7925_roc_grant_tlv {
    __le16 tag;
    __le16 len;
    u8 bss_idx;
    u8 tokenid;
    u8 status;
    u8 primarychannel;
    u8 rfsco;
    u8 rfband;
    u8 channelwidth;
    u8 centerfreqseg1;
    u8 centerfreqseg2;
    u8 reqtype;
    u8 dbdcband;
    u8 rsv[1];
    __le32 max_interval;
} __attribute__((packed));

struct mt7925_beacon_loss_tlv {
    __le16 tag;
    __le16 len;
    u8 reason;
    u8 nr_btolink;
    u8 pad[2];
} __attribute__((packed));

struct mt7925_uni_beacon_loss_event_hdr {
    u8 bss_idx;
    u8 pad[3];
};

struct mt7925_uni_beacon_loss_event {
    struct mt7925_uni_beacon_loss_event_hdr hdr;
    struct mt7925_beacon_loss_tlv beacon_loss;
};

struct mt7925_uni_rssi_monitor_event {
    __le16 tag;
    __le16 len;
    __le32 rssi;
} __attribute__((packed));

#define MT7925_TXQ_BAND0 0ull
#define MT7925_TXQ_BAND1 1ull
#define MT7925_TXQ_MCU_WM 15ull
#define MT7925_TXQ_FWDL 16ull
#define MT7925_RXQ_BAND0 2ull
#define MT7925_RXQ_BAND1 3ull
#define MT7925_RXQ_MCU_WM 0ull
#define MT7925_RXQ_MCU_WM2 1ull
#define MT7927_TXQ_BAND0 0ull
#define MT7927_TXQ_BAND1 1ull
#define MT7927_TXQ_MCU_WM 15ull
#define MT7927_TXQ_FWDL 16ull
#define MT7927_RXQ_BAND0 4ull
#define MT7927_RXQ_MCU_WM 6ull
#define MT7927_RXQ_DATA2 7ull
#define MT7928_RXQ_BAND0 0ull
#define MT7928_RXQ_BAND1 2ull
#define MT7928_RXQ_MCU_WM 3ull
#define MT7928_RXQ_MCU_WM2 1ull
#define MODE_OPEN 0ull
#define MODE_SHARED 1ull
#define MODE_WPA 3ull
#define MODE_WPA_PSK 4ull
#define MODE_WPA_NONE 5ull
#define MODE_WPA2 6ull
#define MODE_WPA2_PSK 7ull
#define MODE_WPA3_SAE 11ull
#define MT7925_CLC_POWER 0ull
#define MT7925_CLC_CHAN 1ull
#define MT7925_CLC_MAX_NUM 2ull
struct mt7925_clc_rule_v2 {
    u32 flag;
    u8 alpha2[2];
    u8 rsv[10];
} __attribute__((packed));

struct mt7925_clc_rule {
    u8 alpha2[2];
    u8 type[2];
    u8 seg_idx;
    u8 flag;
    u8 rsv[2];
};

struct mt7925_clc_segment {
    u8 idx;
    u8 rsv1[3];
    u32 offset;
    u32 len;
    u8 rsv2[4];
} __attribute__((packed));

struct mt7925_clc_type0 {
    u8 nr_country;
    u8 type;
    u8 nr_seg;
    u8 rsv[7];
};

struct mt7925_clc_type2 {
    u8 type;
    u8 rsv[9];
};

union mt7925_clc_anon_3 {
    struct mt7925_clc_type0 t0;
    struct mt7925_clc_type2 t2;
};

struct mt7925_clc {
    __le32 len;
    u8 idx;
    u8 ver;
    union mt7925_clc_anon_3 mt7925_clc_anon_3;
} __attribute__((packed));

#define MT_EE_CHIP_ID 0ull
#define MT_EE_VERSION 2ull
#define MT_EE_MAC_ADDR 4ull
#define MT_EE_HW_TYPE 2673ull
#define __MT_EE_MAX 2559ull
#define TXPWR_USER 0ull
#define TXPWR_EEPROM 1ull
#define TXPWR_MAC 2ull
#define TXPWR_MAX_NUM 3ull
struct mt7925_txpwr {
    s8 cck[4][2];
    s8 ofdm[8][2];
    s8 ht20[8][2];
    s8 ht40[9][2];
    s8 vht20[12][2];
    s8 vht40[12][2];
    s8 vht80[12][2];
    s8 vht160[12][2];
    s8 he26[12][2];
    s8 he52[12][2];
    s8 he106[12][2];
    s8 he242[12][2];
    s8 he484[12][2];
    s8 he996[12][2];
    s8 he996x2[12][2];
    s8 eht26[16][2];
    s8 eht52[16][2];
    s8 eht106[16][2];
    s8 eht242[16][2];
    s8 eht484[16][2];
    s8 eht996[16][2];
    s8 eht996x2[16][2];
    s8 eht996x4[16][2];
    s8 eht26_52[16][2];
    s8 eht26_106[16][2];
    s8 eht484_242[16][2];
    s8 eht996_484[16][2];
    s8 eht996_484_242[16][2];
    s8 eht996x2_484[16][2];
    s8 eht996x3[16][2];
    s8 eht996x3_484[16][2];
};

#endif /* MT79_MT7925_MT7925_H */
