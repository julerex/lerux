//! The generated C headers and the Rust layouts are one ABI.
struct IntRow {
    name: &'static str,
    value: u64,
}

struct FieldRow {
    offset: usize,
}

struct RecRow {
    size: usize,
    align: usize,
    fields: &'static [FieldRow],
}

struct Group {
    source: &'static str,
    ints: &'static [IntRow],
    strs: &'static [&'static [u8]],
    recs: &'static [RecRow],
}

static GROUPS: &[Group] = &[
    Group {
        source: r#"#include "dma.h"
#include <stdio.h>
#include <stddef.h>
int main(void) {
printf("I %s %llu\n", "MT_RING_SIZE", (unsigned long long)(MT_RING_SIZE));
printf("I %s %llu\n", "MT_DMA_CTL_SD_LEN1", (unsigned long long)(MT_DMA_CTL_SD_LEN1));
printf("I %s %llu\n", "MT_DMA_CTL_LAST_SEC1", (unsigned long long)(MT_DMA_CTL_LAST_SEC1));
printf("I %s %llu\n", "MT_DMA_CTL_M_DONE", (unsigned long long)(MT_DMA_CTL_M_DONE));
printf("I %s %llu\n", "MT_DMA_CTL_SD_LEN0", (unsigned long long)(MT_DMA_CTL_SD_LEN0));
printf("I %s %llu\n", "MT_DMA_CTL_LAST_SEC0", (unsigned long long)(MT_DMA_CTL_LAST_SEC0));
printf("I %s %llu\n", "MT_DMA_CTL_DMA_DONE", (unsigned long long)(MT_DMA_CTL_DMA_DONE));
printf("I %s %llu\n", "MT_DMA_CTL_TO_HOST", (unsigned long long)(MT_DMA_CTL_TO_HOST));
printf("I %s %llu\n", "MT_DMA_CTL_TO_HOST_A", (unsigned long long)(MT_DMA_CTL_TO_HOST_A));
printf("I %s %llu\n", "MT_DMA_CTL_DROP", (unsigned long long)(MT_DMA_CTL_DROP));
printf("I %s %llu\n", "MT_DMA_CTL_TOKEN", (unsigned long long)(MT_DMA_CTL_TOKEN));
printf("I %s %llu\n", "MT_DMA_CTL_SDP1_H", (unsigned long long)(MT_DMA_CTL_SDP1_H));
printf("I %s %llu\n", "MT_DMA_CTL_SDP0_H", (unsigned long long)(MT_DMA_CTL_SDP0_H));
printf("I %s %llu\n", "MT_DMA_CTL_WO_DROP", (unsigned long long)(MT_DMA_CTL_WO_DROP));
printf("I %s %llu\n", "MT_DMA_PPE_CPU_REASON", (unsigned long long)(MT_DMA_PPE_CPU_REASON));
printf("I %s %llu\n", "MT_DMA_PPE_ENTRY", (unsigned long long)(MT_DMA_PPE_ENTRY));
printf("I %s %llu\n", "MT_DMA_INFO_DMA_FRAG", (unsigned long long)(MT_DMA_INFO_DMA_FRAG));
printf("I %s %llu\n", "MT_DMA_INFO_PPE_VLD", (unsigned long long)(MT_DMA_INFO_PPE_VLD));
printf("I %s %llu\n", "MT_DMA_CTL_PN_CHK_FAIL", (unsigned long long)(MT_DMA_CTL_PN_CHK_FAIL));
printf("I %s %llu\n", "MT_DMA_CTL_VER_MASK", (unsigned long long)(MT_DMA_CTL_VER_MASK));
printf("I %s %llu\n", "MT_DMA_SDP0", (unsigned long long)(MT_DMA_SDP0));
printf("I %s %llu\n", "MT_DMA_TOKEN_ID", (unsigned long long)(MT_DMA_TOKEN_ID));
printf("I %s %llu\n", "MT_DMA_MAGIC_MASK", (unsigned long long)(MT_DMA_MAGIC_MASK));
printf("I %s %llu\n", "MT_DMA_RRO_EN", (unsigned long long)(MT_DMA_RRO_EN));
printf("I %s %llu\n", "MT_DMA_MAGIC_CNT", (unsigned long long)(MT_DMA_MAGIC_CNT));
printf("I %s %llu\n", "MT_DMA_WED_IND_CMD_CNT", (unsigned long long)(MT_DMA_WED_IND_CMD_CNT));
printf("I %s %llu\n", "MT_DMA_WED_IND_REASON", (unsigned long long)(MT_DMA_WED_IND_REASON));
printf("I %s %llu\n", "MT_DMA_HDR_LEN", (unsigned long long)(MT_DMA_HDR_LEN));
printf("I %s %llu\n", "MT_RX_INFO_LEN", (unsigned long long)(MT_RX_INFO_LEN));
printf("I %s %llu\n", "MT_FCE_INFO_LEN", (unsigned long long)(MT_FCE_INFO_LEN));
printf("I %s %llu\n", "MT_RX_RXWI_LEN", (unsigned long long)(MT_RX_RXWI_LEN));
printf("I %s %llu\n", "RRO_RXDMAD_DATA1_LS_MASK", (unsigned long long)(RRO_RXDMAD_DATA1_LS_MASK));
printf("I %s %llu\n", "RRO_RXDMAD_DATA1_SDL0_MASK", (unsigned long long)(RRO_RXDMAD_DATA1_SDL0_MASK));
printf("I %s %llu\n", "RRO_RXDMAD_DATA2_RX_TOKEN_ID_MASK", (unsigned long long)(RRO_RXDMAD_DATA2_RX_TOKEN_ID_MASK));
printf("I %s %llu\n", "RRO_RXDMAD_DATA2_IND_REASON_MASK", (unsigned long long)(RRO_RXDMAD_DATA2_IND_REASON_MASK));
printf("I %s %llu\n", "RRO_RXDMAD_DATA3_MAGIC_CNT_MASK", (unsigned long long)(RRO_RXDMAD_DATA3_MAGIC_CNT_MASK));
printf("I %s %llu\n", "MT_QSEL_MGMT", (unsigned long long)(MT_QSEL_MGMT));
printf("I %s %llu\n", "MT_QSEL_HCCA", (unsigned long long)(MT_QSEL_HCCA));
printf("I %s %llu\n", "MT_QSEL_EDCA", (unsigned long long)(MT_QSEL_EDCA));
printf("I %s %llu\n", "MT_QSEL_EDCA_2", (unsigned long long)(MT_QSEL_EDCA_2));
printf("I %s %llu\n", "EVT_CMD_DONE", (unsigned long long)(EVT_CMD_DONE));
printf("I %s %llu\n", "EVT_CMD_ERROR", (unsigned long long)(EVT_CMD_ERROR));
printf("I %s %llu\n", "EVT_CMD_RETRY", (unsigned long long)(EVT_CMD_RETRY));
printf("I %s %llu\n", "EVT_EVENT_PWR_RSP", (unsigned long long)(EVT_EVENT_PWR_RSP));
printf("I %s %llu\n", "EVT_EVENT_WOW_RSP", (unsigned long long)(EVT_EVENT_WOW_RSP));
printf("I %s %llu\n", "EVT_EVENT_CARRIER_DETECT_RSP", (unsigned long long)(EVT_EVENT_CARRIER_DETECT_RSP));
printf("I %s %llu\n", "EVT_EVENT_DFS_DETECT_RSP", (unsigned long long)(EVT_EVENT_DFS_DETECT_RSP));
printf("I %s %llu\n", "MT_DMA_WED_IND_REASON_NORMAL", (unsigned long long)(MT_DMA_WED_IND_REASON_NORMAL));
printf("I %s %llu\n", "MT_DMA_WED_IND_REASON_REPEAT", (unsigned long long)(MT_DMA_WED_IND_REASON_REPEAT));
printf("I %s %llu\n", "MT_DMA_WED_IND_REASON_OLDPKT", (unsigned long long)(MT_DMA_WED_IND_REASON_OLDPKT));
printf("R %zu %zu\n", sizeof(struct mt76_desc), _Alignof(struct mt76_desc));
printf("O %zu\n", offsetof(struct mt76_desc, buf0));
printf("O %zu\n", offsetof(struct mt76_desc, ctrl));
printf("O %zu\n", offsetof(struct mt76_desc, buf1));
printf("O %zu\n", offsetof(struct mt76_desc, info));
printf("R %zu %zu\n", sizeof(struct mt76_wed_rro_desc), _Alignof(struct mt76_wed_rro_desc));
printf("O %zu\n", offsetof(struct mt76_wed_rro_desc, buf0));
printf("O %zu\n", offsetof(struct mt76_wed_rro_desc, buf1));
printf("R %zu %zu\n", sizeof(struct mt76_rro_rxdmad_c), _Alignof(struct mt76_rro_rxdmad_c));
printf("O %zu\n", offsetof(struct mt76_rro_rxdmad_c, data0));
printf("O %zu\n", offsetof(struct mt76_rro_rxdmad_c, data1));
printf("O %zu\n", offsetof(struct mt76_rro_rxdmad_c, data2));
printf("O %zu\n", offsetof(struct mt76_rro_rxdmad_c, data3));
return 0; }"#,
        ints: &[
            IntRow {
                name: "MT_RING_SIZE",
                value: crate::dma::MT_RING_SIZE,
            },
            IntRow {
                name: "MT_DMA_CTL_SD_LEN1",
                value: crate::dma::MT_DMA_CTL_SD_LEN1,
            },
            IntRow {
                name: "MT_DMA_CTL_LAST_SEC1",
                value: crate::dma::MT_DMA_CTL_LAST_SEC1,
            },
            IntRow {
                name: "MT_DMA_CTL_M_DONE",
                value: crate::dma::MT_DMA_CTL_M_DONE,
            },
            IntRow {
                name: "MT_DMA_CTL_SD_LEN0",
                value: crate::dma::MT_DMA_CTL_SD_LEN0,
            },
            IntRow {
                name: "MT_DMA_CTL_LAST_SEC0",
                value: crate::dma::MT_DMA_CTL_LAST_SEC0,
            },
            IntRow {
                name: "MT_DMA_CTL_DMA_DONE",
                value: crate::dma::MT_DMA_CTL_DMA_DONE,
            },
            IntRow {
                name: "MT_DMA_CTL_TO_HOST",
                value: crate::dma::MT_DMA_CTL_TO_HOST,
            },
            IntRow {
                name: "MT_DMA_CTL_TO_HOST_A",
                value: crate::dma::MT_DMA_CTL_TO_HOST_A,
            },
            IntRow {
                name: "MT_DMA_CTL_DROP",
                value: crate::dma::MT_DMA_CTL_DROP,
            },
            IntRow {
                name: "MT_DMA_CTL_TOKEN",
                value: crate::dma::MT_DMA_CTL_TOKEN,
            },
            IntRow {
                name: "MT_DMA_CTL_SDP1_H",
                value: crate::dma::MT_DMA_CTL_SDP1_H,
            },
            IntRow {
                name: "MT_DMA_CTL_SDP0_H",
                value: crate::dma::MT_DMA_CTL_SDP0_H,
            },
            IntRow {
                name: "MT_DMA_CTL_WO_DROP",
                value: crate::dma::MT_DMA_CTL_WO_DROP,
            },
            IntRow {
                name: "MT_DMA_PPE_CPU_REASON",
                value: crate::dma::MT_DMA_PPE_CPU_REASON,
            },
            IntRow {
                name: "MT_DMA_PPE_ENTRY",
                value: crate::dma::MT_DMA_PPE_ENTRY,
            },
            IntRow {
                name: "MT_DMA_INFO_DMA_FRAG",
                value: crate::dma::MT_DMA_INFO_DMA_FRAG,
            },
            IntRow {
                name: "MT_DMA_INFO_PPE_VLD",
                value: crate::dma::MT_DMA_INFO_PPE_VLD,
            },
            IntRow {
                name: "MT_DMA_CTL_PN_CHK_FAIL",
                value: crate::dma::MT_DMA_CTL_PN_CHK_FAIL,
            },
            IntRow {
                name: "MT_DMA_CTL_VER_MASK",
                value: crate::dma::MT_DMA_CTL_VER_MASK,
            },
            IntRow {
                name: "MT_DMA_SDP0",
                value: crate::dma::MT_DMA_SDP0,
            },
            IntRow {
                name: "MT_DMA_TOKEN_ID",
                value: crate::dma::MT_DMA_TOKEN_ID,
            },
            IntRow {
                name: "MT_DMA_MAGIC_MASK",
                value: crate::dma::MT_DMA_MAGIC_MASK,
            },
            IntRow {
                name: "MT_DMA_RRO_EN",
                value: crate::dma::MT_DMA_RRO_EN,
            },
            IntRow {
                name: "MT_DMA_MAGIC_CNT",
                value: crate::dma::MT_DMA_MAGIC_CNT,
            },
            IntRow {
                name: "MT_DMA_WED_IND_CMD_CNT",
                value: crate::dma::MT_DMA_WED_IND_CMD_CNT,
            },
            IntRow {
                name: "MT_DMA_WED_IND_REASON",
                value: crate::dma::MT_DMA_WED_IND_REASON,
            },
            IntRow {
                name: "MT_DMA_HDR_LEN",
                value: crate::dma::MT_DMA_HDR_LEN,
            },
            IntRow {
                name: "MT_RX_INFO_LEN",
                value: crate::dma::MT_RX_INFO_LEN,
            },
            IntRow {
                name: "MT_FCE_INFO_LEN",
                value: crate::dma::MT_FCE_INFO_LEN,
            },
            IntRow {
                name: "MT_RX_RXWI_LEN",
                value: crate::dma::MT_RX_RXWI_LEN,
            },
            IntRow {
                name: "RRO_RXDMAD_DATA1_LS_MASK",
                value: crate::dma::RRO_RXDMAD_DATA1_LS_MASK,
            },
            IntRow {
                name: "RRO_RXDMAD_DATA1_SDL0_MASK",
                value: crate::dma::RRO_RXDMAD_DATA1_SDL0_MASK,
            },
            IntRow {
                name: "RRO_RXDMAD_DATA2_RX_TOKEN_ID_MASK",
                value: crate::dma::RRO_RXDMAD_DATA2_RX_TOKEN_ID_MASK,
            },
            IntRow {
                name: "RRO_RXDMAD_DATA2_IND_REASON_MASK",
                value: crate::dma::RRO_RXDMAD_DATA2_IND_REASON_MASK,
            },
            IntRow {
                name: "RRO_RXDMAD_DATA3_MAGIC_CNT_MASK",
                value: crate::dma::RRO_RXDMAD_DATA3_MAGIC_CNT_MASK,
            },
            IntRow {
                name: "MT_QSEL_MGMT",
                value: crate::dma::MT_QSEL_MGMT,
            },
            IntRow {
                name: "MT_QSEL_HCCA",
                value: crate::dma::MT_QSEL_HCCA,
            },
            IntRow {
                name: "MT_QSEL_EDCA",
                value: crate::dma::MT_QSEL_EDCA,
            },
            IntRow {
                name: "MT_QSEL_EDCA_2",
                value: crate::dma::MT_QSEL_EDCA_2,
            },
            IntRow {
                name: "EVT_CMD_DONE",
                value: crate::dma::EVT_CMD_DONE,
            },
            IntRow {
                name: "EVT_CMD_ERROR",
                value: crate::dma::EVT_CMD_ERROR,
            },
            IntRow {
                name: "EVT_CMD_RETRY",
                value: crate::dma::EVT_CMD_RETRY,
            },
            IntRow {
                name: "EVT_EVENT_PWR_RSP",
                value: crate::dma::EVT_EVENT_PWR_RSP,
            },
            IntRow {
                name: "EVT_EVENT_WOW_RSP",
                value: crate::dma::EVT_EVENT_WOW_RSP,
            },
            IntRow {
                name: "EVT_EVENT_CARRIER_DETECT_RSP",
                value: crate::dma::EVT_EVENT_CARRIER_DETECT_RSP,
            },
            IntRow {
                name: "EVT_EVENT_DFS_DETECT_RSP",
                value: crate::dma::EVT_EVENT_DFS_DETECT_RSP,
            },
            IntRow {
                name: "MT_DMA_WED_IND_REASON_NORMAL",
                value: crate::dma::MT_DMA_WED_IND_REASON_NORMAL,
            },
            IntRow {
                name: "MT_DMA_WED_IND_REASON_REPEAT",
                value: crate::dma::MT_DMA_WED_IND_REASON_REPEAT,
            },
            IntRow {
                name: "MT_DMA_WED_IND_REASON_OLDPKT",
                value: crate::dma::MT_DMA_WED_IND_REASON_OLDPKT,
            },
        ],
        strs: &[],
        recs: &[
            RecRow {
                size: core::mem::size_of::<crate::dma::mt76_desc>(),
                align: core::mem::align_of::<crate::dma::mt76_desc>(),
                fields: &[
                    FieldRow {
                        offset: core::mem::offset_of!(crate::dma::mt76_desc, buf0),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(crate::dma::mt76_desc, ctrl),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(crate::dma::mt76_desc, buf1),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(crate::dma::mt76_desc, info),
                    },
                ],
            },
            RecRow {
                size: core::mem::size_of::<crate::dma::mt76_wed_rro_desc>(),
                align: core::mem::align_of::<crate::dma::mt76_wed_rro_desc>(),
                fields: &[
                    FieldRow {
                        offset: core::mem::offset_of!(crate::dma::mt76_wed_rro_desc, buf0),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(crate::dma::mt76_wed_rro_desc, buf1),
                    },
                ],
            },
            RecRow {
                size: core::mem::size_of::<crate::dma::mt76_rro_rxdmad_c>(),
                align: core::mem::align_of::<crate::dma::mt76_rro_rxdmad_c>(),
                fields: &[
                    FieldRow {
                        offset: core::mem::offset_of!(crate::dma::mt76_rro_rxdmad_c, data0),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(crate::dma::mt76_rro_rxdmad_c, data1),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(crate::dma::mt76_rro_rxdmad_c, data2),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(crate::dma::mt76_rro_rxdmad_c, data3),
                    },
                ],
            },
        ],
    },
    Group {
        source: r#"#include "mt7921/mcu.h"
#include <stdio.h>
#include <stddef.h>
int main(void) {
printf("I %s %llu\n", "MT_RA_RATE_NSS", (unsigned long long)(MT_RA_RATE_NSS));
printf("I %s %llu\n", "MT_RA_RATE_MCS", (unsigned long long)(MT_RA_RATE_MCS));
printf("I %s %llu\n", "MT_RA_RATE_TX_MODE", (unsigned long long)(MT_RA_RATE_TX_MODE));
printf("I %s %llu\n", "MT_RA_RATE_DCM_EN", (unsigned long long)(MT_RA_RATE_DCM_EN));
printf("I %s %llu\n", "MT_RA_RATE_BW", (unsigned long long)(MT_RA_RATE_BW));
printf("I %s %llu\n", "MCU_EXT_EVENT_RATE_REPORT", (unsigned long long)(MCU_EXT_EVENT_RATE_REPORT));
printf("I %s %llu\n", "MT_EBF", (unsigned long long)(MT_EBF));
printf("I %s %llu\n", "MT_IBF", (unsigned long long)(MT_IBF));
printf("I %s %llu\n", "TM_SWITCH_MODE", (unsigned long long)(TM_SWITCH_MODE));
printf("I %s %llu\n", "TM_SET_AT_CMD", (unsigned long long)(TM_SET_AT_CMD));
printf("I %s %llu\n", "TM_QUERY_AT_CMD", (unsigned long long)(TM_QUERY_AT_CMD));
printf("I %s %llu\n", "MT7921_TM_NORMAL", (unsigned long long)(MT7921_TM_NORMAL));
printf("I %s %llu\n", "MT7921_TM_TESTMODE", (unsigned long long)(MT7921_TM_TESTMODE));
printf("I %s %llu\n", "MT7921_TM_ICAP", (unsigned long long)(MT7921_TM_ICAP));
printf("I %s %llu\n", "MT7921_TM_ICAP_OVERLAP", (unsigned long long)(MT7921_TM_ICAP_OVERLAP));
printf("I %s %llu\n", "MT7921_TM_WIFISPECTRUM", (unsigned long long)(MT7921_TM_WIFISPECTRUM));
printf("R %zu %zu\n", sizeof(struct mt7921_mcu_tx_done_event), _Alignof(struct mt7921_mcu_tx_done_event));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, pid));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, status));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, seq));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, wlan_idx));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, tx_cnt));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, tx_rate));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, flag));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, tid));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, rsp_rate));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, mcs));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, bw));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, tx_pwr));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, reason));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, rsv0));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, delay));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, timestamp));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, applied_flag));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, txs));
printf("O %zu\n", offsetof(struct mt7921_mcu_tx_done_event, rsv1));
printf("R %zu %zu\n", sizeof(struct mt7921_mcu_ant_id_config), _Alignof(struct mt7921_mcu_ant_id_config));
printf("O %zu\n", offsetof(struct mt7921_mcu_ant_id_config, ant_id));
printf("R %zu %zu\n", sizeof(struct mt7921_txpwr_req), _Alignof(struct mt7921_txpwr_req));
printf("O %zu\n", offsetof(struct mt7921_txpwr_req, ver));
printf("O %zu\n", offsetof(struct mt7921_txpwr_req, action));
printf("O %zu\n", offsetof(struct mt7921_txpwr_req, len));
printf("O %zu\n", offsetof(struct mt7921_txpwr_req, dbdc_idx));
printf("O %zu\n", offsetof(struct mt7921_txpwr_req, rsv));
printf("R %zu %zu\n", sizeof(struct mt7921_wf_rf_pin_ctrl_event), _Alignof(struct mt7921_wf_rf_pin_ctrl_event));
printf("O %zu\n", offsetof(struct mt7921_wf_rf_pin_ctrl_event, result));
printf("O %zu\n", offsetof(struct mt7921_wf_rf_pin_ctrl_event, value));
printf("R %zu %zu\n", sizeof(struct mt7921_rftest_cmd), _Alignof(struct mt7921_rftest_cmd));
printf("O %zu\n", offsetof(struct mt7921_rftest_cmd, action));
printf("O %zu\n", offsetof(struct mt7921_rftest_cmd, rsv));
printf("O %zu\n", offsetof(struct mt7921_rftest_cmd, param0));
printf("O %zu\n", offsetof(struct mt7921_rftest_cmd, param1));
printf("R %zu %zu\n", sizeof(struct mt7921_rftest_evt), _Alignof(struct mt7921_rftest_evt));
printf("O %zu\n", offsetof(struct mt7921_rftest_evt, param0));
printf("O %zu\n", offsetof(struct mt7921_rftest_evt, param1));
printf("R %zu %zu\n", sizeof(struct mt7921_clc_info_tlv), _Alignof(struct mt7921_clc_info_tlv));
printf("O %zu\n", offsetof(struct mt7921_clc_info_tlv, tag));
printf("O %zu\n", offsetof(struct mt7921_clc_info_tlv, len));
printf("O %zu\n", offsetof(struct mt7921_clc_info_tlv, chan_conf));
printf("O %zu\n", offsetof(struct mt7921_clc_info_tlv, rsv));
return 0; }"#,
        ints: &[
            IntRow {
                name: "MT_RA_RATE_NSS",
                value: crate::mt7921::mcu::MT_RA_RATE_NSS,
            },
            IntRow {
                name: "MT_RA_RATE_MCS",
                value: crate::mt7921::mcu::MT_RA_RATE_MCS,
            },
            IntRow {
                name: "MT_RA_RATE_TX_MODE",
                value: crate::mt7921::mcu::MT_RA_RATE_TX_MODE,
            },
            IntRow {
                name: "MT_RA_RATE_DCM_EN",
                value: crate::mt7921::mcu::MT_RA_RATE_DCM_EN,
            },
            IntRow {
                name: "MT_RA_RATE_BW",
                value: crate::mt7921::mcu::MT_RA_RATE_BW,
            },
            IntRow {
                name: "MCU_EXT_EVENT_RATE_REPORT",
                value: crate::mt7921::mcu::MCU_EXT_EVENT_RATE_REPORT,
            },
            IntRow {
                name: "MT_EBF",
                value: crate::mt7921::mcu::MT_EBF,
            },
            IntRow {
                name: "MT_IBF",
                value: crate::mt7921::mcu::MT_IBF,
            },
            IntRow {
                name: "TM_SWITCH_MODE",
                value: crate::mt7921::mcu::TM_SWITCH_MODE,
            },
            IntRow {
                name: "TM_SET_AT_CMD",
                value: crate::mt7921::mcu::TM_SET_AT_CMD,
            },
            IntRow {
                name: "TM_QUERY_AT_CMD",
                value: crate::mt7921::mcu::TM_QUERY_AT_CMD,
            },
            IntRow {
                name: "MT7921_TM_NORMAL",
                value: crate::mt7921::mcu::MT7921_TM_NORMAL,
            },
            IntRow {
                name: "MT7921_TM_TESTMODE",
                value: crate::mt7921::mcu::MT7921_TM_TESTMODE,
            },
            IntRow {
                name: "MT7921_TM_ICAP",
                value: crate::mt7921::mcu::MT7921_TM_ICAP,
            },
            IntRow {
                name: "MT7921_TM_ICAP_OVERLAP",
                value: crate::mt7921::mcu::MT7921_TM_ICAP_OVERLAP,
            },
            IntRow {
                name: "MT7921_TM_WIFISPECTRUM",
                value: crate::mt7921::mcu::MT7921_TM_WIFISPECTRUM,
            },
        ],
        strs: &[],
        recs: &[
            RecRow {
                size: core::mem::size_of::<crate::mt7921::mcu::mt7921_mcu_tx_done_event>(),
                align: core::mem::align_of::<crate::mt7921::mcu::mt7921_mcu_tx_done_event>(),
                fields: &[
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            pid
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            status
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            seq
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            wlan_idx
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            tx_cnt
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            tx_rate
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            flag
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            tid
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            rsp_rate
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            mcs
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            bw
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            tx_pwr
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            reason
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            rsv0
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            delay
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            timestamp
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            applied_flag
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            txs
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_mcu_tx_done_event,
                            rsv1
                        ),
                    },
                ],
            },
            RecRow {
                size: core::mem::size_of::<crate::mt7921::mcu::mt7921_mcu_ant_id_config>(),
                align: core::mem::align_of::<crate::mt7921::mcu::mt7921_mcu_ant_id_config>(),
                fields: &[FieldRow {
                    offset: core::mem::offset_of!(
                        crate::mt7921::mcu::mt7921_mcu_ant_id_config,
                        ant_id
                    ),
                }],
            },
            RecRow {
                size: core::mem::size_of::<crate::mt7921::mcu::mt7921_txpwr_req>(),
                align: core::mem::align_of::<crate::mt7921::mcu::mt7921_txpwr_req>(),
                fields: &[
                    FieldRow {
                        offset: core::mem::offset_of!(crate::mt7921::mcu::mt7921_txpwr_req, ver),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(crate::mt7921::mcu::mt7921_txpwr_req, action),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(crate::mt7921::mcu::mt7921_txpwr_req, len),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_txpwr_req,
                            dbdc_idx
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(crate::mt7921::mcu::mt7921_txpwr_req, rsv),
                    },
                ],
            },
            RecRow {
                size: core::mem::size_of::<crate::mt7921::mcu::mt7921_wf_rf_pin_ctrl_event>(),
                align: core::mem::align_of::<crate::mt7921::mcu::mt7921_wf_rf_pin_ctrl_event>(),
                fields: &[
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_wf_rf_pin_ctrl_event,
                            result
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_wf_rf_pin_ctrl_event,
                            value
                        ),
                    },
                ],
            },
            RecRow {
                size: core::mem::size_of::<crate::mt7921::mcu::mt7921_rftest_cmd>(),
                align: core::mem::align_of::<crate::mt7921::mcu::mt7921_rftest_cmd>(),
                fields: &[
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_rftest_cmd,
                            action
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(crate::mt7921::mcu::mt7921_rftest_cmd, rsv),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_rftest_cmd,
                            param0
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_rftest_cmd,
                            param1
                        ),
                    },
                ],
            },
            RecRow {
                size: core::mem::size_of::<crate::mt7921::mcu::mt7921_rftest_evt>(),
                align: core::mem::align_of::<crate::mt7921::mcu::mt7921_rftest_evt>(),
                fields: &[
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_rftest_evt,
                            param0
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_rftest_evt,
                            param1
                        ),
                    },
                ],
            },
            RecRow {
                size: core::mem::size_of::<crate::mt7921::mcu::mt7921_clc_info_tlv>(),
                align: core::mem::align_of::<crate::mt7921::mcu::mt7921_clc_info_tlv>(),
                fields: &[
                    FieldRow {
                        offset: core::mem::offset_of!(crate::mt7921::mcu::mt7921_clc_info_tlv, tag),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(crate::mt7921::mcu::mt7921_clc_info_tlv, len),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(
                            crate::mt7921::mcu::mt7921_clc_info_tlv,
                            chan_conf
                        ),
                    },
                    FieldRow {
                        offset: core::mem::offset_of!(crate::mt7921::mcu::mt7921_clc_info_tlv, rsv),
                    },
                ],
            },
        ],
    },
];

