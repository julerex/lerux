/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_DMA_H
#define MT79_DMA_H
#include "mt79_types.h"

#define MT_RING_SIZE 16ull
#define MT_DMA_CTL_SD_LEN1 16383ull
#define MT_DMA_CTL_LAST_SEC1 16384ull
#define MT_DMA_CTL_M_DONE 32768ull
#define MT_DMA_CTL_SD_LEN0 1073676288ull
#define MT_DMA_CTL_LAST_SEC0 1073741824ull
#define MT_DMA_CTL_DMA_DONE 2147483648ull
#define MT_DMA_CTL_TO_HOST 256ull
#define MT_DMA_CTL_TO_HOST_A 4096ull
#define MT_DMA_CTL_DROP 16384ull
#define MT_DMA_CTL_TOKEN 4294901760ull
#define MT_DMA_CTL_SDP1_H 983040ull
#define MT_DMA_CTL_SDP0_H 15ull
#define MT_DMA_CTL_WO_DROP 256ull
#define MT_DMA_PPE_CPU_REASON 63488ull
#define MT_DMA_PPE_ENTRY 2147418112ull
#define MT_DMA_INFO_DMA_FRAG 512ull
#define MT_DMA_INFO_PPE_VLD 2147483648ull
#define MT_DMA_CTL_PN_CHK_FAIL 8192ull
#define MT_DMA_CTL_VER_MASK 128ull
#define MT_DMA_SDP0 65535ull
#define MT_DMA_TOKEN_ID 4294901760ull
#define MT_DMA_MAGIC_MASK 4026531840ull
#define MT_DMA_RRO_EN 8192ull
#define MT_DMA_MAGIC_CNT 16ull
#define MT_DMA_WED_IND_CMD_CNT 8ull
#define MT_DMA_WED_IND_REASON 61440ull
#define MT_DMA_HDR_LEN 4ull
#define MT_RX_INFO_LEN 4ull
#define MT_FCE_INFO_LEN 4ull
#define MT_RX_RXWI_LEN 32ull
#define RRO_RXDMAD_DATA1_LS_MASK 1073741824ull
#define RRO_RXDMAD_DATA1_SDL0_MASK 1073676288ull
#define RRO_RXDMAD_DATA2_RX_TOKEN_ID_MASK 4294901760ull
#define RRO_RXDMAD_DATA2_IND_REASON_MASK 61440ull
#define RRO_RXDMAD_DATA3_MAGIC_CNT_MASK 4026531840ull
struct mt76_desc {
    __le32 buf0;
    __le32 ctrl;
    __le32 buf1;
    __le32 info;
};

struct mt76_wed_rro_desc {
    __le32 buf0;
    __le32 buf1;
};

struct mt76_rro_rxdmad_c {
    __le32 data0;
    __le32 data1;
    __le32 data2;
    __le32 data3;
};

#define MT_QSEL_MGMT 0ull
#define MT_QSEL_HCCA 1ull
#define MT_QSEL_EDCA 2ull
#define MT_QSEL_EDCA_2 3ull
#define EVT_CMD_DONE 0ull
#define EVT_CMD_ERROR 1ull
#define EVT_CMD_RETRY 2ull
#define EVT_EVENT_PWR_RSP 3ull
#define EVT_EVENT_WOW_RSP 4ull
#define EVT_EVENT_CARRIER_DETECT_RSP 5ull
#define EVT_EVENT_DFS_DETECT_RSP 6ull
#define MT_DMA_WED_IND_REASON_NORMAL 0ull
#define MT_DMA_WED_IND_REASON_REPEAT 1ull
#define MT_DMA_WED_IND_REASON_OLDPKT 2ull
#endif /* MT79_DMA_H */
