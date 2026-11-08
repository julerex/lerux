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

#[must_use]
pub fn firmware_kind(chip: u32) -> FirmwareKind {
    match chip {
        0x7922 | 0x7925 => FirmwareKind::Unsupported,
        _ => FirmwareKind::Mt7961,
    }
}

/// Ring registers read back after a semaphore transmit that did not finish.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TxStop {
    /// DMA index register.
    pub dma: u8,
    /// CPU index register.
    pub cpu: u8,
    /// Ring size register.
    pub size: u16,
    /// Low 16 bits of the ring base register.
    pub base: u16,
    /// Low byte of the DMA config. Bit 0 is transmit enable.
    pub cfg: u8,
    /// Posted slot index.
    pub slot: u8,
    /// Buffer address in descriptor word 0.
    pub buf: u32,
    /// Done bit in descriptor word 1. Clear means the slot is still a command.
    pub done: bool,
}

/// Why the patch semaphore stopped.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sem {
    /// The DMA index never moved. The fields are the ring registers.
    Tx(TxStop),
    /// The descriptor completed and no event matched the sequence.
    Rx,
    /// The event carried this status byte.
    Status(u8),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Own,
    /// Patch semaphore. The tag separates a stuck descriptor, a missing event, and a status byte.
    FwSem(Sem),
    /// Patch header, a patch region setup, or patch-finish was rejected.
    FwPatch,
    /// A firmware chunk did not leave the download ring.
    FwScatter,
    /// RAM trailer, or a RAM region setup, was rejected.
    FwRam,
    /// The start command after the RAM image got no reply.
    FwStart,
    /// Chip id is not the MT7961 pair baked into this image.
    FwChip,
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
        match self {
            Stage::PingOk(addr) => format_report(&WifiReport::PingOk(addr), out),
            Stage::FwSem(why) => {
                let (word, n) = sem_word(why);
                write_line(out, &word[..n])
            }
            stage => write_line(out, stage.word()),
        }
    }

    fn word(self) -> &'static [u8] {
        match self {
            Stage::Own => b"own",
            Stage::FwPatch => b"fw patch",
            Stage::FwScatter => b"fw scatter",
            Stage::FwRam => b"fw ram",
            Stage::FwStart => b"fw start",
            Stage::FwChip => b"fw chip",
            Stage::Mac => b"mac",
            Stage::Wait => b"wait",
            Stage::Scan => b"scan",
            Stage::Auth => b"auth",
            Stage::Assoc => b"assoc",
            Stage::Eapol => b"eapol",
            Stage::Dhcp => b"dhcp",
            Stage::Ping => b"ping",
            Stage::FwSem(_) | Stage::PingOk(_) => b"",
        }
    }
}

fn write_line(out: &mut [u8], word: &[u8]) -> usize {
    let prefix = b"mt7921: ";
    let n = prefix.len() + word.len();
    if out.len() < n {
        return 0;
    }
    out[..prefix.len()].copy_from_slice(prefix);
    out[prefix.len()..n].copy_from_slice(word);
    n
}

fn sem_word(why: Sem) -> ([u8; 32], usize) {
    let mut word = [0u8; 32];
    let n = match why {
        Sem::Tx(stop) => write_tx_stop(&mut word, stop),
        Sem::Rx => {
            word[..9].copy_from_slice(b"fw sem rx");
            9
        }
        Sem::Status(byte) => {
            word[..7].copy_from_slice(b"fw sem ");
            word[7] = hex_nibble(byte >> 4);
            word[8] = hex_nibble(byte);
            9
        }
    };
    (word, n)
}

/// `fw sem d00 c00 s0000 b0000 g00`: DMA index, CPU index, size, ring base, DMA config.
fn write_tx_stop(word: &mut [u8; 32], stop: TxStop) -> usize {
    word[..6].copy_from_slice(b"fw sem");
    word[6] = b' ';
    word[7] = b'd';
    word[8] = hex_nibble(stop.dma >> 4);
    word[9] = hex_nibble(stop.dma);
    word[10] = b' ';
    word[11] = b'c';
    word[12] = hex_nibble(stop.cpu >> 4);
    word[13] = hex_nibble(stop.cpu);
    word[14] = b' ';
    word[15] = b's';
    for (i, shift) in [12, 8, 4, 0].into_iter().enumerate() {
        word[16 + i] = hex_nibble(low_nibble(u32::from(stop.size), shift));
    }
    word[20] = b' ';
    word[21] = b'b';
    for (i, shift) in [12, 8, 4, 0].into_iter().enumerate() {
        word[22 + i] = hex_nibble(low_nibble(u32::from(stop.base), shift));
    }
    word[26] = b' ';
    word[27] = b'g';
    word[28] = hex_nibble(stop.cfg >> 4);
    word[29] = hex_nibble(stop.cfg);
    30
}

