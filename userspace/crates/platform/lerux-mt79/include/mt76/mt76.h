/* SPDX-License-Identifier: GPL-2.0-only */
/* Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76. */
#ifndef MT79_MT76_H
#define MT79_MT76_H
#include "mt79_types.h"

#define MT_MCU_RING_SIZE 32ull
#define MT_RX_BUF_SIZE 2048ull
#define MT_SKB_HEAD_LEN 256ull
#define MT_MAX_NON_AQL_PKT 16ull
#define MT_TXQ_FREE_THR 32ull
#define MT76_TOKEN_FREE_THR 64ull
#define MT_QFLAG_WED_RING 3ull
#define MT_QFLAG_WED_TYPE 28ull
#define MT_QFLAG_WED 32ull
#define MT_QFLAG_WED_RRO 64ull
#define MT_QFLAG_WED_RRO_EN 128ull
#define MT_QFLAG_EMI_EN 256ull
#define MT_QFLAG_NPU 512ull
#define MT76_RNR_SCAN_MAX_BSSIDS 16ull
struct mt76_reg_pair {
    u32 reg;
    u32 value;
};

#define MT76_BUS_MMIO 0ull
#define MT76_BUS_USB 1ull
#define MT76_BUS_SDIO 2ull
#define MT76_N_WCIDS 1088ull
#define MT76_BEACON_MON_MAX_MISS 7ull
#define MT_TX_HW_QUEUE_PHY 12ull
#define MT_WCID_TX_INFO_RATE 65535ull
#define MT_WCID_TX_INFO_NSS 196608ull
#define MT_WCID_TX_INFO_TXPWR_ADJ 66846720ull
#define MT_WCID_TX_INFO_SET 2147483648ull
#define MT76_WED_Q_TX 0ull
#define MT76_WED_Q_TXFREE 1ull
#define MT76_WED_Q_RX 2ull
#define MT76_WED_RRO_Q_DATA 3ull
#define RRO_IND_DATA0_IND_REASON_MASK 4026531840ull
#define RRO_IND_DATA0_START_SEQ_MASK 268369920ull
#define RRO_IND_DATA0_SEQ_ID_MASK 4095ull
#define RRO_IND_DATA1_MAGIC_CNT_MASK 3758096384ull
#define RRO_IND_DATA1_IND_COUNT_MASK 8191ull
#define MT76_WED_RRO_Q_MSDU_PG 4ull
#define MT_TX_CB_DMA_DONE 1ull
#define MT_TX_CB_TXS_DONE 2ull
#define MT_TX_CB_TXS_FAILED 4ull
#define MT76_WED_RRO_Q_IND 5ull
#define MT_PACKET_ID_MASK 127ull
#define MT_PACKET_ID_NO_ACK 0ull
#define MT_PACKET_ID_NO_SKB 1ull
#define MT_PACKET_ID_WED 2ull
#define MT_PACKET_ID_FIRST 3ull
#define MT_PACKET_ID_HAS_RATE 128ull
#define MT76_WED_RRO_Q_RXDMAD_C 6ull
#define MT_DRV_TXWI_NO_FREE 1ull
#define MT_DRV_TX_ALIGNED4_SKBS 2ull
#define MT_DRV_SW_RX_AIRTIME 4ull
#define MT_DRV_RX_DMA_HDR 8ull
#define MT_DRV_HW_MGMT_TXQ 16ull
#define MT_DRV_AMSDU_OFFLOAD 32ull
#define MT_DRV_IGNORE_TXS_FAILED 64ull
#define MT_DRV_HW_PS_BUFFERING 128ull
#define MT76_HWRRO_OFF 0ull
#define MT76_HWRRO_V3 1ull
#define MT76_HWRRO_V3_1 2ull
#define MT_VEND_TYPE_EEPROM 2147483648ull
#define MT_VEND_TYPE_CFG 1073741824ull
#define MT_VEND_TYPE_MASK 3221225472ull
#define MT_TX_SG_MAX_SIZE 8ull
#define MT_RX_SG_MAX_SIZE 4ull
#define MT_NUM_TX_ENTRIES 256ull
#define MT_NUM_RX_ENTRIES 128ull
#define MCU_RESP_URB_SIZE 1024ull
#define MT76S_XMIT_BUF_SZ 261632ull
#define MT76S_NUM_TX_ENTRIES 256ull
#define MT76S_NUM_RX_ENTRIES 512ull
#define MT_MCUQ_WM 0ull
#define MT_MCUQ_WA 1ull
#define MT_MCUQ_FWDL 2ull
#define __MT_MCUQ_MAX 3ull
#define MT_RXQ_MAIN 0ull
#define MT_RXQ_MCU 1ull
#define MT_RXQ_MCU_WA 2ull
#define MT_RXQ_BAND1 3ull
#define MT_RXQ_BAND1_WA 4ull
#define MT_RXQ_MAIN_WA 5ull
#define MT_RXQ_BAND2 6ull
#define MT_RXQ_BAND2_WA 7ull
#define MT_RXQ_RRO_BAND0 8ull
#define MT_RXQ_RRO_BAND1 9ull
#define MT_RXQ_RRO_BAND2 10ull
#define MT_RXQ_MSDU_PAGE_BAND0 11ull
#define MT_RXQ_MSDU_PAGE_BAND1 12ull
#define MT_RXQ_MSDU_PAGE_BAND2 13ull
#define MT_RXQ_TXFREE_BAND0 14ull
#define MT_RXQ_TXFREE_BAND1 15ull
#define MT_RXQ_TXFREE_BAND2 16ull
#define MT_RXQ_RRO_IND 17ull
#define MT_RXQ_RRO_RXDMAD_C 18ull
#define MT_RXQ_NPU0 19ull
#define MT_RXQ_NPU1 20ull
#define __MT_RXQ_MAX 21ull
#define MT_BAND0 0ull
#define MT_BAND1 1ull
#define MT_BAND2 2ull
#define __MT_MAX_BAND 3ull
#define MT_CIPHER_NONE 0ull
#define MT_CIPHER_WEP40 1ull
#define MT_CIPHER_TKIP 2ull
#define MT_CIPHER_TKIP_NO_MIC 3ull
#define MT_CIPHER_AES_CCMP 4ull
#define MT_CIPHER_WEP104 5ull
#define MT_CIPHER_BIP_CMAC_128 6ull
#define MT_CIPHER_WEP128 7ull
#define MT_CIPHER_WAPI 8ull
#define MT_CIPHER_CCMP_CCX 9ull
#define MT_CIPHER_CCMP_256 10ull
#define MT_CIPHER_GCMP 11ull
#define MT_CIPHER_GCMP_256 12ull
#define MT_DFS_STATE_UNKNOWN 0ull
#define MT_DFS_STATE_DISABLED 1ull
#define MT_DFS_STATE_CAC 2ull
#define MT_DFS_STATE_ACTIVE 3ull
struct mt76_queue_regs {
    u32 desc_base;
    u32 ring_size;
    u32 cpu_idx;
    u32 dma_idx;
};

