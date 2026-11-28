/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT76_CONNAC_H
#define MT79_MT76_CONNAC_H
#include "mt79_types.h"

#define MT76_CONNAC_SCAN_IE_LEN 600ull
#define MT76_CONNAC_MAX_NUM_SCHED_SCAN_INTERVAL 10ull
#define MT76_CONNAC_MAX_SCHED_SCAN_SSID 10ull
#define MT76_CONNAC_MAX_SCAN_MATCH 16ull
#define MT76_CONNAC_MAX_WMM_SETS 4ull
#define MT76_CONNAC_COREDUMP_SZ 1331200ull
#define MT_TXD_SIZE 32ull
#define MT_USB_TXD_SIZE 64ull
#define MT_USB_HDR_SIZE 4ull
#define MT_USB_TAIL_SIZE 4ull
#define MT_SDIO_TXD_SIZE 64ull
#define MT_SDIO_TAIL_SIZE 8ull
#define MT_SDIO_HDR_SIZE 4ull
#define MT_MSDU_ID_VALID 32768ull
#define MT_TXD_LEN_LAST 32768ull
#define MT_TXD_LEN_MASK 4095ull
#define MT_TXD_LEN_MSDU_LAST 16384ull
#define MT_TXD_LEN_AMSDU_LAST 32768ull
#define PCIE_AER_UNC_STATUS_OFFSET 516ull
#define PCIE_AER_UNC_MASK_OFFSET 520ull
#define PCIE_AER_CO_STATUS_OFFSET 528ull
#define PKT_TYPE_TXS 0ull
#define PKT_TYPE_TXRXV 1ull
#define MT_TXP_MAX_BUF_NUM 6ull
#define PKT_TYPE_NORMAL 2ull
#define MT_HW_TXP_MAX_MSDU_NUM 4ull
#define MT_HW_TXP_MAX_BUF_NUM 4ull
#define PKT_TYPE_RX_DUP_RFB 3ull
#define PKT_TYPE_RX_TMR 4ull
#define PKT_TYPE_RETRIEVE 5ull
#define PKT_TYPE_TXRX_NOTIFY 6ull
#define PKT_TYPE_RX_EVENT 7ull
#define PKT_TYPE_NORMAL_MCU 8ull
#define PKT_TYPE_RX_FW_MONITOR 12ull
#define PKT_TYPE_TXRX_NOTIFY_V0 24ull
#define CMD_HE_MCS_BW80 0ull
#define CMD_HE_MCS_BW160 1ull
#define CMD_HE_MCS_BW8080 2ull
#define CMD_HE_MCS_BW_NUM 3ull
#define HW_BSSID_0 0ull
#define HW_BSSID_1 1ull
#define HW_BSSID_2 2ull
#define HW_BSSID_3 3ull
#define HW_BSSID_MAX 3ull
#define EXT_BSSID_START 16ull
#define EXT_BSSID_1 17ull
#define EXT_BSSID_15 31ull
#define EXT_BSSID_MAX 31ull
#define REPEATER_BSSID_START 32ull
#define REPEATER_BSSID_MAX 63ull
struct mt76_connac_reg_map {
    u32 phys;
    u32 maps;
    u32 size;
};

struct mt76_connac_sta_key_conf {
    s8 keyidx;
    u8 key[16];
};

struct mt76_connac_fw_txp {
    __le16 flags;
    __le16 token;
    u8 bss_idx;
    __le16 rept_wds_wcid;
    u8 nbuf;
    __le32 buf[6];
    __le16 len[6];
} __attribute__((packed, aligned(4)));

struct mt76_connac_txp_ptr {
    __le32 buf0;
    __le16 len0;
    __le16 len1;
    __le32 buf1;
};

struct mt76_connac_hw_txp {
    __le16 msdu_id[4];
    struct mt76_connac_txp_ptr ptr[2];
};

union mt76_connac_txp_common_anon_0 {
    struct mt76_connac_fw_txp fw;
    struct mt76_connac_hw_txp hw;
};

struct mt76_connac_txp_common {
    union mt76_connac_txp_common_anon_0 mt76_connac_txp_common_anon_0;
};

struct mt76_connac_tx_free {
    __le16 rx_byte_cnt;
    __le16 ctrl;
    __le32 txd;
};

#endif /* MT79_MT76_CONNAC_H */