#[test]
fn generated_c_matches_rust() {
    for group in GROUPS {
        check_group(group);
    }
}

fn check_group(group: &Group) {
    let output = compile(group.source);
    let lines: Vec<&str> = output.lines().collect();
    let mut cursor = 0;
    cursor = check_ints(group, &lines, cursor);
    cursor = check_strs(group, &lines, cursor);
    check_recs(group, &lines, cursor);
}

fn check_ints(group: &Group, lines: &[&str], mut cursor: usize) -> usize {
    for row in group.ints {
        let parts: Vec<&str> = lines[cursor].split_whitespace().collect();
        assert_eq!(parts.first().copied(), Some("I"), "{}", row.name);
        let got: u64 = parts[2].parse().expect("integer");
        assert_eq!(got, row.value, "{}", row.name);
        cursor += 1;
    }
    cursor
}

fn check_strs(group: &Group, lines: &[&str], mut cursor: usize) -> usize {
    for row in group.strs {
        let line = lines[cursor];
        let rest = line.strip_prefix("S ").expect("string row");
        let (len_text, text) = rest.split_once(' ').unwrap_or((rest, ""));
        let len: usize = len_text.parse().expect("length");
        assert_eq!(len, row.len(), "string length");
        assert_eq!(text.as_bytes(), *row, "string bytes");
        cursor += 1;
    }
    cursor
}

