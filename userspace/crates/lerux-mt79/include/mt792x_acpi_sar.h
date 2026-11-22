/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT792X_ACPI_SAR_H
#define MT79_MT792X_ACPI_SAR_H
#include "mt79_types.h"

#define MT792x_ASAR_MIN_DYN 1ull
#define MT792x_ASAR_MAX_DYN 8ull
#define MT792x_ASAR_MIN_GEO 3ull
#define MT792x_ASAR_MAX_GEO 8ull
#define MT792x_ASAR_MIN_FG 8ull
#define MT792x_ACPI_MTCL "MTCL"
#define MT792x_ACPI_MTDS "MTDS"
#define MT792x_ACPI_MTGS "MTGS"
#define MT792x_ACPI_MTFG "MTFG"
#define MT792X_ACPI_MTCL_INVALID 4294967295ull
struct mt792x_asar_dyn_limit {
    u8 idx;
    u8 frp[5];
};

struct mt792x_asar_dyn_limit_v2 {
    u8 idx;
    u8 frp[11];
};

struct mt792x_asar_geo_band {
    u8 pwr;
    u8 offset;
};

struct mt792x_asar_geo_limit {
    u8 idx;
    struct mt792x_asar_geo_band band[2];
};

struct mt792x_asar_geo_limit_v2 {
    u8 idx;
    struct mt792x_asar_geo_band band[3];
};

struct mt792x_asar_cl_v3 {
    u8 names[4];
    u8 version;
    u8 mode_6g;
    u8 cl6g[6];
    u8 mode_5g9;
    u8 cl5g9[6];
    u8 mode_be;
    u8 clbe[6];
};

struct mt792x_asar_cl {
    u8 names[4];
    u8 version;
    u8 mode_6g;
    u8 cl6g[6];
    u8 mode_5g9;
    u8 cl5g9[6];
};

struct mt792x_asar_fg {
    u8 names[4];
    u8 version;
    u8 rsvd;
    u8 nr_flag;
    u8 rsvd1;
};

#endif /* MT79_MT792X_ACPI_SAR_H */
