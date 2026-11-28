/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7996_MT7996_H
#define MT79_MT7996_MT7996_H
#include "mt79_types.h"

#define MT7996_MAX_RADIOS 3ull
#define MT7996_MAX_INTERFACES 19ull
#define MT7996_MAX_WMM_SETS 4ull
#define MT7996_TX_RING_SIZE 2048ull
#define MT7996_TX_MCU_RING_SIZE 256ull
#define MT7996_TX_FWDL_RING_SIZE 128ull
#define MT7996_RX_RING_SIZE 1536ull
#define MT7996_RX_MCU_RING_SIZE 512ull
#define MT7996_RX_MCU_RING_SIZE_WA 1024ull
#define MT7996_NPU_TX_RING_SIZE 1024ull
#define MT7996_NPU_RX_RING_SIZE 1024ull
#define MT7996_NPU_TXD_SIZE 3ull
#define MT7996_DEVICE_ID 31120ull
#define MT7996_DEVICE_ID_2 31121ull
#define MT7992_DEVICE_ID 31122ull
#define MT7992_DEVICE_ID_2 31130ull
#define MT7990_DEVICE_ID 31123ull
#define MT7990_DEVICE_ID_2 31131ull
#define MT7996_FIRMWARE_WA "mediatek/mt7996/mt7996_wa.bin"
#define MT7996_FIRMWARE_WM "mediatek/mt7996/mt7996_wm.bin"
#define MT7996_FIRMWARE_DSP "mediatek/mt7996/mt7996_dsp.bin"
#define MT7996_ROM_PATCH "mediatek/mt7996/mt7996_rom_patch.bin"
#define MT7996_FIRMWARE_WA_233 "mediatek/mt7996/mt7996_wa_233.bin"
#define MT7996_FIRMWARE_WM_233 "mediatek/mt7996/mt7996_wm_233.bin"
#define MT7996_ROM_PATCH_233 "mediatek/mt7996/mt7996_rom_patch_233.bin"
#define MT7992_FIRMWARE_WA "mediatek/mt7996/mt7992_wa.bin"
#define MT7992_FIRMWARE_WM "mediatek/mt7996/mt7992_wm.bin"
#define MT7992_FIRMWARE_DSP "mediatek/mt7996/mt7992_dsp.bin"
#define MT7992_ROM_PATCH "mediatek/mt7996/mt7992_rom_patch.bin"
#define MT7992_FIRMWARE_WA_23 "mediatek/mt7996/mt7992_wa_23.bin"
#define MT7992_FIRMWARE_WM_23 "mediatek/mt7996/mt7992_wm_23.bin"
#define MT7992_FIRMWARE_DSP_23 "mediatek/mt7996/mt7992_dsp_23.bin"
#define MT7992_ROM_PATCH_23 "mediatek/mt7996/mt7992_rom_patch_23.bin"
#define MT7992_FIRMWARE_WA_24 "mediatek/mt7996/mt7992_wa_24.bin"
#define MT7992_FIRMWARE_WM_24 "mediatek/mt7996/mt7992_wm_24.bin"
#define MT7992_FIRMWARE_DSP_24 "mediatek/mt7996/mt7992_dsp_24.bin"
#define MT7992_ROM_PATCH_24 "mediatek/mt7996/mt7992_rom_patch_24.bin"
#define MT7990_FIRMWARE_WA ""
#define MT7990_FIRMWARE_WM "mediatek/mt7996/mt7990_wm.bin"
#define MT7990_FIRMWARE_DSP ""
#define MT7990_ROM_PATCH "mediatek/mt7996/mt7990_rom_patch.bin"
#define MT7996_EEPROM_DEFAULT "mediatek/mt7996/mt7996_eeprom.bin"
#define MT7996_EEPROM_DEFAULT_INT "mediatek/mt7996/mt7996_eeprom_2i5i6i.bin"
#define MT7996_EEPROM_DEFAULT_233 "mediatek/mt7996/mt7996_eeprom_233.bin"
#define MT7996_EEPROM_DEFAULT_233_INT "mediatek/mt7996/mt7996_eeprom_233_2i5i6i.bin"
#define MT7992_EEPROM_DEFAULT "mediatek/mt7996/mt7992_eeprom.bin"
#define MT7992_EEPROM_DEFAULT_INT "mediatek/mt7996/mt7992_eeprom_2i5i.bin"
#define MT7992_EEPROM_DEFAULT_MIX "mediatek/mt7996/mt7992_eeprom_2i5e.bin"
#define MT7992_EEPROM_DEFAULT_23 "mediatek/mt7996/mt7992_eeprom_23.bin"
#define MT7992_EEPROM_DEFAULT_23_INT "mediatek/mt7996/mt7992_eeprom_23_2i5i.bin"
#define MT7992_EEPROM_DEFAULT_24 "mediatek/mt7996/mt7992_eeprom_24_2i5i.bin"
#define MT7990_EEPROM_DEFAULT "mediatek/mt7996/mt7990_eeprom.bin"
#define MT7990_EEPROM_DEFAULT_INT "mediatek/mt7996/mt7990_eeprom_2i5i.bin"
#define MT7996_EEPROM_SIZE 7680ull
#define MT7996_EEPROM_BLOCK_SIZE 16ull
#define MT7996_EXT_EEPROM_BLOCK_SIZE 1024ull
#define MT7996_TOKEN_SIZE 16384ull
#define MT7996_HW_TOKEN_SIZE 8192ull
#define MT7996_CFEND_RATE_DEFAULT 73ull
#define MT7996_CFEND_RATE_11B 3ull
#define MT7996_IBF_MAX_NC 2ull
#define MT7996_IBF_TIMEOUT 24ull
#define MT7996_IBF_TIMEOUT_LEGACY 72ull
#define MT7992_CFEND_RATE_DEFAULT 75ull
#define MT7992_IBF_TIMEOUT 255ull
#define MT7996_SKU_RATE_NUM 417ull
#define MT7996_SKU_PATH_NUM 494ull
#define MT7996_MAX_TWT_AGRT 16ull
#define MT7996_MAX_STA_TWT_AGRT 8ull
#define MT7996_MIN_TWT_DUR 64ull
#define MT7996_MAX_QUEUE 27ull
#define MT7996_BASIC_RATES_TBL 31ull
#define MT7996_BEACON_RATES_TBL 25ull
#define MT7996_THERMAL_THROTTLE_MAX 100ull
#define MT7996_CDEV_THROTTLE_MAX 99ull
#define MT7996_CRIT_TEMP_IDX 0ull
#define MT7996_MAX_TEMP_IDX 1ull
#define MT7996_CRIT_TEMP 110ull
#define MT7996_MAX_TEMP 120ull
#define MT7996_MAX_HIF_RXD_IN_PG 5ull
#define MT7996_RRO_MSDU_PG_HASH_SIZE 127ull
#define MT7996_RRO_MAX_SESSION 1024ull
#define MT7996_RRO_WINDOW_MAX_LEN 1024ull
#define MT7996_RRO_ADDR_ELEM_LEN 128ull
#define MT7996_RRO_BA_BITMAP_LEN 2ull
#define MT7996_RRO_BA_BITMAP_CR_SIZE 65536ull
#define MT7996_RRO_BA_BITMAP_SESSION_SIZE 8ull
#define MT7996_RRO_WINDOW_MAX_SIZE 8192ull
#define MT7996_RRO_MSDU_PG_CR_CNT 8ull
#define MT7996_RRO_MSDU_PG_SIZE_PER_CR 65536ull
#define MT7996_EFUSE_BASE_OFFS_ADIE0 1024ull
#define MT7996_EFUSE_BASE_OFFS_ADIE1 7680ull
#define MT7996_EFUSE_BASE_OFFS_ADIE2 4608ull
#define MT7992_EFUSE_BASE_OFFS_ADIE1 4608ull
#define WED_RRO_ADDR_SIGNATURE_MASK 4278190080ull
#define WED_RRO_ADDR_COUNT_MASK 32752ull
#define WED_RRO_ADDR_HEAD_HIGH_MASK 15ull
#define RRO_HIF_DATA1_LS_MASK 1073741824ull
#define RRO_HIF_DATA1_SDL_MASK 1073676288ull
#define RRO_HIF_DATA4_RX_TOKEN_ID_MASK 65535ull
#define MSDU_PAGE_INFO_OWNER_MASK 2147483648ull
#define MSDU_PAGE_INFO_PG_HIGH_MASK 15ull
#define MT7996_MAX_RRO_RRS_RING 4ull
#define MT7996_RAM_TYPE_WM 0ull
#define MT7996_RAM_TYPE_WA 1ull
#define MT7996_RAM_TYPE_DSP 2ull
#define MT7996_VAR_TYPE_444 0ull
#define MT7996_VAR_TYPE_233 1ull
#define MT7992_VAR_TYPE_44 0ull
#define MT7992_VAR_TYPE_23 1ull
#define MT7992_VAR_TYPE_24 2ull
#define MT7990_VAR_TYPE_23 0ull
#define MT7996_FEM_EXT 0ull
#define MT7996_FEM_INT 1ull
#define MT7996_FEM_MIX 2ull
#define EEPROM_MODE_DEFAULT_BIN 0ull
#define EEPROM_MODE_EFUSE 1ull
#define EEPROM_MODE_FLASH 2ull
#define EEPROM_MODE_EXT 3ull
#define MT7996_TXQ_FWDL 16ull
#define MT7996_TXQ_MCU_WM 17ull
#define MT7996_TXQ_BAND0 18ull
#define MT7996_TXQ_BAND1 19ull
#define MT7996_TXQ_MCU_WA 20ull
#define MT7996_TXQ_BAND2 21ull
#define MT7996_RXQ_MCU_WM 0ull
#define MT7996_RXQ_MCU_WA 1ull
#define MT7996_RXQ_MCU_WA_MAIN 2ull
#define MT7996_RXQ_MCU_WA_EXT 3ull
#define MT7996_RXQ_MCU_WA_TRI 3ull
#define MT7996_RXQ_BAND0 4ull
#define MT7996_RXQ_BAND1 5ull
#define MT7996_RXQ_BAND2 5ull
#define MT7996_RXQ_RRO_BAND0 8ull
#define MT7996_RXQ_RRO_BAND1 9ull
#define MT7996_RXQ_RRO_BAND2 6ull
#define MT7996_RXQ_MSDU_PG_BAND0 10ull
#define MT7996_RXQ_MSDU_PG_BAND1 11ull
#define MT7996_RXQ_MSDU_PG_BAND2 12ull
#define MT7996_RXQ_TXFREE0 9ull
#define MT7996_RXQ_TXFREE1 9ull
#define MT7996_RXQ_TXFREE2 7ull
#define MT7996_RXQ_RRO_IND 0ull
#define MT7996_RXQ_RRO_RXDMAD_C 0ull
#define MT7990_RXQ_TXFREE0 6ull
#define MT7990_RXQ_TXFREE1 7ull
struct mt7996_wed_rro_addr {
    __le32 head_low;
    __le32 data;
};