#define MT_PHY_TYPE_CCK 0ull
#define MT_PHY_TYPE_OFDM 1ull
#define MT_PHY_TYPE_HT 2ull
#define MT_PHY_TYPE_HT_GF 3ull
#define MT_PHY_TYPE_VHT 4ull
#define MT_PHY_TYPE_HE_SU 8ull
#define MT_PHY_TYPE_HE_EXT_SU 9ull
#define MT_PHY_TYPE_HE_TB 10ull
#define MT_PHY_TYPE_HE_MU 11ull
#define MT_PHY_TYPE_EHT_SU 13ull
#define MT_PHY_TYPE_EHT_TRIG 14ull
#define MT_PHY_TYPE_EHT_MU 15ull
#define __MT_PHY_TYPE_MAX 16ull
struct mt76_sta_stats {
    u64 tx_mode[16];
    u64 tx_bw[5];
    u64 tx_nss[4];
    u64 tx_mcs[16];
    u64 tx_bytes;
    u32 tx_packets;
    u32 tx_retries;
    u32 tx_failed;
    u8 pad_0[4];
    u64 rx_bytes;
    u32 rx_packets;
    u32 rx_errors;
    u32 rx_drops;
};

#define MT_WCID_FLAG_CHECK_PS 0ull
#define MT_WCID_FLAG_PS 1ull
#define MT_WCID_FLAG_4ADDR 2ull
#define MT_WCID_FLAG_HDR_TRANS 3ull
#define MT_WCID_FLAG_TDLS_PEER 4ull
struct mt76_txq {
    u16 wcid;
    u16 agg_ssn;
    bool send_bar;
    bool aggr;
};

