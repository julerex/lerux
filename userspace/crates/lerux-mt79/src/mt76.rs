//! `mt76.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT_MCU_RING_SIZE: u64 = 32;
pub const MT_RX_BUF_SIZE: u64 = 0x800;
pub const MT_SKB_HEAD_LEN: u64 = 256;
pub const MT_MAX_NON_AQL_PKT: u64 = 16;
pub const MT_TXQ_FREE_THR: u64 = 32;
pub const MT76_TOKEN_FREE_THR: u64 = 64;
pub const MT_QFLAG_WED_RING: u64 = 3;
pub const MT_QFLAG_WED_TYPE: u64 = 28;
pub const MT_QFLAG_WED: u64 = 32;
pub const MT_QFLAG_WED_RRO: u64 = 64;
pub const MT_QFLAG_WED_RRO_EN: u64 = 128;
pub const MT_QFLAG_EMI_EN: u64 = 256;
pub const MT_QFLAG_NPU: u64 = 512;
pub const MT76_RNR_SCAN_MAX_BSSIDS: u64 = 16;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_reg_pair {
    pub reg: u32,
    pub value: u32,
}

pub const MT76_BUS_MMIO: u64 = 0;
pub const MT76_BUS_USB: u64 = 1;
pub const MT76_BUS_SDIO: u64 = 2;
pub const MT76_N_WCIDS: u64 = 0x440;
pub const MT76_BEACON_MON_MAX_MISS: u64 = 7;
pub const MT_TX_HW_QUEUE_PHY: u64 = 12;
pub const MT_WCID_TX_INFO_RATE: u64 = 0xffff;
pub const MT_WCID_TX_INFO_NSS: u64 = 0x3_0000;
pub const MT_WCID_TX_INFO_TXPWR_ADJ: u64 = 0x3fc_0000;
pub const MT_WCID_TX_INFO_SET: u64 = 0x8000_0000;
pub const MT76_WED_Q_TX: u64 = 0;
pub const MT76_WED_Q_TXFREE: u64 = 1;
pub const MT76_WED_Q_RX: u64 = 2;
pub const MT76_WED_RRO_Q_DATA: u64 = 3;
pub const RRO_IND_DATA0_IND_REASON_MASK: u64 = 0xf000_0000;
pub const RRO_IND_DATA0_START_SEQ_MASK: u64 = 0xfff_0000;
pub const RRO_IND_DATA0_SEQ_ID_MASK: u64 = 0xfff;
pub const RRO_IND_DATA1_MAGIC_CNT_MASK: u64 = 0xe000_0000;
pub const RRO_IND_DATA1_IND_COUNT_MASK: u64 = 0x1fff;
pub const MT76_WED_RRO_Q_MSDU_PG: u64 = 4;
pub const MT_TX_CB_DMA_DONE: u64 = 1;
pub const MT_TX_CB_TXS_DONE: u64 = 2;
pub const MT_TX_CB_TXS_FAILED: u64 = 4;
pub const MT76_WED_RRO_Q_IND: u64 = 5;
pub const MT_PACKET_ID_MASK: u64 = 127;
pub const MT_PACKET_ID_NO_ACK: u64 = 0;
pub const MT_PACKET_ID_NO_SKB: u64 = 1;
pub const MT_PACKET_ID_WED: u64 = 2;
pub const MT_PACKET_ID_FIRST: u64 = 3;
pub const MT_PACKET_ID_HAS_RATE: u64 = 128;
pub const MT76_WED_RRO_Q_RXDMAD_C: u64 = 6;
pub const MT_DRV_TXWI_NO_FREE: u64 = 1;
pub const MT_DRV_TX_ALIGNED4_SKBS: u64 = 2;
pub const MT_DRV_SW_RX_AIRTIME: u64 = 4;
pub const MT_DRV_RX_DMA_HDR: u64 = 8;
pub const MT_DRV_HW_MGMT_TXQ: u64 = 16;
pub const MT_DRV_AMSDU_OFFLOAD: u64 = 32;
pub const MT_DRV_IGNORE_TXS_FAILED: u64 = 64;
pub const MT_DRV_HW_PS_BUFFERING: u64 = 128;
pub const MT76_HWRRO_OFF: u64 = 0;
pub const MT76_HWRRO_V3: u64 = 1;
pub const MT76_HWRRO_V3_1: u64 = 2;
pub const MT_VEND_TYPE_EEPROM: u64 = 0x8000_0000;
pub const MT_VEND_TYPE_CFG: u64 = 0x4000_0000;
pub const MT_VEND_TYPE_MASK: u64 = 0xc000_0000;
pub const MT_TX_SG_MAX_SIZE: u64 = 8;
pub const MT_RX_SG_MAX_SIZE: u64 = 4;
pub const MT_NUM_TX_ENTRIES: u64 = 256;
pub const MT_NUM_RX_ENTRIES: u64 = 128;
pub const MCU_RESP_URB_SIZE: u64 = 0x400;
pub const MT76S_XMIT_BUF_SZ: u64 = 0x3_fe00;
pub const MT76S_NUM_TX_ENTRIES: u64 = 256;
pub const MT76S_NUM_RX_ENTRIES: u64 = 512;
pub const MT_MCUQ_WM: u64 = 0;
pub const MT_MCUQ_WA: u64 = 1;
pub const MT_MCUQ_FWDL: u64 = 2;
pub const __MT_MCUQ_MAX: u64 = 3;
pub const MT_RXQ_MAIN: u64 = 0;
pub const MT_RXQ_MCU: u64 = 1;
pub const MT_RXQ_MCU_WA: u64 = 2;
pub const MT_RXQ_BAND1: u64 = 3;
pub const MT_RXQ_BAND1_WA: u64 = 4;
pub const MT_RXQ_MAIN_WA: u64 = 5;
pub const MT_RXQ_BAND2: u64 = 6;
pub const MT_RXQ_BAND2_WA: u64 = 7;
pub const MT_RXQ_RRO_BAND0: u64 = 8;
pub const MT_RXQ_RRO_BAND1: u64 = 9;
pub const MT_RXQ_RRO_BAND2: u64 = 10;
pub const MT_RXQ_MSDU_PAGE_BAND0: u64 = 11;
pub const MT_RXQ_MSDU_PAGE_BAND1: u64 = 12;
pub const MT_RXQ_MSDU_PAGE_BAND2: u64 = 13;
pub const MT_RXQ_TXFREE_BAND0: u64 = 14;
pub const MT_RXQ_TXFREE_BAND1: u64 = 15;
pub const MT_RXQ_TXFREE_BAND2: u64 = 16;
pub const MT_RXQ_RRO_IND: u64 = 17;
pub const MT_RXQ_RRO_RXDMAD_C: u64 = 18;
pub const MT_RXQ_NPU0: u64 = 19;
pub const MT_RXQ_NPU1: u64 = 20;
pub const __MT_RXQ_MAX: u64 = 21;
pub const MT_BAND0: u64 = 0;
pub const MT_BAND1: u64 = 1;
pub const MT_BAND2: u64 = 2;
pub const __MT_MAX_BAND: u64 = 3;
pub const MT_CIPHER_NONE: u64 = 0;
pub const MT_CIPHER_WEP40: u64 = 1;
pub const MT_CIPHER_TKIP: u64 = 2;
pub const MT_CIPHER_TKIP_NO_MIC: u64 = 3;
pub const MT_CIPHER_AES_CCMP: u64 = 4;
pub const MT_CIPHER_WEP104: u64 = 5;
pub const MT_CIPHER_BIP_CMAC_128: u64 = 6;
pub const MT_CIPHER_WEP128: u64 = 7;
pub const MT_CIPHER_WAPI: u64 = 8;
pub const MT_CIPHER_CCMP_CCX: u64 = 9;
pub const MT_CIPHER_CCMP_256: u64 = 10;
pub const MT_CIPHER_GCMP: u64 = 11;
pub const MT_CIPHER_GCMP_256: u64 = 12;
pub const MT_DFS_STATE_UNKNOWN: u64 = 0;
pub const MT_DFS_STATE_DISABLED: u64 = 1;
pub const MT_DFS_STATE_CAC: u64 = 2;
pub const MT_DFS_STATE_ACTIVE: u64 = 3;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_queue_regs {
    pub desc_base: u32,
    pub ring_size: u32,
    pub cpu_idx: u32,
    pub dma_idx: u32,
}