struct mt7996_rro_hif {
    __le32 data0;
    __le32 data1;
    __le32 data2;
    __le32 data3;
    __le32 data4;
    __le32 data5;
};

struct mt7996_msdu_page_info {
    struct mt7996_rro_hif rxd[5];
    __le32 pg_low;
    __le32 data;
};

struct mt7996_rro_queue_regs_emi_ring {
    __le16 idx;
    __le16 rsv;
};

struct mt7996_rro_queue_regs_emi {
    struct mt7996_rro_queue_regs_emi_ring ring[4];
};

#define WFDMA0 0ull
#define WFDMA1 1ull
#define WFDMA_EXT 2ull
#define __MT_WFDMA_MAX 3ull
#define MT_RDD_IDX_BAND2 0ull
#define MT_RDD_IDX_BAND1 1ull
#define MT_RDD_IDX_BACKGROUND 2ull
#define RDD_STOP 0ull
#define RDD_START 1ull
#define RDD_DET_MODE 2ull
#define RDD_RADAR_EMULATE 3ull
#define RDD_START_TXQ 20ull
#define RDD_CAC_START 50ull
#define RDD_CAC_END 51ull
#define RDD_NORMAL_START 52ull
#define RDD_DISABLE_DFS_CAL 53ull
#define RDD_PULSE_DBG 54ull
#define RDD_READ_PULSE 55ull
#define RDD_RESUME_BF 56ull
#define RDD_IRQ_OFF 57ull
#endif /* MT79_MT7996_MT7996_H */
