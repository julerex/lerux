/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7996_MAC_H
#define MT79_MT7996_MAC_H
#include "mt79_types.h"

struct mt7996_dfs_pulse {
    u32 max_width;
    s32 max_pwr;
    s32 min_pwr;
    u32 min_stgr_pri;
    u32 max_stgr_pri;
    u32 min_cr_pri;
    u32 max_cr_pri;
};

struct mt7996_dfs_pattern {
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

#endif /* MT79_MT7996_MAC_H */