pub const MT_PHY_TYPE_CCK: u64 = 0;
pub const MT_PHY_TYPE_OFDM: u64 = 1;
pub const MT_PHY_TYPE_HT: u64 = 2;
pub const MT_PHY_TYPE_HT_GF: u64 = 3;
pub const MT_PHY_TYPE_VHT: u64 = 4;
pub const MT_PHY_TYPE_HE_SU: u64 = 8;
pub const MT_PHY_TYPE_HE_EXT_SU: u64 = 9;
pub const MT_PHY_TYPE_HE_TB: u64 = 10;
pub const MT_PHY_TYPE_HE_MU: u64 = 11;
pub const MT_PHY_TYPE_EHT_SU: u64 = 13;
pub const MT_PHY_TYPE_EHT_TRIG: u64 = 14;
pub const MT_PHY_TYPE_EHT_MU: u64 = 15;
pub const __MT_PHY_TYPE_MAX: u64 = 16;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_sta_stats {
    pub tx_mode: [u64; 16],
    pub tx_bw: [u64; 5],
    pub tx_nss: [u64; 4],
    pub tx_mcs: [u64; 16],
    pub tx_bytes: u64,
    pub tx_packets: u32,
    pub tx_retries: u32,
    pub tx_failed: u32,
    pub pad_0: [u8; 4],
    pub rx_bytes: u64,
    pub rx_packets: u32,
    pub rx_errors: u32,
    pub rx_drops: u32,
}