/// `slot 00 00000000 d0`: posted index, buffer address, done bit (`d0` clear, `d1` set).
#[must_use]
pub fn write_slot_line(out: &mut [u8], stop: TxStop) -> usize {
    const N: usize = 19;
    if out.len() < N {
        return 0;
    }
    out[..5].copy_from_slice(b"slot ");
    out[5] = hex_nibble(stop.slot >> 4);
    out[6] = hex_nibble(stop.slot);
    out[7] = b' ';
    for (i, shift) in [28, 24, 20, 16, 12, 8, 4, 0].into_iter().enumerate() {
        out[8 + i] = hex_nibble(low_nibble(stop.buf, shift));
    }
    out[16] = b' ';
    out[17] = b'd';
    out[18] = if stop.done { b'1' } else { b'0' };
    N
}

fn low_nibble(value: u32, shift: u32) -> u8 {
    u8::try_from((value >> shift) & 0xf).expect("nibble")
}

fn hex_nibble(nibble: u8) -> u8 {
    let nibble = nibble & 0xf;
    if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + (nibble - 10)
    }
}

/// Where a Linux `mt76_rr` address lands in the 2 MiB BAR.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Map {
    Direct(u32),
    /// `base` is the page (`addr >> 16`). `rr` and `wr` merge it into bits
    /// 15:0 of `MT_HIF_REMAP_L1` and then use `window`.
    Remap {
        base: u32,
        window: u32,
    },
    Unsupported,
}

/// `__mt7921_reg_addr`. The fixed windows are the table in `mt7921/pci.c`.
#[must_use]
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

/// Word Linux writes to [`HIF_REMAP_L1`].
///
/// Bits 31:16 stay as the register already holds them. Bits 15:0 are the
/// page from [`map_reg`]. On the 8 November 2026 Swift capture the chip-id
/// step read `0x1845_1844` and wrote `0x1845_7001`.
#[must_use]
pub fn remap_l1(current: u32, page: u32) -> u32 {
    (current & !0xffff) | (page & 0xffff)
}
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
pub const BUSY_ENA: u32 = WFDMA0 + 0x13c;
pub const MCU_CMD: u32 = WFDMA0 + 0x1f0;
pub const HOST_INT_STA: u32 = WFDMA0 + 0x200;
pub const RST_DRX_PTR: u32 = WFDMA0 + 0x280;
pub const HIF_MISC: u32 = 0xd_7044;
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

#[must_use]
pub fn ring_regs(base: u32, idx: u32) -> u32 {
    base + idx * RING_STRIDE
}

/// Registers the dump rows read, in row order: three rows of seven.
/// The names are the `dump::write_named_row` labels. The first row is
/// ownership and reset state, the second the WFDMA control block, the third
/// the reset, shaper, and mode registers.
pub const DUMP_REGS: [(&[u8], u32); 21] = [
    (b"lpctl", CONN_ON_LPCTL),
    (b"misc", CONN_ON_MISC),
    (b"wfsys", WFSYS_RST),
    (b"lpcr", TOP_LPCR),
    (b"chip", HW_CHIPID),
    (b"rev", HW_REV),
    (b"mint", PCIE_MAC_INT),
    (b"mpm", PCIE_MAC_PM),
    (b"iena", HOST_INT_ENA),
    (b"ista", HOST_INT_STA),
    (b"glo", GLO_CFG),
    (b"ext0", GLO_CFG_EXT0),
    (b"dtx", RST_DTX_PTR),
    (b"drx", RST_DRX_PTR),
    (b"mcmd", MCU_CMD),
    (b"m2h", MCU2HOST),
    (b"rst", WFDMA_RST),
    (b"busy", BUSY_ENA),
    (b"shdl", DMASHDL_SW),
    (b"dummy", DUMMY_CR),
    (b"swdef", SWDEF_MODE),
];

/// Ring register blocks the dump rows read: transmit 0, 16, 17, then receive
/// event, data, and the post-download event ring.
pub const DUMP_RINGS: [(&[u8], u32); 6] = [
    (b"t00", TX_RING_BASE),
    (b"t16", TX_RING_BASE + 16 * RING_STRIDE),
    (b"t17", TX_RING_BASE + 17 * RING_STRIDE),
    (b"r00", RX_EVENT_BASE),
    (b"r01", RX_DATA_BASE),
    (b"r02", RX_WA_BASE),
];

