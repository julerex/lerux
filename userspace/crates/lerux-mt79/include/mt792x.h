/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT792X_H
#define MT79_MT792X_H
#include "mt79_types.h"

#define MT792x_MAX_INTERFACES 4ull
#define MT792x_WTBL_SIZE 20ull
#define MT792x_WTBL_RESERVED 19ull
#define MT792x_WTBL_STA 15ull
#define MT792x_CFEND_RATE_DEFAULT 73ull
#define MT792x_CFEND_RATE_11B 3ull
#define MT792x_FW_TAG_FEATURE 4ull
#define MT792x_FW_CAP_NAN 32ull
#define MT792x_FW_CAP_CNM 128ull
#define MT792x_CHIP_CAP_CLC_EVT_EN 1ull
#define MT792x_CHIP_CAP_RSSI_NOTIFY_EVT_EN 2ull
#define MT792x_CHIP_CAP_WF_RF_PIN_CTRL_EVT_EN 8ull
#define MT792x_CHIP_CAP_11D_EN 16ull
#define MT792x_CHIP_CAP_REGD_EN 32ull
#define MT792x_CHIP_CAP_MLO_EN 256ull
#define MT792x_CHIP_CAP_MLO_EML_EN 512ull
#define MT792x_BASIC_RATES_TBL 14ull
#define MT792x_DRV_OWN_RETRY_COUNT 10ull
#define MT792x_MCU_INIT_RETRY_COUNT 10ull
#define MT792x_WFSYS_INIT_RETRY_COUNT 2ull
#define MT7902_FIRMWARE_WM "mediatek/WIFI_RAM_CODE_MT7902_1.bin"
#define MT7920_FIRMWARE_WM "mediatek/WIFI_RAM_CODE_MT7961_1a.bin"
#define MT7921_FIRMWARE_WM "mediatek/WIFI_RAM_CODE_MT7961_1.bin"
#define MT7922_FIRMWARE_WM "mediatek/WIFI_RAM_CODE_MT7922_1.bin"
#define MT7925_FIRMWARE_WM "mediatek/mt7925/WIFI_RAM_CODE_MT7925_1_1.bin"
#define MT7927_FIRMWARE_WM "mediatek/mt7927/WIFI_RAM_CODE_MT6639_2_1.bin"
#define MT7928_FIRMWARE_WM "mediatek/mt7928/WIFI_RAM_CODE_MT7935_1_1.bin"
#define MT7902_ROM_PATCH "mediatek/WIFI_MT7902_patch_mcu_1_1_hdr.bin"
#define MT7920_ROM_PATCH "mediatek/WIFI_MT7961_patch_mcu_1a_2_hdr.bin"
#define MT7921_ROM_PATCH "mediatek/WIFI_MT7961_patch_mcu_1_2_hdr.bin"
#define MT7922_ROM_PATCH "mediatek/WIFI_MT7922_patch_mcu_1_1_hdr.bin"
#define MT7925_ROM_PATCH "mediatek/mt7925/WIFI_MT7925_PATCH_MCU_1_1_hdr.bin"
#define MT7927_ROM_PATCH "mediatek/mt7927/WIFI_MT6639_PATCH_MCU_2_1_hdr.bin"
#define MT7928_ROM_PATCH "mediatek/mt7928/WIFI_MT7935_PATCH_MCU_1_1_hdr.bin"
#define MT7928_CB_ROM_PATCH "mediatek/mt7928/CBMCU_CODE_MT7935_1_1.bin"
#define MT7928_PHY_RAM "mediatek/mt7928/WIFI_MT7935_PHY_RAM_CODE_1_1.bin"
#define MT792x_SDIO_HDR_TX_BYTES 65535ull
#define MT792x_SDIO_HDR_PKT_TYPE 196608ull
struct mt792x_realease_info {
    __le16 len;
    u8 pad_len;
    u8 tag;
} __attribute__((packed));

struct mt792x_fw_features {
    u8 segment;
    u8 data;
    u8 rsv[14];
};

#define MT792x_CLC_POWER 0ull
#define MT792x_CLC_POWER_EXT 1ull
#define MT792x_CLC_BE_CTRL 2ull
#define MT792x_CLC_REGD 3ull
#define MT792x_CLC_MAX_NUM 4ull
#define MT_AP_UNSET 0ull
#define MT_AP_DEFAULT 1ull
#define MT_AP_LPI 2ull
#define MT_AP_SP 3ull
#define MT_AP_VLP 4ull
#define MT792x_MLO_LINK_DISASSOC 0ull
#define MT792x_MLO_LINK_ASSOC 1ull
#define MT792x_MLO_CHANGED_PS_PENDING 2ull
#define MT792x_MLO_CHANGED_PS 3ull
struct mt792x_irq_map_tx {
    u32 all_complete_mask;
    u32 mcu_complete_mask;
};

struct mt792x_irq_map_rx {
    u32 all_complete_mask;
    u32 data_complete_mask;
    u32 wm_complete_mask;
    u32 wm2_complete_mask;
};

struct mt792x_irq_map {
    u32 host_irq_enable;
    struct mt792x_irq_map_tx tx;
    struct mt792x_irq_map_rx rx;
};

struct mt792x_dma_ring {
    u8 qid;
    u8 pad_0;
    u16 n_desc;
    u32 ring_base;
};

struct mt792x_dma_layout {
    struct mt792x_dma_ring tx_data0;
    struct mt792x_dma_ring tx_mcu;
    struct mt792x_dma_ring tx_fwdl;
    struct mt792x_dma_ring tx_done;
    struct mt792x_dma_ring rx_data;
    struct mt792x_dma_ring rx_mcu;
};

struct mt792x_pcie_reg {
    u32 imask;
    u32 pm;
};

#endif /* MT79_MT792X_H */