struct mt76_wed_rro_ind {
    __le32 data0;
    __le32 data1;
};

struct mt76_tx_cb {
    u64 jiffies;
    u16 wcid;
    u8 pktid;
    u8 flags;
};

#define MT76_STATE_INITIALIZED 0ull
#define MT76_STATE_REGISTERED 1ull
#define MT76_STATE_RUNNING 2ull
#define MT76_STATE_MCU_RUNNING 3ull
#define MT76_SCANNING 4ull
#define MT76_HW_SCANNING 5ull
#define MT76_HW_SCHED_SCANNING 6ull
#define MT76_RESTART 7ull
#define MT76_RESET 8ull
#define MT76_MCU_RESET 9ull
#define MT76_REMOVED 10ull
#define MT76_READING_STATS 11ull
#define MT76_STATE_POWER_OFF 12ull
#define MT76_STATE_SUSPEND 13ull
#define MT76_STATE_ROC 14ull
#define MT76_STATE_PM 15ull
#define MT76_STATE_WED_RESET 16ull
#define MT76_STA_EVENT_ASSOC 0ull
#define MT76_STA_EVENT_AUTHORIZE 1ull
#define MT76_STA_EVENT_DISASSOC 2ull
struct mt76_hw_cap {
    bool has_2ghz;
    bool has_5ghz;
    bool has_6ghz;
};

struct mt76_channel_state {
    u64 cc_active;
    u64 cc_busy;
    u64 cc_rx;
    u64 cc_bss_rx;
    u64 cc_tx;
    s8 noise;
};

