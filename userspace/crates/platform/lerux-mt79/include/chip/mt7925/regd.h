/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7925_REGD_H
#define MT79_MT7925_REGD_H
#include "mt79_types.h"

struct mt7925_regd_rule_header {
    u8 alpha2[2];
    u8 dfs_region;
    u8 rsv[13];
};

struct mt7925_regd_rule {
    __le32 start_freq;
    __le32 end_freq;
    __le32 max_bw;
    __le32 eirp;
    __le32 flags;
    u8 rsv[12];
};

struct mt7925_regd_cc {
    u8 alpha2[2];
    u8 ver;
    u8 rsv;
    __le32 n_reg_rules;
    u8 sign_type;
    u8 rsv1[7];
};

struct mt7925_regd_rule_ev {
    __le16 tag;
    __le16 len;
    __le32 n_reg_rules;
    u8 dfs_region;
    u8 rsv[15];
};

struct mt7925_regd_query_req {
    u8 rsv[4];
    __le16 tag;
    __le16 len;
    u8 ver;
    u8 sign_type;
    u8 rsv1[2];
    __le32 size;
    u8 alpha2[2];
    u8 rsv2[2];
    __le32 n_reg_rules;
    u8 rsv3[64];
};

#endif /* MT79_MT7925_REGD_H */
