/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7996_EEPROM_H
#define MT79_MT7996_EEPROM_H
#include "../mt79_types.h"

#define MT_EE_WIFI_CONF0_TX_PATH 7ull
#define MT_EE_WIFI_CONF0_BAND_SEL 7ull
#define MT_EE_WIFI_CONF1_BAND_SEL 56ull
#define MT_EE_WIFI_CONF2_BAND_SEL 7ull
#define MT_EE_WIFI_CONF1_TX_PATH_BAND0 56ull
#define MT_EE_WIFI_CONF2_TX_PATH_BAND1 7ull
#define MT_EE_WIFI_CONF2_TX_PATH_BAND2 56ull
#define MT_EE_WIFI_CONF3_RX_PATH_BAND0 7ull
#define MT_EE_WIFI_CONF3_RX_PATH_BAND1 56ull
#define MT_EE_WIFI_CONF4_RX_PATH_BAND2 7ull
#define MT_EE_WIFI_CONF4_STREAM_NUM_BAND0 56ull
#define MT_EE_WIFI_CONF5_STREAM_NUM_BAND1 7ull
#define MT_EE_WIFI_CONF5_STREAM_NUM_BAND2 56ull
#define MT_EE_WIFI_PA_LNA_CONFIG 3ull
#define MT_EE_RATE_DELTA_MASK 63ull
#define MT_EE_RATE_DELTA_SIGN 64ull
#define MT_EE_RATE_DELTA_EN 128ull
#define MT_EE_CHIP_ID 0ull
#define MT_EE_VERSION 2ull
#define MT_EE_MAC_ADDR 4ull
#define MT_EE_MAC_ADDR2 10ull
#define MT_EE_WIFI_CONF 400ull
#define MT_EE_MAC_ADDR3 704ull
#define MT_EE_RATE_DELTA_2G 5120ull
#define MT_EE_RATE_DELTA_5G 5245ull
#define MT_EE_RATE_DELTA_6G 5450ull
#define MT_EE_TX0_POWER_2G 4864ull
#define MT_EE_TX0_POWER_5G 4865ull
#define MT_EE_TX0_POWER_6G 4880ull
#define __MT_EE_MAX 7679ull
#define MT_EE_BAND_SEL_DEFAULT 0ull
#define MT_EE_BAND_SEL_2GHZ 1ull
#define MT_EE_BAND_SEL_5GHZ 2ull
#define MT_EE_BAND_SEL_6GHZ 3ull
#endif /* MT79_MT7996_EEPROM_H */