#define MT_VEND_DEV_MODE 1ull
#define MT_VEND_WRITE 2ull
#define MT_VEND_POWER_ON 4ull
#define MT_VEND_MULTI_WRITE 6ull
#define MT_VEND_MULTI_READ 7ull
#define MT_VEND_READ_EEPROM 9ull
#define MT_VEND_WRITE_FCE 66ull
#define MT_VEND_WRITE_CFG 70ull
#define MT_VEND_READ_CFG 71ull
#define MT_VEND_READ_EXT 99ull
#define MT_VEND_WRITE_EXT 102ull
#define MT_VEND_FEATURE_SET 145ull
#define MT_EP_IN_PKT_RX 0ull
#define MT_EP_IN_CMD_RESP 1ull
#define __MT_EP_IN_MAX 2ull
#define MT_EP_OUT_INBAND_CMD 0ull
#define MT_EP_OUT_AC_BE 1ull
#define MT_EP_OUT_AC_BK 2ull
#define MT_EP_OUT_AC_VI 3ull
#define MT_EP_OUT_AC_VO 4ull
#define MT_EP_OUT_HCCA 5ull
#define __MT_EP_OUT_MAX 6ull
struct mt76_mib_stats {
    u32 ack_fail_cnt;
    u32 fcs_err_cnt;
    u32 rts_cnt;
    u32 rts_retries_cnt;
    u32 ba_miss_cnt;
    u32 tx_bf_cnt;
    u32 tx_mu_bf_cnt;
    u32 tx_mu_mpdu_cnt;
    u32 tx_mu_acked_mpdu_cnt;
    u32 tx_su_acked_mpdu_cnt;
    u32 tx_bf_ibf_ppdu_cnt;
    u32 tx_bf_ebf_ppdu_cnt;
    u32 tx_bf_rx_fb_all_cnt;
    u32 tx_bf_rx_fb_eht_cnt;
    u32 tx_bf_rx_fb_he_cnt;
    u32 tx_bf_rx_fb_vht_cnt;
    u32 tx_bf_rx_fb_ht_cnt;
    u32 tx_bf_rx_fb_bw;
    u32 tx_bf_rx_fb_nc_cnt;
    u32 tx_bf_rx_fb_nr_cnt;
    u32 tx_bf_fb_cpl_cnt;
    u32 tx_bf_fb_trig_cnt;
    u32 tx_ampdu_cnt;
    u32 tx_stop_q_empty_cnt;
    u32 tx_mpdu_attempts_cnt;
    u32 tx_mpdu_success_cnt;
    u32 tx_pkt_ebf_cnt;
    u32 tx_pkt_ibf_cnt;
    u32 tx_rwp_fail_cnt;
    u32 tx_rwp_need_cnt;
    u32 rx_fifo_full_cnt;
    u32 channel_idle_cnt;
    u32 primary_cca_busy_time;
    u32 secondary_cca_busy_time;
    u32 primary_energy_detect_time;
    u32 cck_mdrdy_time;
    u32 ofdm_mdrdy_time;
    u32 green_mdrdy_time;
    u32 rx_vector_mismatch_cnt;
    u32 rx_delimiter_fail_cnt;
    u32 rx_mrdy_cnt;
    u32 rx_len_mismatch_cnt;
    u32 rx_mpdu_cnt;
    u32 rx_ampdu_cnt;
    u32 rx_ampdu_bytes_cnt;
    u32 rx_ampdu_valid_subframe_cnt;
    u32 rx_ampdu_valid_subframe_bytes_cnt;
    u32 rx_pfdrop_cnt;
    u32 rx_vec_queue_overflow_drop_cnt;
    u32 rx_ba_cnt;
    u32 tx_amsdu[8];
    u32 tx_amsdu_cnt;
    u32 dl_cck_cnt;
    u32 dl_ofdm_cnt;
    u32 dl_htmix_cnt;
    u32 dl_htgf_cnt;
    u32 dl_vht_su_cnt;
    u32 dl_vht_2mu_cnt;
    u32 dl_vht_3mu_cnt;
    u32 dl_vht_4mu_cnt;
    u32 dl_he_su_cnt;
    u32 dl_he_ext_su_cnt;
    u32 dl_he_2ru_cnt;
    u32 dl_he_2mu_cnt;
    u32 dl_he_3ru_cnt;
    u32 dl_he_3mu_cnt;
    u32 dl_he_4ru_cnt;
    u32 dl_he_4mu_cnt;
    u32 dl_he_5to8ru_cnt;
    u32 dl_he_9to16ru_cnt;
    u32 dl_he_gtr16ru_cnt;
    u32 ul_hetrig_su_cnt;
    u32 ul_hetrig_2ru_cnt;
    u32 ul_hetrig_3ru_cnt;
    u32 ul_hetrig_4ru_cnt;
    u32 ul_hetrig_5to8ru_cnt;
    u32 ul_hetrig_9to16ru_cnt;
    u32 ul_hetrig_gtr16ru_cnt;
    u32 ul_hetrig_2mu_cnt;
    u32 ul_hetrig_3mu_cnt;
    u32 ul_hetrig_4mu_cnt;
};

struct mt76_power_limits_path {
    s8 cck[4];
    s8 ofdm[4];
    s8 ofdm_bf[4];
    s8 ru[7][10];
    s8 ru_bf[7][10];
};

struct mt76_power_limits {
    s8 cck[4];
    s8 ofdm[8];
    s8 mcs[4][10];
    s8 ru[7][12];
    s8 eht[16][16];
    struct mt76_power_limits_path path;
};

#endif /* MT79_MT76_H */
