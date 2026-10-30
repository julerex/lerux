//! MT7921 host sequence from Linux v6.8 `mt76`.
//!
//! Register addresses and firmware-download steps are the ones in
//! `mt7921/pci.c`, `mt792x_dma.c`, and `mt76_connac_mcu.c`. A register that
//! those files do not map is [`Map::Unsupported`].

use crate::wifi::{format_report, WifiReport};

/// `mt792x_patch_name` / `mt792x_ram_name`. 0x7922 and 0x7925 are different images.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FirmwareKind {
    Mt7961,
    Unsupported,
}

pub fn firmware_kind(chip: u32) -> FirmwareKind {
    match chip {
        0x7922 | 0x7925 => FirmwareKind::Unsupported,
        _ => FirmwareKind::Mt7961,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Own,
    Fw,
    Mac,
    Wait,
    Scan,
    Auth,
    Assoc,
    Eapol,
    Dhcp,
    Ping,
    PingOk([u8; 4]),
}

impl Stage {
    pub fn paint(self, out: &mut [u8]) -> usize {
        let word: &[u8] = match self {
            Stage::Own => b"own",
            Stage::Fw => b"fw",
            Stage::Mac => b"mac",
            Stage::Wait => b"wait",
            Stage::Scan => b"scan",
            Stage::Auth => b"auth",
            Stage::Assoc => b"assoc",
            Stage::Eapol => b"eapol",
            Stage::Dhcp => b"dhcp",
            Stage::Ping => b"ping",
            Stage::PingOk(addr) => {
                return format_report(&WifiReport::PingOk(addr), out);
            }
        };
        let prefix = b"mt7921: ";
        let n = prefix.len() + word.len();
        if out.len() < n {
            return 0;
        }
        out[..prefix.len()].copy_from_slice(prefix);
        out[prefix.len()..n].copy_from_slice(word);
        n
    }
}

/// Where a Linux `mt76_rr` address lands in the 2 MiB BAR.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Map {
    Direct(u32),
    /// Write `base` into `MT_HIF_REMAP_L1`, then use `window`.
    Remap {
        base: u32,
        window: u32,
    },
    Unsupported,
}

/// `__mt7921_reg_addr`. The fixed windows are the table in `mt7921/pci.c`.
pub fn map_reg(addr: u32) -> Map {
    if addr < 0x10_0000 {
        return Map::Direct(addr);
    }
    for (phys, maps, size) in FIXED {
        if addr < phys {
            continue;
        }
        let ofs = addr - phys;
        if ofs > size {
            continue;
        }
        return Map::Direct(maps + ofs);
    }
    if (0x1800_0000..0x18c0_0000).contains(&addr)
        || (0x7000_0000..0x7800_0000).contains(&addr)
        || (0x7c00_0000..0x7c40_0000).contains(&addr)
    {
        return Map::Remap {
            base: addr >> 16,
            window: 0x4_0000 + (addr & 0xffff),
        };
    }
    Map::Unsupported
}

