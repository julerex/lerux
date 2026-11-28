/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7921_MT7921_H
#define MT79_MT7921_MT7921_H
#include "mt79_types.h"

#define MT7921_MAX_AID 20ull
#define MT7921_TX_RING_SIZE 2048ull
#define MT7921_TX_MCU_RING_SIZE 256ull
#define MT7921_TX_FWDL_RING_SIZE 128ull
#define MT7921_RX_RING_SIZE 1536ull
#define MT7921_RX_MCU_RING_SIZE 8ull
#define MT7921_RX_MCU_WA_RING_SIZE 512ull
#define MT7902_RX_MCU_RING_SIZE 512ull
#define MT7921_EEPROM_SIZE 3584ull
#define MT7921_TOKEN_SIZE 8192ull
#define MT7921_EEPROM_BLOCK_SIZE 16ull
#define MT7921_SKU_RATE_NUM 161ull
#define MT7921_SKU_MAX_DELTA_IDX 161ull
#define MT7921_SKU_TABLE_SIZE 162ull
#define MCU_UNI_EVENT_ROC 39ull
#define MCU_UNI_EVENT_CLC 128ull
#define EXT_CMD_RADIO_LED_CTRL_ENABLE 1ull
#define EXT_CMD_RADIO_ON_LED 2ull
#define EXT_CMD_RADIO_OFF_LED 3ull
#define WF_RF_PIN_INIT 0ull
#define WF_RF_PIN_POLL 1ull
#define to_rcpi(rssi) ((2ull * rssi) + 220ull)
#define MT_EE_HW_TYPE_ENCAP 1ull
#define UNI_ROC_ACQUIRE 0ull
#define UNI_ROC_ABORT 1ull
#define UNI_ROC_NUM 2ull
#define MT7921_ROC_REQ_JOIN 0ull
#define MT7921_ROC_REQ_ROC 1ull
#define MT7921_ROC_REQ_NUM 2ull
#define UNI_EVENT_ROC_GRANT 0ull
#define UNI_EVENT_ROC_TAG_NUM 1ull
struct mt7921_realease_info {
    __le16 len;
    u8 pad_len;
    u8 tag;
} __attribute__((packed));

struct mt7921_fw_features {
    u8 segment;
    u8 data;
    u8 rsv[14];
};

struct mt7921_roc_grant_tlv {
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

#define MT7921_SDIO_TXD 0ull
#define MT7921_SDIO_DATA 1ull
#define MT7921_SDIO_CMD 2ull
#define MT7921_SDIO_FWDL 3ull
struct mt7921_sdio_intr_tx {
    u32 wtqcr[16];
};

struct mt7921_sdio_intr_rx {
    u16 num[2];
    u16 len0[16];
    u16 len1[128];
};

struct mt7921_sdio_intr {
    u32 isr;
    struct mt7921_sdio_intr_tx tx;
    struct mt7921_sdio_intr_rx rx;
    u32 rec_mb[2];
} __attribute__((packed));

#define MT7921_TXQ_BAND0 0ull
#define MT7921_TXQ_BAND1 1ull
#define MT7921_TXQ_FWDL 16ull
#define MT7921_TXQ_MCU_WM 17ull
#define MT7921_RXQ_BAND0 0ull
#define MT7921_RXQ_BAND1 1ull
#define MT7921_RXQ_MCU_WM 0ull
#define MT7902_TXQ_MCU_WM 15ull
struct mt7921_dma_layout {
    u8 mcu_wm_txq;
    u8 pad_0;
    u16 mcu_rxdone_ring_size;
    bool has_mcu_wa;
};

#define MT7921_CLC_POWER 0ull
#define MT7921_CLC_CHAN 1ull
#define MT7921_CLC_MAX_NUM 2ull
struct mt7921_clc_rule {
    u8 alpha2[2];
    u8 type[2];
    __le16 len;
} __attribute__((packed));

struct mt7921_clc {
    __le32 len;
    u8 idx;
    u8 ver;
    u8 nr_country;
    u8 type;
    u8 rsv[8];
} __attribute__((packed));

#define MT_EE_CHIP_ID 0ull
#define MT_EE_VERSION 2ull
#define MT_EE_MAC_ADDR 4ull
#define MT_EE_WIFI_CONF 124ull
#define MT_EE_HW_TYPE 1371ull
#define __MT_EE_MAX 2559ull
#define TXPWR_USER 0ull
#define TXPWR_EEPROM 1ull
#define TXPWR_MAC 2ull
#define TXPWR_MAX_NUM 3ull
struct mt7921_txpwr_data {
    u8 ch;
    u8 cck[4];
    u8 ofdm[8];
    u8 ht20[8];
    u8 ht40[9];
    u8 vht20[12];
    u8 vht40[12];
    u8 vht80[12];
    u8 vht160[12];
    u8 he26[12];
    u8 he52[12];
    u8 he106[12];
    u8 he242[12];
    u8 he484[12];
    u8 he996[12];
    u8 he996x2[12];
};

struct mt7921_txpwr {
    u8 ch;
    u8 rsv[3];
    struct mt7921_txpwr_data data[3];
};

#endif /* MT79_MT7921_MT7921_H */
