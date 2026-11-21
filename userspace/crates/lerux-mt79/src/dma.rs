//! `dma.h` layouts reconstructed from Linux v7.3.5 mt76.
#[allow(unused_imports)] // function-like macros call the unsigned helpers
use crate::bits::{bit, div, genmask, rem, shl, shr};
#[allow(unused_imports)] // fields name endian wrappers directly
use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};

pub const MT_RING_SIZE: u64 = 16;
pub const MT_DMA_CTL_SD_LEN1: u64 = 0x3fff;
pub const MT_DMA_CTL_LAST_SEC1: u64 = 0x4000;
pub const MT_DMA_CTL_M_DONE: u64 = 0x8000;
pub const MT_DMA_CTL_SD_LEN0: u64 = 0x3fff_0000;
pub const MT_DMA_CTL_LAST_SEC0: u64 = 0x4000_0000;
pub const MT_DMA_CTL_DMA_DONE: u64 = 0x8000_0000;
pub const MT_DMA_CTL_TO_HOST: u64 = 256;
pub const MT_DMA_CTL_TO_HOST_A: u64 = 0x1000;
pub const MT_DMA_CTL_DROP: u64 = 0x4000;
pub const MT_DMA_CTL_TOKEN: u64 = 0xffff_0000;
pub const MT_DMA_CTL_SDP1_H: u64 = 0xf_0000;
pub const MT_DMA_CTL_SDP0_H: u64 = 15;
pub const MT_DMA_CTL_WO_DROP: u64 = 256;
pub const MT_DMA_PPE_CPU_REASON: u64 = 0xf800;
pub const MT_DMA_PPE_ENTRY: u64 = 0x7fff_0000;
pub const MT_DMA_INFO_DMA_FRAG: u64 = 512;
pub const MT_DMA_INFO_PPE_VLD: u64 = 0x8000_0000;
pub const MT_DMA_CTL_PN_CHK_FAIL: u64 = 0x2000;
pub const MT_DMA_CTL_VER_MASK: u64 = 128;
pub const MT_DMA_SDP0: u64 = 0xffff;
pub const MT_DMA_TOKEN_ID: u64 = 0xffff_0000;
pub const MT_DMA_MAGIC_MASK: u64 = 0xf000_0000;
pub const MT_DMA_RRO_EN: u64 = 0x2000;
pub const MT_DMA_MAGIC_CNT: u64 = 16;
pub const MT_DMA_WED_IND_CMD_CNT: u64 = 8;
pub const MT_DMA_WED_IND_REASON: u64 = 0xf000;
pub const MT_DMA_HDR_LEN: u64 = 4;
pub const MT_RX_INFO_LEN: u64 = 4;
pub const MT_FCE_INFO_LEN: u64 = 4;
pub const MT_RX_RXWI_LEN: u64 = 32;
pub const RRO_RXDMAD_DATA1_LS_MASK: u64 = 0x4000_0000;
pub const RRO_RXDMAD_DATA1_SDL0_MASK: u64 = 0x3fff_0000;
pub const RRO_RXDMAD_DATA2_RX_TOKEN_ID_MASK: u64 = 0xffff_0000;
pub const RRO_RXDMAD_DATA2_IND_REASON_MASK: u64 = 0xf000;
pub const RRO_RXDMAD_DATA3_MAGIC_CNT_MASK: u64 = 0xf000_0000;
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct mt76_desc {
    pub buf0: Le32,
    pub ctrl: Le32,
    pub buf1: Le32,
    pub info: Le32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct mt76_wed_rro_desc {
    pub buf0: Le32,
    pub buf1: Le32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct mt76_rro_rxdmad_c {
    pub data0: Le32,
    pub data1: Le32,
    pub data2: Le32,
    pub data3: Le32,
}

pub const MT_QSEL_MGMT: u64 = 0;
pub const MT_QSEL_HCCA: u64 = 1;
pub const MT_QSEL_EDCA: u64 = 2;
pub const MT_QSEL_EDCA_2: u64 = 3;
pub const EVT_CMD_DONE: u64 = 0;
pub const EVT_CMD_ERROR: u64 = 1;
pub const EVT_CMD_RETRY: u64 = 2;
pub const EVT_EVENT_PWR_RSP: u64 = 3;
pub const EVT_EVENT_WOW_RSP: u64 = 4;
pub const EVT_EVENT_CARRIER_DETECT_RSP: u64 = 5;
pub const EVT_EVENT_DFS_DETECT_RSP: u64 = 6;
pub const MT_DMA_WED_IND_REASON_NORMAL: u64 = 0;
pub const MT_DMA_WED_IND_REASON_REPEAT: u64 = 1;
pub const MT_DMA_WED_IND_REASON_OLDPKT: u64 = 2;