/// `(phys, bar, size)` from `mt7921/pci.c` `fixed_map`.
const FIXED: [(u32, u32, u32); 44] = [
    (0x820d_0000, 0x3_0000, 0x1_0000),
    (0x820e_d000, 0x2_4800, 0x00800),
    (0x820e_4000, 0x2_1000, 0x00400),
    (0x820e_7000, 0x2_1e00, 0x00200),
    (0x820e_b000, 0x2_4200, 0x00400),
    (0x820e_2000, 0x2_0800, 0x00400),
    (0x820e_3000, 0x2_0c00, 0x00400),
    (0x820e_5000, 0x2_1400, 0x00800),
    (0x0040_0000, 0x8_0000, 0x1_0000),
    (0x0041_0000, 0x9_0000, 0x1_0000),
    (0x4000_0000, 0x7_0000, 0x1_0000),
    (0x5400_0000, 0x0_2000, 0x01000),
    (0x5500_0000, 0x0_3000, 0x01000),
    (0x5800_0000, 0x0_6000, 0x01000),
    (0x5900_0000, 0x0_7000, 0x01000),
    (0x7c00_0000, 0xf_0000, 0x1_0000),
    (0x7c02_0000, 0xd_0000, 0x1_0000),
    (0x7c06_0000, 0xe_0000, 0x1_0000),
    (0x8002_0000, 0xb_0000, 0x1_0000),
    (0x8102_0000, 0xc_0000, 0x1_0000),
    (0x820c_0000, 0x0_8000, 0x04000),
    (0x820c_8000, 0x0_c000, 0x02000),
    (0x820c_c000, 0x0_e000, 0x01000),
    (0x820c_d000, 0x0_f000, 0x01000),
    (0x7403_0000, 0x1_0000, 0x1_0000),
    (0x820c_e000, 0x2_1c00, 0x00200),
    (0x820c_f000, 0x2_2000, 0x01000),
    (0x820e_0000, 0x2_0000, 0x00400),
    (0x820e_1000, 0x2_0400, 0x00200),
    (0x820e_9000, 0x2_3400, 0x00200),
    (0x820e_a000, 0x2_4000, 0x00200),
    (0x820e_c000, 0x2_4600, 0x00200),
    (0x820f_0000, 0xa_0000, 0x00400),
    (0x820f_1000, 0xa_0600, 0x00200),
    (0x820f_2000, 0xa_0800, 0x00400),
    (0x820f_3000, 0xa_0c00, 0x00400),
    (0x820f_4000, 0xa_1000, 0x00400),
    (0x820f_5000, 0xa_1400, 0x00800),
    (0x820f_7000, 0xa_1e00, 0x00200),
    (0x820f_9000, 0xa_3400, 0x00200),
    (0x820f_a000, 0xa_4000, 0x00200),
    (0x820f_b000, 0xa_4200, 0x00400),
    (0x820f_c000, 0xa_4600, 0x00200),
    (0x820f_d000, 0xa_4800, 0x00800),
];

pub const CONN_ON_LPCTL: u32 = 0x7c06_0010;
pub const CONN_ON_MISC: u32 = 0x7c06_00f0;
pub const HOST_SET_OWN: u32 = 1 << 0;
pub const HOST_CLR_OWN: u32 = 1 << 1;
pub const HOST_OWN_SYNC: u32 = 1 << 2;
pub const FW_N9_RDY: u32 = 0b11;
pub const HW_CHIPID: u32 = 0x7001_0200;
pub const HW_REV: u32 = 0x7001_0204;
pub const WFSYS_RST: u32 = 0x1800_0140;
pub const WFSYS_RST_B: u32 = 1 << 0;
pub const WFSYS_INIT_DONE: u32 = 1 << 4;
pub const HIF_REMAP_L1: u32 = 0xfe24c;
pub const SWDEF_MODE: u32 = 0x41_f23c;
pub const WFDMA0: u32 = 0xd_4000;
pub const GLO_CFG: u32 = WFDMA0 + 0x208;
pub const GLO_CFG_TX_EN: u32 = 1 << 0;
pub const GLO_CFG_TX_BUSY: u32 = 1 << 1;
pub const GLO_CFG_RX_EN: u32 = 1 << 2;
pub const GLO_CFG_RX_BUSY: u32 = 1 << 3;
pub const GLO_CFG_TX_WB: u32 = 1 << 6;
pub const GLO_CFG_LE: u32 = 1 << 12;
pub const GLO_CFG_CHAIN: u32 = 1 << 15;
pub const GLO_CFG_OMIT_RX_PFET: u32 = 1 << 21;
pub const GLO_CFG_OMIT_TX: u32 = 1 << 28;
pub const GLO_CFG_CLK: u32 = 1 << 30;
pub const GLO_CFG_OMIT_RX: u32 = 1 << 27;
pub const WFDMA_RST: u32 = WFDMA0 + 0x100;
pub const RST_LOGIC: u32 = 1 << 4;
pub const RST_DMASHDL: u32 = 1 << 5;
pub const GLO_CFG_EXT0: u32 = WFDMA0 + 0x2b0;
pub const DMASHDL_ENABLE: u32 = 1 << 6;
pub const DMASHDL_SW: u32 = 0x7c02_6004;
pub const DMASHDL_BYPASS: u32 = 1 << 28;
pub const RST_DTX_PTR: u32 = WFDMA0 + 0x20c;
pub const PRI_DLY: u32 = WFDMA0 + 0x2f0;
pub const DUMMY_CR: u32 = 0x5400_0120;
pub const NEED_REINIT: u32 = 1 << 1;
pub const HOST_INT_ENA: u32 = WFDMA0 + 0x204;
pub const MCU2HOST: u32 = WFDMA0 + 0x1f4;
pub const MCU_WAKE: u32 = 1;
pub const PCIE_MAC_INT: u32 = 0x1_0188;
pub const PCIE_MAC_PM: u32 = 0x1_0194;
pub const L0S_DIS: u32 = 1 << 8;
pub const TX_RING_BASE: u32 = WFDMA0 + 0x300;
pub const RX_EVENT_BASE: u32 = WFDMA0 + 0x500;
pub const RX_DATA_BASE: u32 = WFDMA0 + 0x520;
pub const RX_WA_BASE: u32 = WFDMA0 + 0x540;
pub const TOP_LPCR: u32 = 0x1806_0010;
pub const LPCR_DRV_OWN: u32 = 1 << 1;
pub const LPCR_FW_OWN: u32 = 1 << 0;
pub const MDP_DCR0: u32 = 0x820c_d000;
pub const MDP_DCR1: u32 = 0x820c_d004;
pub const MDP_DAMSDU: u32 = 1 << 15;
pub const MDP_HDR_TRANS: u32 = 1 << 19;
pub const WTBL_UPDATE: u32 = 0x820d_4230;
pub const WTBL_BUSY: u32 = 1 << 31;
pub const RING_STRIDE: u32 = 0x10;
pub const DMA_DONE: u32 = 1 << 31;
pub const LAST_SEC0: u32 = 1 << 30;
pub const RXD_LEN: usize = 36;
pub const MCU_TXD_LEN: usize = 64;
pub const TXD_LEN: usize = 32;
pub const TXP_LEN: usize = 32;
pub const DESC_LEN: usize = 16;

