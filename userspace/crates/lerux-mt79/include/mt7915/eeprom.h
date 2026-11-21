/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7915_EEPROM_H
#define MT79_MT7915_EEPROM_H
#include "../mt79_types.h"

#define MT_EE_WIFI_CAL_GROUP 1ull
#define MT_EE_WIFI_CAL_DPD_2G 4ull
#define MT_EE_WIFI_CAL_DPD_5G 2ull
#define MT_EE_WIFI_CAL_DPD_6G 8ull
#define MT_EE_WIFI_CAL_DPD 14ull
#define MT_EE_CAL_UNIT 1024ull
#define MT_EE_CAL_GROUP_SIZE_7915 50192ull
#define MT_EE_CAL_GROUP_SIZE_7916 55312ull
#define MT_EE_CAL_GROUP_SIZE_7975 55312ull
#define MT_EE_CAL_GROUP_SIZE_7976 96272ull
#define MT_EE_CAL_GROUP_SIZE_7916_6G 96272ull
#define MT_EE_CAL_GROUP_SIZE_7981 147472ull
#define MT_EE_CAL_DPD_SIZE_V1 55296ull
#define MT_EE_CAL_DPD_SIZE_V2 307200ull
#define MT_EE_WIFI_CONF0_TX_PATH 7ull
#define MT_EE_WIFI_CONF0_RX_PATH 56ull
#define MT_EE_WIFI_CONF0_BAND_SEL 192ull
#define MT_EE_WIFI_CONF1_BAND_SEL 192ull
#define MT_EE_WIFI_CONF_STREAM_NUM 224ull
#define MT_EE_WIFI_CONF3_TX_PATH_B0 3ull
#define MT_EE_WIFI_CONF3_TX_PATH_B1 48ull
#define MT_EE_WIFI_CONF7_TSSI0_2G 1ull
#define MT_EE_WIFI_CONF7_TSSI0_5G 4ull
#define MT_EE_WIFI_CONF7_TSSI1_5G 16ull
#define MT_EE_RATE_DELTA_MASK 63ull
#define MT_EE_RATE_DELTA_SIGN 64ull
#define MT_EE_RATE_DELTA_EN 128ull
struct cal_data {
    u8 count;
    u8 pad_0;
    u16 offset[60];
};

#define MT_EE_CHIP_ID 0ull
#define MT_EE_VERSION 2ull
#define MT_EE_MAC_ADDR 4ull
#define MT_EE_MAC_ADDR2 10ull
#define MT_EE_DDIE_FT_VERSION 80ull
#define MT_EE_DO_PRE_CAL 98ull
#define MT_EE_WIFI_CONF 400ull
#define MT_EE_DO_PRE_CAL_V2 410ull
#define MT_EE_RATE_DELTA_2G 594ull
#define MT_EE_RATE_DELTA_5G 669ull
#define MT_EE_TX0_POWER_2G 764ull
#define MT_EE_TX0_POWER_5G 843ull
#define MT_EE_RATE_DELTA_2G_V2 2003ull
#define MT_EE_RATE_DELTA_5G_V2 2078ull
#define MT_EE_RATE_DELTA_6G_V2 2180ull
#define MT_EE_TX0_POWER_2G_V2 1089ull
#define MT_EE_TX0_POWER_5G_V2 1093ull
#define MT_EE_TX0_POWER_6G_V2 1125ull
#define MT_EE_ADIE_FT_VERSION 2464ull
#define __MT_EE_MAX 3584ull
#define __MT_EE_MAX_V2 4096ull
#define MT_EE_PRECAL 3600ull
#define MT_EE_PRECAL_V2 4112ull
#define MT7976_ONE_ADIE_DBDC 7ull
#define MT7975_ONE_ADIE 8ull
#define MT7976_ONE_ADIE 10ull
#define MT7975_DUAL_ADIE 13ull
#define MT7976_DUAL_ADIE 15ull
#define MT_EE_BAND_SEL_DEFAULT 0ull
#define MT_EE_BAND_SEL_5GHZ 1ull
#define MT_EE_BAND_SEL_2GHZ 2ull
#define MT_EE_BAND_SEL_DUAL 3ull
#define MT_EE_V2_BAND_SEL_2GHZ 0ull
#define MT_EE_V2_BAND_SEL_5GHZ 1ull
#define MT_EE_V2_BAND_SEL_6GHZ 2ull
#define MT_EE_V2_BAND_SEL_5GHZ_6GHZ 3ull
#define SKU_CCK 0ull
#define SKU_OFDM 1ull
#define SKU_HT_BW20 2ull
#define SKU_HT_BW40 3ull
#define SKU_VHT_BW20 4ull
#define SKU_VHT_BW40 5ull
#define SKU_VHT_BW80 6ull
#define SKU_VHT_BW160 7ull
#define SKU_HE_RU26 8ull
#define SKU_HE_RU52 9ull
#define SKU_HE_RU106 10ull
#define SKU_HE_RU242 11ull
#define SKU_HE_RU484 12ull
#define SKU_HE_RU996 13ull
#define SKU_HE_RU2x996 14ull
#define MAX_SKU_RATE_GROUP_NUM 15ull
#endif /* MT79_MT7915_EEPROM_H */