fn check_recs(group: &Group, lines: &[&str], mut cursor: usize) {
    for row in group.recs {
        let parts: Vec<&str> = lines[cursor].split_whitespace().collect();
        cursor += 1;
        assert_eq!(parts.first().copied(), Some("R"));
        let size: usize = parts[1].parse().expect("size");
        let align: usize = parts[2].parse().expect("align");
        assert_eq!(size, row.size, "record size");
        assert_eq!(align, row.align, "record align");
        for field in row.fields {
            let got: usize = lines[cursor]
                .split_whitespace()
                .nth(1)
                .expect("offset")
                .parse()
                .expect("offset");
            assert_eq!(got, field.offset, "field offset");
            cursor += 1;
        }
    }
}

fn compile(source: &str) -> String {
    let dir = std::env::temp_dir().join("lerux-mt79-proof");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("probe.c");
    std::fs::write(&path, source).expect("probe source");
    let binary = dir.join("probe");
    let include = concat!(env!("CARGO_MANIFEST_DIR"), "/include");
    let compiled = std::process::Command::new("gcc")
        .args(["-std=gnu11", "-I", include])
        .arg(&path)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("gcc");
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr),
    );
    let ran = std::process::Command::new(&binary).output().expect("probe");
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    String::from_utf8(ran.stdout).expect("utf8")
}