pub fn ring_regs(base: u32, idx: u32) -> u32 {
    base + idx * RING_STRIDE
}

/// `PREFETCH(base, depth)` values `mt792x_dma_prefetch` writes for MT7921.
pub fn prefetch() -> [(u32, u32); 14] {
    const fn word(base: u32, depth: u32) -> u32 {
        (base << 16) | depth
    }
    [
        (WFDMA0 + 0x680, word(0x0, 0x4)),
        (WFDMA0 + 0x688, word(0x40, 0x4)),
        (WFDMA0 + 0x68c, word(0x80, 0x4)),
        (WFDMA0 + 0x690, word(0xc0, 0x4)),
        (WFDMA0 + 0x694, word(0x100, 0x4)),
        (WFDMA0 + 0x600, word(0x140, 0x4)),
        (WFDMA0 + 0x604, word(0x180, 0x4)),
        (WFDMA0 + 0x608, word(0x1c0, 0x4)),
        (WFDMA0 + 0x60c, word(0x200, 0x4)),
        (WFDMA0 + 0x610, word(0x240, 0x4)),
        (WFDMA0 + 0x614, word(0x280, 0x4)),
        (WFDMA0 + 0x618, word(0x2c0, 0x4)),
        (WFDMA0 + 0x640, word(0x340, 0x4)),
        (WFDMA0 + 0x644, word(0x380, 0x4)),
    ]
}

/// Interrupt mask `mt792x_dma_enable` programs from the MT7921 irq map.
pub const IRQ_MASK: u32 = {
    let tx_all = (1 << 27) | (1 << 4) | 0x7_fff0;
    let rx_data = 1 << 2;
    let rx_wm2 = 1 << 22;
    let rx_wm = 1 << 0;
    let mcu = 1 << 29;
    tx_all | rx_data | rx_wm2 | rx_wm | mcu
};

pub const DL_NEED_RSP: u32 = 1 << 31;
pub const PATCH_START: u32 = 0x05;
pub const TARGET_LEN: u32 = 0x01;
pub const FW_START: u32 = 0x02;
pub const PATCH_FINISH: u32 = 0x07;
pub const PATCH_SEM: u32 = 0x10;
pub const FW_SCATTER: u32 = 0xee;
pub const EXT_CID: u32 = 0xed;
const CE: u32 = 1 << 18;
const QUERY: u32 = 1 << 16;

pub fn ext_cmd(id: u32) -> u32 {
    EXT_CID | (id << 8)
}

pub fn ext_query(id: u32) -> u32 {
    ext_cmd(id) | QUERY
}

pub fn ce_cmd(id: u32) -> u32 {
    id | CE
}

