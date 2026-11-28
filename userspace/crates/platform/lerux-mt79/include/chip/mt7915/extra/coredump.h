/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT7915_COREDUMP_H
#define MT79_MT7915_COREDUMP_H
#include "mt79_types.h"

struct trace {
    u32 id;
    u32 timestamp;
};

struct mt7915_coredump_mem {
    u32 len;
} __attribute__((packed));

struct mt7915_mem_hdr {
    u32 start;
    u32 len;
};

#endif /* MT79_MT7915_COREDUMP_H */