/// First register of each prefetch row: transmit rings 0 to 11, transmit
/// rings 12 to 17 and the words after them, then receive rings 0 to 7.
pub const DUMP_PREFETCH: [u32; 3] = [WFDMA0 + 0x600, WFDMA0 + 0x630, WFDMA0 + 0x680];

/// `t17 01f62000 00000100 00000001 00000000`: one ring's base, size, CPU
/// index, and DMA index. Three of these fill one row.
#[must_use]
pub fn write_ring(out: &mut [u8], name: &[u8], regs: [u32; 4]) -> usize {
    const N: usize = 3 + 4 * 9;
    if out.len() < N || name.len() < 3 {
        return 0;
    }
    out[..3].copy_from_slice(&name[..3]);
    for (i, value) in regs.iter().enumerate() {
        let at = 3 + i * 9;
        out[at] = b' ';
        for (byte, shift) in out[at + 1..at + 9].iter_mut().zip((0..8).rev()) {
            *byte = hex_nibble(low_nibble(*value, shift * 4));
        }
    }
    N
}

/// `PREFETCH(base, depth)` values `mt792x_dma_prefetch` writes for MT7921.
#[must_use]
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

#[must_use]
pub fn ext_cmd(id: u32) -> u32 {
    EXT_CID | (id << 8)
}

#[must_use]
pub fn ext_query(id: u32) -> u32 {
    ext_cmd(id) | QUERY
}

#[must_use]
pub fn ce_cmd(id: u32) -> u32 {
    id | CE
}

/// `mt76_connac_mcu_init_download` command choice for MT7921.
#[must_use]
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

fn low_u16(value: usize) -> u16 {
    u16::try_from(value & 0xffff).expect("low 16 bits")
}

fn fit_u32(value: usize) -> u32 {
    u32::try_from(value).expect("length fits in u32")
}

fn fit_u16(value: usize) -> u16 {
    u16::try_from(value).expect("length fits in u16")
}

fn fit_u8(value: usize) -> u8 {
    u8::try_from(value).expect("length fits in a byte")
}

/// Hardware TXD plus the connac2 MCU header. Payload follows at [`MCU_TXD_LEN`].
pub fn write_mcu_txd(buf: &mut [u8], cmd: u32, seq: u8, payload: usize) -> bool {
    if buf.len() < MCU_TXD_LEN {
        return false;
    }
    buf[..MCU_TXD_LEN].fill(0);
    let total = MCU_TXD_LEN + payload;
    let tx0 = u32::from(low_u16(total)) | (2 << 23) | (0x20 << 25);
    let tx1: u32 = 1 << 31 | (1 << 16);
    buf[..4].copy_from_slice(&tx0.to_le_bytes());
    buf[4..8].copy_from_slice(&tx1.to_le_bytes());
    // `mt76_connac2_mcu_fill_message` counts every byte after the eight-dword TXD.
    let body = low_u16((MCU_TXD_LEN - TXD_LEN) + payload);
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
        buf[38] = u8::from(cmd & QUERY == 0);
        buf[43] = u8::from(ext != 0);
    } else {
        buf[38] = 3;
    }
    buf[39] = seq;
    buf[41] = ext;
    buf[42] = 0;
    true
}

#[must_use]
pub fn rxd_seq(buf: &[u8]) -> Option<u8> {
    buf.get(29).copied()
}

#[must_use]
pub fn rxd_eid(buf: &[u8]) -> Option<u8> {
    buf.get(28).copied()
}

/// Status byte `mt7921_mcu_parse_response` reads for the patch semaphore.
#[must_use]
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

