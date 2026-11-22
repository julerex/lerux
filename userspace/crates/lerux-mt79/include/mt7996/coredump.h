/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7996_COREDUMP_H
#define MT79_MT7996_COREDUMP_H
#include "../mt79_types.h"

struct mt7996_coredump_mem {
    u32 len;
} __attribute__((packed));

struct mt7996_mem_hdr {
    u32 start;
    u32 len;
};

#endif /* MT79_MT7996_COREDUMP_H */