/// `mt76_connac_mcu_init_download` command choice for MT7921.
pub fn download_cmd(addr: u32) -> u32 {
    if addr == 0x20_0000 || addr == 0x90_0000 {
        PATCH_START
    } else {
        TARGET_LEN
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub addr: u32,
    pub off: usize,
    pub len: usize,
    pub mode: u32,
    pub skip: bool,
    pub override_addr: bool,
}

const PATCH_HDR: usize = 96;
const PATCH_SEC: usize = 64;
const RAM_TRAILER: usize = 36;
const RAM_REGION: usize = 40;
const MAX_REGIONS: usize = 8;

pub fn patch_regions(image: &[u8], out: &mut [Region]) -> Option<usize> {
    if image.len() < PATCH_HDR || out.is_empty() {
        return None;
    }
    let n = u32::from_be_bytes(image[44..48].try_into().ok()?) as usize;
    if n == 0 || n > out.len().min(MAX_REGIONS) {
        return None;
    }
    for (i, slot) in out.iter_mut().enumerate().take(n) {
        let at = PATCH_HDR + i * PATCH_SEC;
        let sec = image.get(at..at + PATCH_SEC)?;
        let kind = u32::from_be_bytes(sec[0..4].try_into().ok()?);
        if kind & 0xffff != 2 {
            return None;
        }
        let addr = u32::from_be_bytes(sec[12..16].try_into().ok()?);
        let len = u32::from_be_bytes(sec[16..20].try_into().ok()?) as usize;
        let data_off = u32::from_be_bytes(sec[4..8].try_into().ok()?) as usize;
        let info = u32::from_be_bytes(sec[20..24].try_into().ok()?);
        let mode = patch_mode(info)?;
        if data_off.checked_add(len)? > image.len() {
            return None;
        }
        *slot = Region {
            addr,
            off: data_off,
            len,
            mode,
            skip: false,
            override_addr: false,
        };
    }
    Some(n)
}

fn patch_mode(info: u32) -> Option<u32> {
    if info == 0xffff_ffff {
        return Some(DL_NEED_RSP);
    }
    let enc = info >> 24;
    let mut mode = DL_NEED_RSP;
    match enc {
        0 => Some(mode),
        1 => {
            mode |= 1 | (1 << 3);
            mode |= (info & 3) << 1;
            Some(mode)
        }
        2 => {
            mode |= 1 | (1 << 6) | (1 << 3);
            Some(mode)
        }
        _ => None,
    }
}

pub fn ram_regions(image: &[u8], out: &mut [Region]) -> Option<(usize, u32, u32)> {
    if image.len() < RAM_TRAILER || out.is_empty() {
        return None;
    }
    let trailer = image.len() - RAM_TRAILER;
    let n = image[trailer + 2] as usize;
    if n == 0 || n > out.len().min(MAX_REGIONS) {
        return None;
    }
    let mut data = 0usize;
    let mut override_addr = 0u32;
    let mut option = 0u32;
    for (i, slot) in out.iter_mut().enumerate().take(n) {
        let at = trailer - (n - i) * RAM_REGION;
        let region = image.get(at..at + RAM_REGION)?;
        let len = u32::from_le_bytes(region[20..24].try_into().ok()?) as usize;
        let addr = u32::from_le_bytes(region[16..20].try_into().ok()?);
        let feature = region[24];
        let skip = feature & (1 << 6) != 0;
        if feature & (1 << 5) != 0 {
            override_addr = addr;
            option |= 1;
        }
        if data.checked_add(len)? > trailer {
            return None;
        }
        *slot = Region {
            addr,
            off: data,
            len,
            mode: ram_mode(feature),
            skip,
            override_addr: feature & (1 << 5) != 0,
        };
        data += len;
    }
    Some((n, override_addr, option))
}

fn ram_mode(feature: u8) -> u32 {
    let mut mode = DL_NEED_RSP;
    if feature & 1 != 0 {
        mode |= 1 | (1 << 3);
    }
    if feature & (1 << 4) != 0 {
        mode |= 1 << 6;
    }
    mode |= u32::from(feature & 0b110);
    mode
}

/// Hardware TXD plus the connac2 MCU header. Payload follows at [`MCU_TXD_LEN`].
pub fn write_mcu_txd(buf: &mut [u8], cmd: u32, seq: u8, payload: usize) -> bool {
    if buf.len() < MCU_TXD_LEN {
        return false;
    }
    buf[..MCU_TXD_LEN].fill(0);
    let total = MCU_TXD_LEN + payload;
    let tx0 = (total as u32 & 0xffff) | (2 << 23) | (0x20 << 25);
    let tx1: u32 = 1 << 31 | (1 << 16);
    buf[..4].copy_from_slice(&tx0.to_le_bytes());
    buf[4..8].copy_from_slice(&tx1.to_le_bytes());
    let body = payload as u16;
    buf[32..34].copy_from_slice(&body.to_le_bytes());
    let port = 1u16;
    let queue = 0x20u16;
    let pq = (port << 15) | (queue << 10);
    buf[34..36].copy_from_slice(&pq.to_le_bytes());
    buf[36] = (cmd & 0xff) as u8;
    buf[37] = 0xa0;
    let ext = ((cmd >> 8) & 0xff) as u8;
    let ce = cmd & CE != 0;
    if ext != 0 || ce {
        buf[38] = if cmd & QUERY != 0 { 0 } else { 1 };
        buf[43] = u8::from(ext != 0);
    } else {
        buf[38] = 3;
    }
    buf[39] = seq;
    buf[41] = ext;
    buf[42] = 0;
    true
}

pub fn rxd_seq(buf: &[u8]) -> Option<u8> {
    buf.get(29).copied()
}

pub fn rxd_eid(buf: &[u8]) -> Option<u8> {
    buf.get(28).copied()
}

/// Status byte `mt7921_mcu_parse_response` reads for the patch semaphore.
pub fn patch_status(buf: &[u8]) -> Option<u8> {
    buf.get(32).copied()
}

pub const PATCH_IS_DL: u8 = 1;
pub const PATCH_NOT_DL: u8 = 2;
pub const PATCH_REL: u8 = 3;
pub const SCAN_DONE: u8 = 0x0d;

const SCAN_SSID: usize = 36;
const SCAN_CH: usize = 2;

/// Byte length of `mt76_connac_hw_scan_req`.
pub const SCAN_REQ_LEN: usize = 1186;

/// Active scan for one SSID. Layout is `mt76_connac_hw_scan_req` in Linux v6.8.
pub fn write_scan(out: &mut [u8], ssid: &[u8], seq: u8) -> bool {
    if out.len() < SCAN_REQ_LEN || ssid.is_empty() || ssid.len() > 32 {
        return false;
    }
    out[..SCAN_REQ_LEN].fill(0);
    out[0] = seq & 0x7f;
    out[2] = 1;
    out[3] = 1 << 2;
    out[4] = 1;
    out[5] = 2;
    out[6] = 1 << 5;
    out[7] = 1;
    out[8..12].copy_from_slice(&(ssid.len() as u32).to_le_bytes());
    out[12..12 + ssid.len()].copy_from_slice(ssid);
    let channels: &[(u8, u8)] = &[
        (1, 1),
        (1, 2),
        (1, 3),
        (1, 4),
        (1, 5),
        (1, 6),
        (1, 7),
        (1, 8),
        (1, 9),
        (1, 10),
        (1, 11),
        (1, 12),
        (1, 13),
        (2, 36),
        (2, 40),
        (2, 44),
        (2, 48),
        (2, 149),
        (2, 153),
        (2, 157),
        (2, 161),
    ];
    let base = 8 + 4 * SCAN_SSID + 6;
    out[base] = 4;
    out[base + 1] = channels.len() as u8;
    for (i, (band, ch)) in channels.iter().enumerate() {
        let at = base + 2 + i * SCAN_CH;
        out[at] = *band;
        out[at + 1] = *ch;
    }
    let after_ies = base + 2 + 32 * SCAN_CH + 2 + 600;
    out[after_ies + 2] = 0;
    out[after_ies + 3] = 0;
    let bssid_at = after_ies + 4 + 32 * SCAN_CH + 6 * SCAN_SSID;
    out[bssid_at..bssid_at + 6].fill(0xff);
    let ext_flag = bssid_at + 12 + 63;
    if ext_flag < SCAN_REQ_LEN {
        out[ext_flag] = 1;
    }
    true
}

/// `mt76_connac2_mac_write_txwi` for one 802.11 frame on the ALTX queue.
pub fn write_txwi(txwi: &mut [u8], frame: &[u8], frame_phys: u32) -> bool {
    if txwi.len() < TXD_LEN + TXP_LEN || frame.len() < 24 || frame.len() > 0x3fff {
        return false;
    }
    txwi[..TXD_LEN + TXP_LEN].fill(0);
    let total = frame.len() + TXD_LEN;
    let q = 0x10u32;
    let tx0 = (total as u32 & 0xffff) | (2 << 23) | (q << 25);
    txwi[..4].copy_from_slice(&tx0.to_le_bytes());
    let fc = u16::from_le_bytes([frame[0], frame[1]]);
    let fc_type = u32::from((fc & 0x000c) >> 2);
    let fc_stype = u32::from((fc & 0x00f0) >> 4);
    let hdr = header_len(frame);
    let tx1 = (1u32 << 31) | ((hdr / 2) as u32) << 11 | (2 << 16);
    txwi[4..8].copy_from_slice(&tx1.to_le_bytes());
    let multicast = frame[4] & 1 != 0;
    let mut tx2 = (fc_type << 4) | fc_stype;
    if multicast {
        tx2 |= 1 << 10;
    }
    let data = fc & 0x000c == 0x0008;
    if !data || multicast {
        tx2 |= 1 << 31;
        tx2 |= 1 << 13;
    }
    txwi[8..12].copy_from_slice(&tx2.to_le_bytes());
    let mut tx3 = 15u32 << 11;
    if multicast {
        tx3 |= 1;
    }
    if tx2 & (1 << 31) != 0 {
        let rate = 0x40u32;
        let tx6 = (1u32 << 2) | (rate << 16);
        txwi[24..28].copy_from_slice(&tx6.to_le_bytes());
        tx3 |= 1 << 28;
    }
    txwi[12..16].copy_from_slice(&tx3.to_le_bytes());
    let tx7 = (fc_type << 20) | (fc_stype << 16);
    txwi[28..32].copy_from_slice(&tx7.to_le_bytes());
    txwi[32..34].copy_from_slice(&0x8000u16.to_le_bytes());
    txwi[40..44].copy_from_slice(&frame_phys.to_le_bytes());
    let len = (frame.len() as u16) | 0x8000;
    txwi[44..46].copy_from_slice(&len.to_le_bytes());
    true
}

fn header_len(frame: &[u8]) -> usize {
    if frame[0] & 0x80 != 0 {
        26
    } else {
        24
    }
}

/// Skip the connac2 RXD groups the way `mt7921_mac_fill_rx` does.
pub fn frame_off(buf: &[u8]) -> Option<usize> {
    if buf.len() < 24 {
        return None;
    }
    let rxd1 = u32::from_le_bytes(buf[4..8].try_into().ok()?);
    let rxd2 = u32::from_le_bytes(buf[8..12].try_into().ok()?);
    let mut words = 6usize;
    if rxd1 & (1 << 14) != 0 {
        words += 4;
    }
    if rxd1 & (1 << 11) != 0 {
        words += 4;
    }
    if rxd1 & (1 << 12) != 0 {
        words += 2;
    }
    if rxd1 & (1 << 13) != 0 {
        words += 2;
        if rxd1 & (1 << 15) != 0 {
            words += 18;
        }
    }
    let pad = ((rxd2 >> 14) & 3) as usize;
    let off = words * 4 + pad * 2;
    if off >= buf.len() {
        None
    } else {
        Some(off)
    }
}

pub fn desc_ctrl(len: u32, last: bool) -> u32 {
    let mut ctrl = len & 0x3fff;
    if last {
        ctrl |= LAST_SEC0;
    }
    ctrl
}

/// Credential page the shell writes and the radio reads.
pub fn write_key(page: &mut [u8], ssid: &[u8], psk: &[u8]) -> bool {
    if page.len() < 0x1000 || !crate::wpa::key_ok(ssid, psk) {
        return false;
    }
    page[..128].fill(0);
    page[0] = 1;
    page[1] = ssid.len() as u8;
    page[2] = psk.len() as u8;
    page[4..4 + ssid.len()].copy_from_slice(ssid);
    page[36..36 + psk.len()].copy_from_slice(psk);
    true
}

pub fn read_key(page: &[u8]) -> Option<(&[u8], &[u8])> {
    if page.len() < 128 || page[0] != 1 {
        return None;
    }
    let ssid_len = usize::from(page[1]);
    let psk_len = usize::from(page[2]);
    if ssid_len > 32 || psk_len > 63 || ssid_len == 0 || psk_len < 8 {
        return None;
    }
    Some((&page[4..4 + ssid_len], &page[36..36 + psk_len]))
}

/// `mt7921_mcu_set_chan_info` body for a 20 MHz channel, one stream.
pub fn write_chan(out: &mut [u8], channel: u8, band: u8) -> bool {
    if out.len() < 76 {
        return false;
    }
    out[..76].fill(0);
    out[0] = channel;
    out[1] = channel;
    out[3] = 1;
    out[4] = 1;
    out[10] = band;
    true
}

pub fn write_eeprom_mode(out: &mut [u8]) {
    out[..4].fill(0);
    out[1] = 1;
}

pub fn write_mac_enable(out: &mut [u8]) {
    out[..4].fill(0);
    out[0] = 1;
}

pub fn write_rts(out: &mut [u8]) {
    out[..12].fill(0);
    out[0] = 1;
    out[4..8].copy_from_slice(&0x92bu32.to_le_bytes());
    out[8..12].copy_from_slice(&2u32.to_le_bytes());
}

pub fn write_efuse_read(out: &mut [u8]) {
    out[..4].fill(0);
}

pub fn efuse_mac(buf: &[u8]) -> Option<[u8; 6]> {
    let data = buf.get(RXD_LEN..)?;
    if data.len() < 8 + 16 {
        return None;
    }
    let mut mac = [0u8; 6];
    mac.copy_from_slice(&data[8 + 4..8 + 10]);
    if mac == [0; 6] || mac == [0xff; 6] {
        None
    } else {
        Some(mac)
    }
}

pub fn wtbl_word(idx: u32) -> u32 {
    (idx & 0x3ff) | (1 << 12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lpctl_is_in_the_fixed_window() {
        assert_eq!(map_reg(CONN_ON_LPCTL), Map::Direct(0xe_0010));
        assert_eq!(map_reg(0xd_4300), Map::Direct(0xd_4300));
        assert_eq!(
            map_reg(WFSYS_RST),
            Map::Remap {
                base: 0x1800,
                window: 0x4_0140
            }
        );
    }

    #[test]
    fn stage_line_names_the_wait() {
        let mut out = [0u8; 32];
        let n = Stage::Wait.paint(&mut out);
        assert_eq!(&out[..n], b"mt7921: wait");
        let n = Stage::PingOk([192, 168, 1, 20]).paint(&mut out);
        assert_eq!(&out[..n], b"ip 192.168.1.20 ping ok");
    }

    #[test]
    fn chip_7922_is_not_the_7961_image() {
        assert_eq!(firmware_kind(0x7961), FirmwareKind::Mt7961);
        assert_eq!(firmware_kind(0x7922), FirmwareKind::Unsupported);
    }

    #[test]
    fn key_page_round_trip() {
        let mut page = [0u8; 0x1000];
        assert!(write_key(&mut page, b"home", b"password"));
        let (ssid, psk) = read_key(&page).unwrap();
        assert_eq!(ssid, b"home");
        assert_eq!(psk, b"password");
        assert!(!write_key(&mut page, b"", b"password"));
    }

    #[test]
    fn patch_header_yields_one_region() {
        let mut image = [0u8; 96 + 64 + 8];
        image[44..48].copy_from_slice(&1u32.to_be_bytes());
        let sec = 96;
        image[sec..sec + 4].copy_from_slice(&2u32.to_be_bytes());
        image[sec + 4..sec + 8].copy_from_slice(&160u32.to_be_bytes());
        image[sec + 12..sec + 16].copy_from_slice(&0x20_0000u32.to_be_bytes());
        image[sec + 16..sec + 20].copy_from_slice(&4u32.to_be_bytes());
        image[160..164].copy_from_slice(&[1, 2, 3, 4]);
        let mut regions = [Region {
            addr: 0,
            off: 0,
            len: 0,
            mode: 0,
            skip: false,
            override_addr: false,
        }; 4];
        let n = patch_regions(&image, &mut regions).unwrap();
        assert_eq!(n, 1);
        assert_eq!(regions[0].addr, 0x20_0000);
        assert_eq!(regions[0].len, 4);
        assert_eq!(download_cmd(regions[0].addr), PATCH_START);
    }

    #[test]
    fn scan_request_carries_the_ssid() {
        let mut buf = [0u8; SCAN_REQ_LEN];
        assert!(write_scan(&mut buf, b"home", 3));
        assert_eq!(&buf[12..16], b"home");
        assert_eq!(buf[2], 1);
    }
}