/// `(band, channel)` pairs in `mt76_connac_hw_scan_req`. Band 1 is 2.4 GHz.
const SCAN_CHANNELS: &[(u8, u8)] = &[
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

/// Active scan. Layout is `mt76_connac_hw_scan_req` in Linux v6.8.
///
/// An empty `ssid` is the wildcard survey: `ssid_type` bit 0, `ssids_num` 0.
/// A name is the directed scan association uses.
pub fn write_scan(out: &mut [u8], ssid: &[u8], seq: u8) -> bool {
    if out.len() < SCAN_REQ_LEN || ssid.len() > 32 {
        return false;
    }
    out[..SCAN_REQ_LEN].fill(0);
    out[0] = seq & 0x7f;
    out[2] = 1;
    if ssid.is_empty() {
        out[3] = 1;
    } else {
        out[3] = 1 << 2;
        out[4] = 1;
        out[8..12].copy_from_slice(&fit_u32(ssid.len()).to_le_bytes());
        out[12..12 + ssid.len()].copy_from_slice(ssid);
    }
    out[5] = 2;
    out[6] = 1 << 5;
    out[7] = 1;
    let base = 8 + 4 * SCAN_SSID + 6;
    out[base] = 4;
    out[base + 1] = fit_u8(SCAN_CHANNELS.len());
    for (i, (band, ch)) in SCAN_CHANNELS.iter().enumerate() {
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
    write_txd(txwi, frame);
    txwi[32..34].copy_from_slice(&0x8000u16.to_le_bytes());
    txwi[40..44].copy_from_slice(&frame_phys.to_le_bytes());
    let len = fit_u16(frame.len()) | 0x8000;
    txwi[44..46].copy_from_slice(&len.to_le_bytes());
    true
}

/// TXD words before the TXP buffer pointer. `frame` is at least 24 bytes.
fn write_txd(txwi: &mut [u8], frame: &[u8]) {
    let total = frame.len() + TXD_LEN;
    let q = 0x10u32;
    let tx0 = u32::from(low_u16(total)) | (2 << 23) | (q << 25);
    txwi[..4].copy_from_slice(&tx0.to_le_bytes());
    let fc = u16::from_le_bytes([frame[0], frame[1]]);
    let fc_kind = u32::from((fc & 0x000c) >> 2);
    let subtype = u32::from((fc & 0x00f0) >> 4);
    let hdr = header_len(frame);
    let words = u32::try_from(hdr / 2).expect("header words");
    let tx1 = (1u32 << 31) | (words << 11) | (2 << 16);
    txwi[4..8].copy_from_slice(&tx1.to_le_bytes());
    let multicast = frame[4] & 1 != 0;
    let mut tx2 = (fc_kind << 4) | subtype;
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
    let tx7 = (fc_kind << 20) | (subtype << 16);
    txwi[28..32].copy_from_slice(&tx7.to_le_bytes());
}

fn header_len(frame: &[u8]) -> usize {
    if frame[0] & 0x80 != 0 {
        26
    } else {
        24
    }
}

/// Skip the connac2 RXD groups the way `mt7921_mac_fill_rx` does.
#[must_use]
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

#[must_use]
/// `MT_DMA_CTL_SD_LEN0` is bits 29:16. The low half is the unused second segment.
pub fn desc_ctrl(len: u32, last: bool) -> u32 {
    let mut ctrl = (len & 0x3fff) << 16;
    if last {
        ctrl |= LAST_SEC0;
    }
    ctrl
}

#[must_use]
pub fn desc_len(ctrl: u32) -> usize {
    ((ctrl >> 16) & 0x3fff) as usize
}

/// Credential page the shell writes and the radio reads.
pub fn write_key(page: &mut [u8], ssid: &[u8], psk: &[u8]) -> bool {
    if page.len() < 0x1000 || !crate::wpa::key_ok(ssid, psk) {
        return false;
    }
    page[..128].fill(0);
    page[0] = 1;
    page[1] = fit_u8(ssid.len());
    page[2] = fit_u8(psk.len());
    page[4..4 + ssid.len()].copy_from_slice(ssid);
    page[36..36 + psk.len()].copy_from_slice(psk);
    true
}

#[must_use]
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

/// Names kept from one survey. The shell prints one line each.
pub const SCAN_MAX: usize = 16;

const PAGE_SURVEY: u8 = 2;
const PAGE_LIST: u8 = 3;
const SURVEY_STRIDE: usize = 33;

fn survey_span() -> usize {
    4 + SCAN_MAX * SURVEY_STRIDE
}

/// The shell asks which names the radio can hear. The radio replaces this tag.
pub fn write_survey(page: &mut [u8]) -> bool {
    if page.len() < survey_span() {
        return false;
    }
    page[..survey_span()].fill(0);
    page[0] = PAGE_SURVEY;
    true
}

#[must_use]
pub fn survey_requested(page: &[u8]) -> bool {
    !page.is_empty() && page[0] == PAGE_SURVEY
}

/// Store the heard names in byte order. Tag becomes the list the shell reads.
pub fn write_survey_list(page: &mut [u8], names: &[&[u8]]) -> bool {
    if page.len() < survey_span() || names.len() > SCAN_MAX {
        return false;
    }
    for name in names {
        if name.is_empty() || name.len() > 32 {
            return false;
        }
    }
    let mut owned = [[0u8; 32]; SCAN_MAX];
    let mut lens = [0u8; SCAN_MAX];
    for (i, name) in names.iter().enumerate() {
        owned[i][..name.len()].copy_from_slice(name);
        lens[i] = fit_u8(name.len());
    }
    sort_names(&mut owned, &mut lens, names.len());
    page[..survey_span()].fill(0);
    page[0] = PAGE_LIST;
    page[1] = fit_u8(names.len());
    for i in 0..names.len() {
        let at = 4 + i * SURVEY_STRIDE;
        let n = usize::from(lens[i]);
        page[at] = lens[i];
        page[at + 1..at + 1 + n].copy_from_slice(&owned[i][..n]);
    }
    true
}

#[must_use]
pub fn read_survey_count(page: &[u8]) -> Option<usize> {
    let n = survey_len(page)?;
    for i in 0..n {
        survey_name(page, i)?;
    }
    Some(n)
}

#[must_use]
pub fn read_survey_ssid(page: &[u8], index: usize) -> Option<&[u8]> {
    let n = read_survey_count(page)?;
    if index >= n {
        return None;
    }
    survey_name(page, index)
}

/// A listed name, or the 1-based index of one. A line that is neither is `None`.
#[must_use]
pub fn survey_choice<'a>(page: &'a [u8], line: &[u8]) -> Option<&'a [u8]> {
    let n = read_survey_count(page)?;
    for i in 0..n {
        let name = survey_name(page, i)?;
        if name == line {
            return Some(name);
        }
    }
    let index = menu_index(line)?;
    if index >= n {
        return None;
    }
    survey_name(page, index)
}