pub const MT_WCID_FLAG_CHECK_PS: u64 = 0;
pub const MT_WCID_FLAG_PS: u64 = 1;
pub const MT_WCID_FLAG_4ADDR: u64 = 2;
pub const MT_WCID_FLAG_HDR_TRANS: u64 = 3;
pub const MT_WCID_FLAG_TDLS_PEER: u64 = 4;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_txq {
    pub wcid: u16,
    pub agg_ssn: u16,
    pub send_bar: u8,
    pub aggr: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_wed_rro_ind {
    pub data0: Le32,
    pub data1: Le32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_tx_cb {
    pub jiffies: u64,
    pub wcid: u16,
    pub pktid: u8,
    pub flags: u8,
}

pub const MT76_STATE_INITIALIZED: u64 = 0;
pub const MT76_STATE_REGISTERED: u64 = 1;
pub const MT76_STATE_RUNNING: u64 = 2;
pub const MT76_STATE_MCU_RUNNING: u64 = 3;
pub const MT76_SCANNING: u64 = 4;
pub const MT76_HW_SCANNING: u64 = 5;
pub const MT76_HW_SCHED_SCANNING: u64 = 6;
pub const MT76_RESTART: u64 = 7;
pub const MT76_RESET: u64 = 8;
pub const MT76_MCU_RESET: u64 = 9;
pub const MT76_REMOVED: u64 = 10;
pub const MT76_READING_STATS: u64 = 11;
pub const MT76_STATE_POWER_OFF: u64 = 12;
pub const MT76_STATE_SUSPEND: u64 = 13;
pub const MT76_STATE_ROC: u64 = 14;
pub const MT76_STATE_PM: u64 = 15;
pub const MT76_STATE_WED_RESET: u64 = 16;
pub const MT76_STA_EVENT_ASSOC: u64 = 0;
pub const MT76_STA_EVENT_AUTHORIZE: u64 = 1;
pub const MT76_STA_EVENT_DISASSOC: u64 = 2;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_hw_cap {
    pub has_2ghz: u8,
    pub has_5ghz: u8,
    pub has_6ghz: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_channel_state {
    pub cc_active: u64,
    pub cc_busy: u64,
    pub cc_rx: u64,
    pub cc_bss_rx: u64,
    pub cc_tx: u64,
    pub noise: i8,
}

pub const MT_VEND_DEV_MODE: u64 = 1;
pub const MT_VEND_WRITE: u64 = 2;
pub const MT_VEND_POWER_ON: u64 = 4;
pub const MT_VEND_MULTI_WRITE: u64 = 6;
pub const MT_VEND_MULTI_READ: u64 = 7;
pub const MT_VEND_READ_EEPROM: u64 = 9;
pub const MT_VEND_WRITE_FCE: u64 = 66;
pub const MT_VEND_WRITE_CFG: u64 = 70;
pub const MT_VEND_READ_CFG: u64 = 71;
pub const MT_VEND_READ_EXT: u64 = 99;
pub const MT_VEND_WRITE_EXT: u64 = 102;
pub const MT_VEND_FEATURE_SET: u64 = 145;
pub const MT_EP_IN_PKT_RX: u64 = 0;
pub const MT_EP_IN_CMD_RESP: u64 = 1;
pub const __MT_EP_IN_MAX: u64 = 2;
pub const MT_EP_OUT_INBAND_CMD: u64 = 0;
pub const MT_EP_OUT_AC_BE: u64 = 1;
pub const MT_EP_OUT_AC_BK: u64 = 2;
pub const MT_EP_OUT_AC_VI: u64 = 3;
pub const MT_EP_OUT_AC_VO: u64 = 4;
pub const MT_EP_OUT_HCCA: u64 = 5;
pub const __MT_EP_OUT_MAX: u64 = 6;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_mib_stats {
    pub ack_fail_cnt: u32,
    pub fcs_err_cnt: u32,
    pub rts_cnt: u32,
    pub rts_retries_cnt: u32,
    pub ba_miss_cnt: u32,
    pub tx_bf_cnt: u32,
    pub tx_mu_bf_cnt: u32,
    pub tx_mu_mpdu_cnt: u32,
    pub tx_mu_acked_mpdu_cnt: u32,
    pub tx_su_acked_mpdu_cnt: u32,
    pub tx_bf_ibf_ppdu_cnt: u32,
    pub tx_bf_ebf_ppdu_cnt: u32,
    pub tx_bf_rx_fb_all_cnt: u32,
    pub tx_bf_rx_fb_eht_cnt: u32,
    pub tx_bf_rx_fb_he_cnt: u32,
    pub tx_bf_rx_fb_vht_cnt: u32,
    pub tx_bf_rx_fb_ht_cnt: u32,
    pub tx_bf_rx_fb_bw: u32,
    pub tx_bf_rx_fb_nc_cnt: u32,
    pub tx_bf_rx_fb_nr_cnt: u32,
    pub tx_bf_fb_cpl_cnt: u32,
    pub tx_bf_fb_trig_cnt: u32,
    pub tx_ampdu_cnt: u32,
    pub tx_stop_q_empty_cnt: u32,
    pub tx_mpdu_attempts_cnt: u32,
    pub tx_mpdu_success_cnt: u32,
    pub tx_pkt_ebf_cnt: u32,
    pub tx_pkt_ibf_cnt: u32,
    pub tx_rwp_fail_cnt: u32,
    pub tx_rwp_need_cnt: u32,
    pub rx_fifo_full_cnt: u32,
    pub channel_idle_cnt: u32,
    pub primary_cca_busy_time: u32,
    pub secondary_cca_busy_time: u32,
    pub primary_energy_detect_time: u32,
    pub cck_mdrdy_time: u32,
    pub ofdm_mdrdy_time: u32,
    pub green_mdrdy_time: u32,
    pub rx_vector_mismatch_cnt: u32,
    pub rx_delimiter_fail_cnt: u32,
    pub rx_mrdy_cnt: u32,
    pub rx_len_mismatch_cnt: u32,
    pub rx_mpdu_cnt: u32,
    pub rx_ampdu_cnt: u32,
    pub rx_ampdu_bytes_cnt: u32,
    pub rx_ampdu_valid_subframe_cnt: u32,
    pub rx_ampdu_valid_subframe_bytes_cnt: u32,
    pub rx_pfdrop_cnt: u32,
    pub rx_vec_queue_overflow_drop_cnt: u32,
    pub rx_ba_cnt: u32,
    pub tx_amsdu: [u32; 8],
    pub tx_amsdu_cnt: u32,
    pub dl_cck_cnt: u32,
    pub dl_ofdm_cnt: u32,
    pub dl_htmix_cnt: u32,
    pub dl_htgf_cnt: u32,
    pub dl_vht_su_cnt: u32,
    pub dl_vht_2mu_cnt: u32,
    pub dl_vht_3mu_cnt: u32,
    pub dl_vht_4mu_cnt: u32,
    pub dl_he_su_cnt: u32,
    pub dl_he_ext_su_cnt: u32,
    pub dl_he_2ru_cnt: u32,
    pub dl_he_2mu_cnt: u32,
    pub dl_he_3ru_cnt: u32,
    pub dl_he_3mu_cnt: u32,
    pub dl_he_4ru_cnt: u32,
    pub dl_he_4mu_cnt: u32,
    pub dl_he_5to8ru_cnt: u32,
    pub dl_he_9to16ru_cnt: u32,
    pub dl_he_gtr16ru_cnt: u32,
    pub ul_hetrig_su_cnt: u32,
    pub ul_hetrig_2ru_cnt: u32,
    pub ul_hetrig_3ru_cnt: u32,
    pub ul_hetrig_4ru_cnt: u32,
    pub ul_hetrig_5to8ru_cnt: u32,
    pub ul_hetrig_9to16ru_cnt: u32,
    pub ul_hetrig_gtr16ru_cnt: u32,
    pub ul_hetrig_2mu_cnt: u32,
    pub ul_hetrig_3mu_cnt: u32,
    pub ul_hetrig_4mu_cnt: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_power_limits_path {
    pub cck: [i8; 4],
    pub ofdm: [i8; 4],
    pub ofdm_bf: [i8; 4],
    pub ru: [[i8; 10]; 7],
    pub ru_bf: [[i8; 10]; 7],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct mt76_power_limits {
    pub cck: [i8; 4],
    pub ofdm: [i8; 8],
    pub mcs: [[i8; 10]; 4],
    pub ru: [[i8; 12]; 7],
    pub eht: [[i8; 16]; 16],
    pub path: mt76_power_limits_path,
}
