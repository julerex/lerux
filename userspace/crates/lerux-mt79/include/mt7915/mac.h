/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7915_MAC_H
#define MT79_MT7915_MAC_H
#include "../mt79_types.h"

#define MT_TX_FREE_VER 458752ull
#define MT_TX_FREE_MSDU_CNT_V0 127ull
#define MT_TX_FREE_COUNT 8191ull
#define MT_TX_FREE_COUNT_V3 251658240ull
#define MT_TX_FREE_STAT 24576ull
#define MT_TX_FREE_STAT_V3 805306368ull
#define MT_TX_FREE_MPDU_HEADER 32768ull
#define MT_TX_FREE_MPDU_HEADER_V3 1073741824ull
#define MT_TX_FREE_MSDU_ID_V3 32767ull
#define MT_TXS5_F0_FINAL_MPDU 2147483648ull
#define MT_TXS5_F0_QOS 1073741824ull
#define MT_TXS5_F0_TX_COUNT 1040187392ull
#define MT_TXS5_F0_FRONT_TIME 33554431ull
#define MT_TXS5_F1_MPDU_TX_COUNT 4278190080ull
#define MT_TXS5_F1_MPDU_TX_BYTES 16777215ull
#define MT_TXS6_F0_NOISE_3 4278190080ull
#define MT_TXS6_F0_NOISE_2 16711680ull
#define MT_TXS6_F0_NOISE_1 65280ull
#define MT_TXS6_F0_NOISE_0 255ull
#define MT_TXS6_F1_MPDU_FAIL_COUNT 4278190080ull
#define MT_TXS6_F1_MPDU_FAIL_BYTES 16777215ull
#define MT_TXS7_F0_RCPI_3 4278190080ull
#define MT_TXS7_F0_RCPI_2 16711680ull
#define MT_TXS7_F0_RCPI_1 65280ull
#define MT_TXS7_F0_RCPI_0 255ull
#define MT_TXS7_F1_MPDU_RETRY_COUNT 4278190080ull
#define MT_TXS7_F1_MPDU_RETRY_BYTES 16777215ull
struct mt7915_dfs_pulse {
    u32 max_width;
    s32 max_pwr;
    s32 min_pwr;
    u32 min_stgr_pri;
    u32 max_stgr_pri;
    u32 min_cr_pri;
    u32 max_cr_pri;
};

struct mt7915_dfs_pattern {
    u8 enb;
    u8 stgr;
    u8 min_crpn;
    u8 max_crpn;
    u8 min_crpr;
    u8 min_pw;
    u32 min_pri;
    u32 max_pri;
    u8 max_pw;
    u8 min_crbn;
    u8 max_crbn;
    u8 min_stgpn;
    u8 max_stgpn;
    u8 min_stgpr;
    u8 rsv[2];
    u32 min_stgpr_diff;
} __attribute__((packed));

struct mt7915_dfs_radar_spec {
    struct mt7915_dfs_pulse pulse_th;
    struct mt7915_dfs_pattern radar_pattern[16];
};

#endif /* MT79_MT7915_MAC_H */