fn survey_len(page: &[u8]) -> Option<usize> {
    if page.len() < survey_span() || page[0] != PAGE_LIST {
        return None;
    }
    let n = usize::from(page[1]);
    if n > SCAN_MAX {
        None
    } else {
        Some(n)
    }
}

fn survey_name(page: &[u8], index: usize) -> Option<&[u8]> {
    let at = 4 + index * SURVEY_STRIDE;
    let len = usize::from(*page.get(at)?);
    if len == 0 || len > 32 || page.len() < at + 1 + len {
        return None;
    }
    Some(&page[at + 1..at + 1 + len])
}

fn menu_index(line: &[u8]) -> Option<usize> {
    if line.is_empty() || line.len() > 2 || line[0] == b'0' {
        return None;
    }
    let mut value = 0usize;
    for byte in line {
        let digit = byte.wrapping_sub(b'0');
        if digit > 9 {
            return None;
        }
        value = value * 10 + usize::from(digit);
    }
    value.checked_sub(1)
}

fn sort_names(names: &mut [[u8; 32]], lens: &mut [u8], count: usize) {
    for i in 1..count {
        let mut j = i;
        while j > 0 && name_before(&names[j], lens[j], &names[j - 1], lens[j - 1]) {
            names.swap(j, j - 1);
            lens.swap(j, j - 1);
            j -= 1;
        }
    }
}

fn name_before(left: &[u8; 32], left_len: u8, right: &[u8; 32], right_len: u8) -> bool {
    left[..usize::from(left_len)] < right[..usize::from(right_len)]
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

#[must_use]
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

#[must_use]
pub fn wtbl_word(idx: u32) -> u32 {
    (idx & 0x3ff) | (1 << 12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remap_l1_keeps_the_high_half() {
        // Chip id, wfsys, and driver-own, in the order the trace wrote them.
        assert_eq!(remap_l1(0x1845_1844, 0x7001), 0x1845_7001);
        assert_eq!(remap_l1(0x1845_7001, 0x1800), 0x1845_1800);
        assert_eq!(remap_l1(0x1845_1800, 0x1806), 0x1845_1806);
    }

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
        let mut out = [0u8; 40];
        let n = Stage::Wait.paint(&mut out);
        assert_eq!(&out[..n], b"mt7921: wait");
        let stop = TxStop {
            dma: 0,
            cpu: 1,
            size: 0x0100,
            base: 0x2000,
            cfg: 0x45,
            slot: 0,
            buf: 0x01f6_6000,
            done: false,
        };
        let n = Stage::FwSem(Sem::Tx(stop)).paint(&mut out);
        assert_eq!(&out[..n], b"mt7921: fw sem d00 c01 s0100 b2000 g45");
        let n = write_slot_line(&mut out, stop);
        assert_eq!(&out[..n], b"slot 00 01f66000 d0");
        let n = write_slot_line(
            &mut out,
            TxStop {
                buf: 0,
                done: true,
                ..stop
            },
        );
        assert_eq!(&out[..n], b"slot 00 00000000 d1");
        let n = Stage::FwSem(Sem::Rx).paint(&mut out);
        assert_eq!(&out[..n], b"mt7921: fw sem rx");
        let n = Stage::FwSem(Sem::Status(0x2a)).paint(&mut out);
        assert_eq!(&out[..n], b"mt7921: fw sem 2a");
        let n = Stage::FwScatter.paint(&mut out);
        assert_eq!(&out[..n], b"mt7921: fw scatter");
        let n = Stage::PingOk([192, 168, 1, 20]).paint(&mut out);
        assert_eq!(&out[..n], b"ip 192.168.1.20 ping ok");
    }

    #[test]
    fn ring_line_holds_four_registers() {
        let mut out = [0u8; 40];
        let n = write_ring(&mut out, b"t17", [0x01f6_2000, 0x100, 1, 0]);
        assert_eq!(&out[..n], b"t17 01f62000 00000100 00000001 00000000");
        assert_eq!(n * 3 + 2, 119);
        assert_eq!(DUMP_REGS.len(), 21);
        assert!(DUMP_REGS.iter().all(|(name, _)| name.len() <= 5));
        assert_eq!(DUMP_RINGS[2].1, 0xd_4410);
        assert_eq!(DUMP_PREFETCH, [0xd_4600, 0xd_4630, 0xd_4680]);
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
    fn dma_length_sits_in_the_first_segment() {
        let ctrl = desc_ctrl(36, true);
        assert_eq!(ctrl & 0x3fff, 0);
        assert_eq!(desc_len(ctrl), 36);
        assert_eq!(ctrl & LAST_SEC0, LAST_SEC0);
        assert_eq!(ctrl & DMA_DONE, 0);
    }

    #[test]
    fn mcu_header_len_includes_itself() {
        let mut buf = [0u8; MCU_TXD_LEN];
        assert!(write_mcu_txd(&mut buf, PATCH_SEM, 1, 4));
        let len = u16::from_le_bytes([buf[32], buf[33]]);
        assert_eq!(len, u16::try_from(MCU_TXD_LEN - TXD_LEN + 4).unwrap());
        assert_eq!(buf[36], 0x10);
        assert_eq!(buf[39], 1);
    }

    #[test]
    fn scan_request_carries_the_ssid() {
        let mut buf = [0u8; SCAN_REQ_LEN];
        assert!(write_scan(&mut buf, b"home", 3));
        assert_eq!(&buf[12..16], b"home");
        assert_eq!(buf[2], 1);
        assert_eq!(buf[3], 1 << 2);
        assert_eq!(buf[4], 1);
        assert!(write_scan(&mut buf, b"", 3));
        assert_eq!(buf[2], 1);
        assert_eq!(buf[3], 1);
        assert_eq!(buf[4], 0);
        assert_eq!(buf[12], 0);
    }

    #[test]
    fn survey_page_lists_names_in_byte_order() {
        let mut page = [0u8; 0x1000];
        assert!(write_survey(&mut page));
        assert!(survey_requested(&page));
        assert!(read_survey_count(&page).is_none());
        assert!(write_survey_list(&mut page, &[b"zeta", b"alpha", b"2"]));
        assert_eq!(read_survey_count(&page), Some(3));
        assert_eq!(read_survey_ssid(&page, 0), Some(&b"2"[..]));
        assert_eq!(read_survey_ssid(&page, 1), Some(&b"alpha"[..]));
        assert_eq!(read_survey_ssid(&page, 2), Some(&b"zeta"[..]));
        assert_eq!(survey_choice(&page, b"2"), Some(&b"2"[..]));
        assert_eq!(survey_choice(&page, b"1"), Some(&b"2"[..]));
        assert_eq!(survey_choice(&page, b"alpha"), Some(&b"alpha"[..]));
        assert!(survey_choice(&page, b"02").is_none());
        assert!(survey_choice(&page, b"other").is_none());
        assert!(write_survey_list(&mut page, &[]));
        assert_eq!(read_survey_count(&page), Some(0));
        assert!(write_key(&mut page, b"alpha", b"password"));
        assert!(read_survey_count(&page).is_none());
    }
}
