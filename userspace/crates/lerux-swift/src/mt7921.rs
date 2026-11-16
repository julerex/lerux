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
    /// The open-system authentication frame did not leave the transmit ring.
    ///
    /// The boot that painted a single `auth` covered this return and [`Stage::AuthRx`].
    AuthTx,
    /// The authentication frame was posted and no matching reply arrived.
    AuthRx,
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
            Stage::AuthTx => b"auth tx",
            Stage::AuthRx => b"auth rx",
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
/// `sizeof(struct mt76_connac2_mcu_uni_txd)`.
pub const UNI_TXD_LEN: usize = 48;
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
const UNI: u32 = 1 << 17;
const QUERY: u32 = 1 << 16;
/// `MCU_CMD_UNI_EXT_ACK`: reply, unified command, set.
const UNI_EXT_ACK: u8 = 0x07;
/// `CONNECTION_INFRA_STA`.
const CONN_INFRA_STA: u32 = 1 | (1 << 16);
const CONN_INFRA_AP: u32 = 2 | (1 << 16);

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

/// `MCU_UNI_CMD`. The header is [`write_uni_txd`], shorter than [`write_mcu_txd`].
#[must_use]
pub fn uni_cmd(id: u32) -> u32 {
    id | UNI
}

#[must_use]
pub fn is_uni_cmd(cmd: u32) -> bool {
    cmd & UNI != 0
}

/// Bytes before the payload. A unified command uses the 48-byte header.
#[must_use]
pub fn mcu_header_len(cmd: u32) -> usize {
    if is_uni_cmd(cmd) {
        UNI_TXD_LEN
    } else {
        MCU_TXD_LEN
    }
}

/// Header for `cmd`. The payload length is the bytes that follow the header.
#[must_use]
pub fn write_mcu_header(buf: &mut [u8], cmd: u32, seq: u8, payload: usize) -> bool {
    if is_uni_cmd(cmd) {
        write_uni_txd(buf, cmd, seq, payload)
    } else {
        write_mcu_txd(buf, cmd, seq, payload)
    }
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

/// Unified MCU header. Payload follows at [`UNI_TXD_LEN`].
///
/// `len` counts every byte after the eight-dword TXD, including this header.
pub fn write_uni_txd(buf: &mut [u8], cmd: u32, seq: u8, payload: usize) -> bool {
    if buf.len() < UNI_TXD_LEN {
        return false;
    }
    buf[..UNI_TXD_LEN].fill(0);
    let total = UNI_TXD_LEN + payload;
    let tx0 = u32::from(low_u16(total)) | (2 << 23) | (0x20 << 25);
    let tx1: u32 = 1 << 31 | (1 << 16);
    buf[..4].copy_from_slice(&tx0.to_le_bytes());
    buf[4..8].copy_from_slice(&tx1.to_le_bytes());
    let body = low_u16(total - TXD_LEN);
    buf[32..34].copy_from_slice(&body.to_le_bytes());
    buf[34..36].copy_from_slice(&u16::from((cmd & 0xff) as u8).to_le_bytes());
    buf[37] = 0xa0;
    buf[39] = seq;
    buf[43] = UNI_EXT_ACK;
    true
}

/// Bytes in [`write_dev_info`].
pub const DEV_INFO_LEN: usize = 16;
/// Bytes in [`write_bss_info`].
pub const BSS_INFO_LEN: usize = 36;
/// Bytes in [`write_bss_connect`]: the four-byte header, the basic TLV, and the QBSS TLV.
pub const BSS_CONNECT_LEN: usize = 44;
/// Bytes in [`write_bss_rlm`].
pub const BSS_RLM_LEN: usize = 20;
/// Bytes in [`write_rx_filter_enable`].
pub const RX_FILTER_LEN: usize = 68;

/// `DEV_INFO_UPDATE` for omac 0. `mac` is the station address.
pub fn write_dev_info(out: &mut [u8], mac: &[u8; 6]) -> bool {
    if out.len() < DEV_INFO_LEN {
        return false;
    }
    out[..DEV_INFO_LEN].fill(0);
    out[6..8].copy_from_slice(&12u16.to_le_bytes());
    out[8] = 1;
    out[10..16].copy_from_slice(mac);
    true
}

/// `BSS_INFO_UPDATE` basic TLV for an unassociated station at `bss_idx` 0.
///
/// The wlan index stays 0. `mt7921_add_interface` assigns the table index
/// after this command returns, so the firmware sees 0.
pub fn write_bss_info(out: &mut [u8]) -> bool {
    if out.len() < BSS_INFO_LEN {
        return false;
    }
    out[..BSS_INFO_LEN].fill(0);
    out[6..8].copy_from_slice(&32u16.to_le_bytes());
    out[8] = 1;
    out[12..16].copy_from_slice(&CONN_INFRA_STA.to_le_bytes());
    out[16] = 1;
    true
}

/// `BSS_INFO_UPDATE` from `mt76_connac_mcu_uni_add_bss` once the station associates.
///
/// `conn_state` is 0 while the station is connecting. The BSSID is the access
/// point. `bmc_tx_wlan_idx` and `sta_idx` are the interface entry
/// [`WTBL_RESERVED`]. Beacon interval, DTIM, and the qos byte stay 0 because
/// those beacon fields are not parsed. `phymode` is A on 5 GHz and B plus G on
/// 2.4 GHz. `nonht_basic_phy` is OFDM on 5 GHz and HR/DSSS plus ERP on 2.4 GHz.
pub fn write_bss_connect(out: &mut [u8], bssid: &[u8; 6], band5: bool) -> bool {
    if out.len() < BSS_CONNECT_LEN {
        return false;
    }
    out[..BSS_CONNECT_LEN].fill(0);
    out[6..8].copy_from_slice(&32u16.to_le_bytes());
    out[8] = 1;
    out[12..16].copy_from_slice(&CONN_INFRA_STA.to_le_bytes());
    out[18..24].copy_from_slice(bssid);
    out[24..26].copy_from_slice(&VIF_WTBL.to_le_bytes());
    let phymode = if band5 {
        PHY_MODE_A
    } else {
        PHY_MODE_B | PHY_MODE_G
    };
    let legacy = if band5 {
        PHY_OFDM
    } else {
        PHY_HR_DSSS | PHY_ERP
    };
    out[29] = phymode;
    out[30..32].copy_from_slice(&VIF_WTBL.to_le_bytes());
    out[32..34].copy_from_slice(&u16::from(legacy).to_le_bytes());
    out[36..38].copy_from_slice(&15u16.to_le_bytes());
    out[38..40].copy_from_slice(&8u16.to_le_bytes());
    true
}

/// Channel context `mt76_connac_mcu_uni_set_chctx` sends at the end of the
/// association BSS update.
///
/// The width is 20 MHz, the same width [`write_chan`] programs, so `bw` is 0,
/// `ht_op_info` is 0, and the center channel equals the control channel.
/// `tx_streams` is the number of set bits in `antenna`. `rx_streams` is that
/// mask: Linux writes the popcount into the transmit field and the chain mask
/// into the receive field, and this device programs both masks with one value.
pub fn write_bss_rlm(out: &mut [u8], channel: u8, band: u8, antenna: u8) -> bool {
    if out.len() < BSS_RLM_LEN {
        return false;
    }
    out[..BSS_RLM_LEN].fill(0);
    out[4..6].copy_from_slice(&2u16.to_le_bytes());
    out[6..8].copy_from_slice(&16u16.to_le_bytes());
    out[8] = channel;
    out[9] = channel;
    out[12] = u8::try_from(antenna.count_ones()).unwrap_or(0);
    out[13] = antenna;
    out[14] = 1;
    out[17] = band;
    true
}

const STA_REQ_HDR: usize = 8;
const STA_BASIC: usize = 20;
const STA_PHY: usize = 12;
const STA_RA: usize = 16;
const STA_STATE: usize = 12;
const STA_WTBL_TLV: usize = 4;
const WTBL_HDR: usize = 8;
const WTBL_GENERIC_LEN: usize = 20;
const WTBL_RX_LEN: usize = 12;
const WTBL_HDR_TRANS_LEN: usize = 8;
const WTBL_SMPS_LEN: usize = 8;
const STA_WTBL: usize =
    STA_WTBL_TLV + WTBL_HDR + WTBL_GENERIC_LEN + WTBL_RX_LEN + WTBL_HDR_TRANS_LEN + WTBL_SMPS_LEN;

/// Bytes in [`write_sta_rec`].
pub const STA_REC_LEN: usize = STA_REQ_HDR + STA_BASIC + STA_PHY + STA_RA + STA_STATE + STA_WTBL;

/// `STA_REC_UPDATE` for the access point.
///
/// `mt7921_mac_sta_add` calls this with `assoc` false, before authentication.
/// Index 0 is the global entry (`mt792x_init_wcid`), so the peer is [`PEER_WTBL`].
/// Extra info is version plus new, the state byte is none, and `aid` is 0.
/// `mt7921_mac_sta_event` calls it again with `assoc` true after the association
/// response. Extra info is version only, because `newly` is false for
/// `MT76_STA_INFO_STATE_ASSOC`. The state byte is 2, and the generic
/// wireless-table TLV stores `aid` as `partial_aid`. The basic TLV stays
/// port-secure because the station is enabled. HT, VHT, and HE stay absent:
/// `sta_tlv` adds them when the peer capabilities are set, and those
/// capabilities are not parsed here.
pub fn write_sta_rec(
    out: &mut [u8],
    bssid: &[u8; 6],
    band5: bool,
    assoc: bool,
    aid: u16,
) -> Option<usize> {
    if out.len() < STA_REC_LEN {
        return None;
    }
    out[..STA_REC_LEN].fill(0);
    let mut at = 0;
    let (extra, state): (u16, u8) = if assoc { (1, 2) } else { (3, 0) };
    put_sta_header(out, &mut at);
    put_sta_basic(out, &mut at, *bssid, aid, extra);
    put_sta_rates(out, &mut at, band5, state);
    put_sta_wtbl(out, &mut at, *bssid, aid);
    (at == STA_REC_LEN).then_some(at)
}

fn put_sta_header(out: &mut [u8], at: &mut usize) {
    put_u8(out, at, 0);
    put_u8(out, at, PEER_INDEX);
    put_u16(out, at, 5);
    put_u8(out, at, 1);
    put_bytes(out, at, &[0; 3]);
}

fn put_sta_basic(out: &mut [u8], at: &mut usize, bssid: [u8; 6], aid: u16, extra: u16) {
    put_u16(out, at, 0);
    put_u16(out, at, fit_u16(STA_BASIC));
    put_u32(out, at, CONN_INFRA_AP);
    put_u8(out, at, 2);
    put_u8(out, at, 0);
    put_u16(out, at, aid);
    put_bytes(out, at, &bssid);
    put_u16(out, at, extra);
}

fn put_sta_rates(out: &mut [u8], at: &mut usize, band5: bool, state: u8) {
    let phy = if band5 {
        PHY_OFDM
    } else {
        PHY_HR_DSSS | PHY_ERP
    };
    let basic = if band5 { 0x0015u16 } else { 0x000f };
    put_u16(out, at, 0x15);
    put_u16(out, at, fit_u16(STA_PHY));
    put_u16(out, at, basic);
    put_u8(out, at, phy);
    put_bytes(out, at, &[0; 5]);

    let legacy = if band5 {
        0xffu16 << 6
    } else {
        (0xffu16 << 6) | 0xf
    };
    put_u16(out, at, 1);
    put_u16(out, at, fit_u16(STA_RA));
    put_u16(out, at, legacy);
    put_bytes(out, at, &[0; 10]);

    put_u16(out, at, 7);
    put_u16(out, at, fit_u16(STA_STATE));
    put_u32(out, at, 0);
    put_u8(out, at, state);
    put_bytes(out, at, &[0; 3]);
}

fn put_sta_wtbl(out: &mut [u8], at: &mut usize, bssid: [u8; 6], aid: u16) {
    put_u16(out, at, 13);
    put_u16(out, at, fit_u16(STA_WTBL));
    put_u8(out, at, PEER_INDEX);
    put_u8(out, at, 1);
    put_u16(out, at, 4);
    put_bytes(out, at, &[0; 4]);

    put_u16(out, at, 0);
    put_u16(out, at, fit_u16(WTBL_GENERIC_LEN));
    put_bytes(out, at, &bssid);
    put_bytes(out, at, &[0; 6]);
    put_u16(out, at, aid);
    put_bytes(out, at, &[0; 2]);

    put_u16(out, at, 1);
    put_u16(out, at, fit_u16(WTBL_RX_LEN));
    put_u8(out, at, 0);
    put_bytes(out, at, &[1, 1, 1]);
    put_bytes(out, at, &[0; 4]);

    put_u16(out, at, 6);
    put_u16(out, at, fit_u16(WTBL_HDR_TRANS_LEN));
    put_bytes(out, at, &[1, 0, 1, 0]);

    put_u16(out, at, 13);
    put_u16(out, at, fit_u16(WTBL_SMPS_LEN));
    put_bytes(out, at, &[0; 4]);
}

const PHY_HR_DSSS: u8 = 1;
const PHY_ERP: u8 = 2;
const PHY_OFDM: u8 = 8;
const PHY_MODE_A: u8 = 1;
const PHY_MODE_B: u8 = 2;
const PHY_MODE_G: u8 = 4;

fn put_u8(out: &mut [u8], at: &mut usize, value: u8) {
    out[*at] = value;
    *at += 1;
}

fn put_u16(out: &mut [u8], at: &mut usize, value: u16) {
    out[*at..*at + 2].copy_from_slice(&value.to_le_bytes());
    *at += 2;
}

fn put_u32(out: &mut [u8], at: &mut usize, value: u32) {
    out[*at..*at + 4].copy_from_slice(&value.to_le_bytes());
    *at += 4;
}

fn put_bytes(out: &mut [u8], at: &mut usize, bytes: &[u8]) {
    out[*at..*at + bytes.len()].copy_from_slice(bytes);
    *at += bytes.len();
}

/// `SET_RX_FILTER` mode 1, the filter word `mt7921_configure_filter` sends
/// before a scan. Mode 1 replaces the filter. `BIT(31)` is `FILTER_ENABLE`.
/// Linux adds other-BSS, FCS-fail, and control only when mac80211 asks.
pub fn write_rx_filter_enable(out: &mut [u8]) -> bool {
    if out.len() < RX_FILTER_LEN {
        return false;
    }
    out[..RX_FILTER_LEN].fill(0);
    out[4] = 1;
    out[8..12].copy_from_slice(&(1u32 << 31).to_le_bytes());
    true
}

/// `SET_RX_FILTER` bitmap clear of `MT_WF_RFCR_DROP_OTHER_BEACON`.
///
/// Mode 2 updates one bit. `bit_op` bit 1 clears it. This is the command
/// `mt7921_mcu_set_beacon_filter` sends when the station is not associated.
/// The firmware can count those beacons and still drop them before the host.
pub fn write_clear_other_beacon(out: &mut [u8]) -> bool {
    if out.len() < RX_FILTER_LEN {
        return false;
    }
    out[..RX_FILTER_LEN].fill(0);
    out[4] = 2;
    out[12..16].copy_from_slice(&(1u32 << 11).to_le_bytes());
    out[16] = 1 << 1;
    true
}

/// True when a unified command reply is success for `cid`.
///
/// The event follows the connac2 RXD: command id, then a little-endian status.
/// Status 0 is success.
#[must_use]
pub fn uni_event_ok(buf: &[u8], cid: u8) -> bool {
    let Some(event) = buf.get(RXD_LEN..RXD_LEN + 8) else {
        return false;
    };
    event[0] == cid && event[4..8] == [0, 0, 0, 0]
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

/// Completed channels and `beacon_scan_num` from `mt76_connac_hw_scan_done`.
pub struct ScanDone {
    pub channels: u8,
    pub beacons: u32,
}

/// `MCU_EVENT_SCAN_DONE`. The payload follows the connac2 RXD.
///
/// A short event still counts: the caller has to stop, and missing counts stay 0.
#[must_use]
pub fn read_scan_done(buf: &[u8]) -> Option<ScanDone> {
    let word = buf.get(..4)?;
    let kind = u32::from_le_bytes(word.try_into().ok()?) >> 27;
    if kind != 7 || buf.get(28) != Some(&SCAN_DONE) {
        return None;
    }
    let channels = buf.get(RXD_LEN + 4).copied().unwrap_or(0);
    let beacons = buf
        .get(RXD_LEN + 8..RXD_LEN + 12)
        .and_then(|bytes| bytes.try_into().ok())
        .map_or(0, u32::from_le_bytes);
    Some(ScanDone { channels, beacons })
}

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

/// Active scan. Layout is `mt76_connac_hw_scan_req`.
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
    // `ssid_type_ext` bit 0 means the named SSID only. A wildcard survey leaves it 0.
    if !ssid.is_empty() {
        let ext_flag = bssid_at + 12 + 63;
        if ext_flag < SCAN_REQ_LEN {
            out[ext_flag] = 1;
        }
    }
    true
}

/// Bytes in [`write_channel_domain`]. One header, then one record per scan channel.
pub const CHANNEL_DOMAIN_LEN: usize = 12 + SCAN_CHANNELS.len() * 8;

const _: () = assert!(SCAN_CHANNELS.len() <= 32);

/// `mt76_connac_mcu_set_channel_domain` for the channels [`write_scan`] names.
///
/// `alpha2` is `00`, the world domain Linux keeps until a country hint.
/// 2.4 GHz is 20/40 MHz. 5 GHz is 20/40/80/160 MHz. Channel flags stay 0.
#[must_use]
pub fn write_channel_domain(out: &mut [u8]) -> Option<usize> {
    let n2 = SCAN_CHANNELS.iter().filter(|(band, _)| *band == 1).count();
    let n5 = SCAN_CHANNELS.iter().filter(|(band, _)| *band == 2).count();
    if n2 + n5 != SCAN_CHANNELS.len() || out.len() < CHANNEL_DOMAIN_LEN {
        return None;
    }
    out[..CHANNEL_DOMAIN_LEN].fill(0);
    out[0] = b'0';
    out[1] = b'0';
    out[5] = 3;
    out[6] = 3;
    out[8] = fit_u8(n2);
    out[9] = fit_u8(n5);
    let mut at = 12;
    for band in [1u8, 2] {
        for (entry_band, channel) in SCAN_CHANNELS {
            if *entry_band != band {
                continue;
            }
            out[at..at + 2].copy_from_slice(&u16::from(*channel).to_le_bytes());
            at += 8;
        }
    }
    Some(CHANNEL_DOMAIN_LEN)
}

/// `mt76_connac2_mac_write_txwi` for one 802.11 frame.
///
/// The MMIO path posts a management frame with cut-through format on the ALTX
/// queue (`qid >= MT_TXQ_PSD`). The wireless index is [`PEER_WTBL`].
/// `band5` selects the fixed rate: CCK 1 Mbps below channel 15, OFDM 6 Mbps above.
pub fn write_txwi(txwi: &mut [u8], frame: &[u8], frame_phys: u32, band5: bool) -> bool {
    if txwi.len() < TXD_LEN + TXP_LEN || frame.len() < 24 || frame.len() > 0x3fff {
        return false;
    }
    txwi[..TXD_LEN + TXP_LEN].fill(0);
    write_txd(txwi, frame, band5);
    txwi[32..34].copy_from_slice(&0x8000u16.to_le_bytes());
    txwi[40..44].copy_from_slice(&frame_phys.to_le_bytes());
    let len = fit_u16(frame.len()) | 0x8000;
    txwi[44..46].copy_from_slice(&len.to_le_bytes());
    true
}

/// `MT_LMAC_ALTX0`. Management frames use this queue on the MMIO path.
const MT_LMAC_ALTX0: u32 = 0x10;

/// TXD words before the TXP buffer pointer. `frame` is at least 24 bytes.
///
/// Packet format is cut-through: `MT_TXD0_PKT_FMT` (bits 24:23) stays clear.
fn write_txd(txwi: &mut [u8], frame: &[u8], band5: bool) {
    let total = frame.len() + TXD_LEN;
    let tx0 = u32::from(low_u16(total)) | (MT_LMAC_ALTX0 << 25);
    txwi[..4].copy_from_slice(&tx0.to_le_bytes());
    let fc = u16::from_le_bytes([frame[0], frame[1]]);
    let fc_kind = u32::from((fc & 0x000c) >> 2);
    let subtype = u32::from((fc & 0x00f0) >> 4);
    let hdr = header_len(frame);
    let words = u32::try_from(hdr / 2).expect("header words");
    let tx1 = (1u32 << 31) | (words << 11) | (2 << 16) | PEER_WTBL;
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
        // `mt76_connac2_mac_tx_rate_val` on connac2 uses the lowest basic rate.
        // That is CCK 1 Mbps on 2.4 GHz and OFDM 6 Mbps (`0x40`) on 5 GHz.
        let rate = if band5 { 0x40u32 } else { 0 };
        let tx6 = (1u32 << 2) | (rate << 16);
        txwi[24..28].copy_from_slice(&tx6.to_le_bytes());
        tx3 |= 1 << 28;
    }
    txwi[12..16].copy_from_slice(&tx3.to_le_bytes());
    let tx7 = (fc_kind << 20) | (subtype << 16);
    txwi[28..32].copy_from_slice(&tx7.to_le_bytes());
}

/// Header length from `ieee80211_hdrlen`.
///
/// Bit 7 of the first frame-control byte is the `QoS` bit only on a data frame.
/// On a management frame it is a subtype bit. Authentication is `0xb0`, so
/// treating that bit as `QoS` reports 26 bytes and the MAC reads the algorithm
/// field as part of the header.
fn header_len(frame: &[u8]) -> usize {
    let fc = frame[0];
    if fc & 0x0c == 0x08 && fc & 0x80 != 0 {
        26
    } else {
        24
    }
}

const PKT_TYPE_NORMAL: u32 = 2;
const PKT_TYPE_RX_EVENT: u32 = 7;
const PKT_TYPE_NORMAL_MCU: u32 = 8;

/// True when `mt7921_queue_rx_skb` passes this packet to `mt7921_mac_fill_rx`.
///
/// That is `PKT_TYPE_NORMAL`, or `PKT_TYPE_RX_EVENT` with flag 1, which Linux
/// renames `PKT_TYPE_NORMAL_MCU`. Linux applies this to every receive ring,
/// the MCU rings included. The type is RXD0 bits 31:27 and the flag is 19:16.
#[must_use]
pub fn is_rx_frame(buf: &[u8]) -> bool {
    let Some(word) = buf.get(..4) else {
        return false;
    };
    let rxd0 = u32::from_le_bytes([word[0], word[1], word[2], word[3]]);
    let kind = rxd0 >> 27;
    let flag = (rxd0 >> 16) & 0xf;
    matches!(kind, PKT_TYPE_NORMAL | PKT_TYPE_NORMAL_MCU)
        || (kind == PKT_TYPE_RX_EVENT && flag == 1)
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

/// How `mt7921_mac_fill_rx` classifies one connac2 receive descriptor.
#[derive(Clone, Copy, Debug, Default)]
pub struct RxLayout(u8);

const RX_TRANSLATED: u8 = 1;
const RX_DROP: u8 = 2;
const RX_PLAIN: u8 = 4;
const RX_AMSDU: u8 = 8;

impl RxLayout {
    /// `MT_RXD2_NORMAL_HDR_TRANS`. The bytes after [`frame_off`] are Ethernet.
    #[must_use]
    pub fn translated(self) -> bool {
        self.0 & RX_TRANSLATED != 0
    }

    /// Header translation together with `MT_RXD1_NORMAL_CM`. `fill_rx` rejects it.
    #[must_use]
    pub fn rejected(self) -> bool {
        self.0 & RX_DROP != 0
    }

    /// `SEC_MODE` is set and neither `CLM` nor `CM` is. The IV and MIC are gone.
    #[must_use]
    pub fn plain(self) -> bool {
        self.0 & RX_PLAIN != 0
    }

    /// `MT_RXD4_NORMAL_PAYLOAD_FORMAT` is non-zero. Two bytes follow the 802.11 header.
    #[must_use]
    pub fn amsdu(self) -> bool {
        self.0 & RX_AMSDU != 0
    }
}

/// Read the `fill_rx` flags from a connac2 receive descriptor.
#[must_use]
pub fn rx_layout(buf: &[u8]) -> RxLayout {
    let Some(rxd1) = rx_word(buf, 4) else {
        return RxLayout(0);
    };
    let Some(rxd2) = rx_word(buf, 8) else {
        return RxLayout(0);
    };
    let mut flags = 0;
    if rxd2 & (1 << 13) != 0 {
        flags |= RX_TRANSLATED;
        if rxd1 & (1 << 23) != 0 {
            flags |= RX_DROP;
        }
    }
    let sec = (rxd1 >> 16) & 0x1f;
    if sec != 0 && rxd1 & ((1 << 23) | (1 << 24)) == 0 {
        flags |= RX_PLAIN;
    }
    if rx_word(buf, 16).is_some_and(|rxd4| rxd4 & 3 != 0) {
        flags |= RX_AMSDU;
    }
    RxLayout(flags)
}

fn rx_word(buf: &[u8], at: usize) -> Option<u32> {
    let bytes = buf.get(at..at + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
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

/// Byte 3 is the association result. `write_key` leaves it clear.
const JOIN_UP: u8 = 1;

/// The radio sets this after `associate` returns. `up` is a finished handshake.
pub fn write_join(page: &mut [u8], up: bool) -> bool {
    if page.len() < 4 || page[0] != 1 {
        return false;
    }
    page[3] = if up { JOIN_UP } else { 2 };
    true
}

/// True when the radio finished the handshake for the posted key.
#[must_use]
pub fn join_up(page: &[u8]) -> bool {
    page.len() >= 4 && page[0] == 1 && page[3] == JOIN_UP
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

/// Room for [`SCAN_MAX`] lines of `NN `, a 32-byte name, and a newline.
/// A survey mark is shorter than that.
pub const SURVEY_REPORT_MAX: usize = SCAN_MAX * 36;

const PAGE_SURVEY: u8 = 2;
const PAGE_LIST: u8 = 3;
const SURVEY_STRIDE: usize = 33;
/// Byte after the name list. The length byte and the mark text follow.
const SURVEY_MARK: usize = 4 + SCAN_MAX * SURVEY_STRIDE;
const SURVEY_MARK_MAX: usize = 80;

const _: () = assert!(SURVEY_MARK + 1 + SURVEY_MARK_MAX <= 0x1000);
const _: () = assert!(SURVEY_REPORT_MAX > SURVEY_MARK_MAX);

fn survey_span() -> usize {
    SURVEY_MARK
}

/// The shell asks which names the radio can hear. The radio replaces this tag.
pub fn write_survey(page: &mut [u8]) -> bool {
    if page.len() < survey_span() {
        return false;
    }
    page[..survey_span()].fill(0);
    page[0] = PAGE_SURVEY;
    if page.len() > SURVEY_MARK {
        page[SURVEY_MARK] = 0;
    }
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

/// Counters for one survey, in the order the mark prints them.
#[derive(Clone, Copy, Default)]
pub struct SurveyCounts {
    /// Completions on the data ring.
    pub rx: u32,
    /// Frames that arrived on an MCU ring. See [`is_rx_frame`].
    pub ev: u32,
    /// Frames whose network name was kept.
    pub ok: u32,
    /// The firmware signaled scan-done.
    pub done: bool,
    /// Control byte of the first copied frame.
    pub fc: Option<u8>,
    /// Channels the firmware finished.
    pub channels: u8,
    /// `beacon_scan_num` from scan-done.
    pub beacons: u32,
}

/// `rx`, `ev`, `ok`, and `done` for one survey.
///
/// `fc` is present only when frames arrived and no name was kept. A copied
/// frame contributes its control byte. `--` means `frame_off` rejected every frame.
/// `ch` and `bcn` follow a scan-done event: channels the firmware finished,
/// and `beacon_scan_num`.
#[must_use]
pub fn format_survey_mark(out: &mut [u8], counts: &SurveyCounts) -> usize {
    format_survey_mark_at(out, counts).unwrap_or(0)
}

fn format_survey_mark_at(out: &mut [u8], counts: &SurveyCounts) -> Option<usize> {
    let mut n = 0;
    n = write_bytes(out, n, b"rx ")?;
    n = write_dec(out, n, counts.rx)?;
    n = write_bytes(out, n, b" ev ")?;
    n = write_dec(out, n, counts.ev)?;
    n = write_bytes(out, n, b" ok ")?;
    n = write_dec(out, n, counts.ok)?;
    n = write_bytes(out, n, b" done ")?;
    n = write_bytes(out, n, if counts.done { b"1" } else { b"0" })?;
    if (counts.rx > 0 || counts.ev > 0) && counts.ok == 0 {
        n = write_bytes(out, n, b" fc ")?;
        n = match counts.fc {
            Some(byte) => write_bytes(out, n, &hex_byte(byte))?,
            None => write_bytes(out, n, b"--")?,
        };
    }
    if counts.done {
        n = write_bytes(out, n, b" ch ")?;
        n = write_dec(out, n, u32::from(counts.channels))?;
        n = write_bytes(out, n, b" bcn ")?;
        n = write_dec(out, n, counts.beacons)?;
    }
    Some(n)
}

fn hex_byte(byte: u8) -> [u8; 2] {
    [hex_nibble(byte >> 4), hex_nibble(byte)]
}

/// What the authentication wait kept instead of a matching reply.
///
/// `rx` and `ev` match the survey mark. `skip` counts MCU completions
/// [`is_rx_frame`] rejected. `tr` counts frames whose descriptor has header
/// translation. `fc` is the last frame that copied, when one did.
#[derive(Clone, Copy, Default)]
pub struct AuthMiss {
    pub rx: u16,
    pub ev: u16,
    pub skip: u16,
    pub tr: u16,
    pub fc: Option<u8>,
}

/// Panel line for a posted authentication frame with no matching reply.
///
/// The stage word stays `auth rx`. The counts use the survey words, plus
/// `skip` for an MCU completion that was not a frame.
#[must_use]
pub fn write_auth_miss(out: &mut [u8], miss: &AuthMiss) -> usize {
    write_miss_at(out, b"mt7921: auth rx fc ", *miss).unwrap_or(0)
}

/// Panel line when the four-way handshake returns without a key.
///
/// The stage word stays `eapol`. `rx`, `ev`, `skip`, and `fc` mean the same
/// thing they mean on the `auth rx` line. `tr` is the header-translation count.
#[must_use]
pub fn write_eapol_miss(out: &mut [u8], miss: &AuthMiss) -> usize {
    let Some(n) = write_miss_at(out, b"mt7921: eapol fc ", *miss) else {
        return 0;
    };
    let Some(n) = write_bytes(out, n, b" tr ") else {
        return n;
    };
    write_dec(out, n, u32::from(miss.tr)).unwrap_or(n)
}

fn write_miss_at(out: &mut [u8], prefix: &[u8], miss: AuthMiss) -> Option<usize> {
    let mut n = 0;
    n = write_bytes(out, n, prefix)?;
    n = match miss.fc {
        Some(byte) => write_bytes(out, n, &hex_byte(byte))?,
        None => write_bytes(out, n, b"--")?,
    };
    n = write_bytes(out, n, b" rx ")?;
    n = write_dec(out, n, u32::from(miss.rx))?;
    n = write_bytes(out, n, b" ev ")?;
    n = write_dec(out, n, u32::from(miss.ev))?;
    n = write_bytes(out, n, b" skip ")?;
    n = write_dec(out, n, u32::from(miss.skip))?;
    Some(n)
}

/// Store the mark after the name list. The shell prints these bytes.
pub fn write_survey_mark(page: &mut [u8], counts: &SurveyCounts) -> bool {
    let mut line = [0u8; SURVEY_MARK_MAX];
    let n = format_survey_mark(&mut line, counts);
    if n == 0 || n > SURVEY_MARK_MAX || page.len() < SURVEY_MARK + 1 + n {
        return false;
    }
    page[SURVEY_MARK] = fit_u8(n);
    page[SURVEY_MARK + 1..SURVEY_MARK + 1 + n].copy_from_slice(&line[..n]);
    true
}

#[must_use]
pub fn read_survey_mark(page: &[u8]) -> Option<&[u8]> {
    let len = usize::from(*page.get(SURVEY_MARK)?);
    if len == 0 || len > SURVEY_MARK_MAX || page.len() < SURVEY_MARK + 1 + len {
        return None;
    }
    Some(&page[SURVEY_MARK + 1..SURVEY_MARK + 1 + len])
}

/// Bytes `setup-wifi` prints after the radio stores a list.
///
/// Each stored name is one line: the 1-based row, a space, the name, and a
/// newline, in the list's byte order. Zero names are the survey mark and a
/// newline. `None` means the page has no list, or it lists no name and has no mark.
#[must_use]
pub fn format_survey_report(page: &[u8], out: &mut [u8]) -> Option<usize> {
    let count = read_survey_count(page)?;
    if count == 0 {
        let mark = read_survey_mark(page)?;
        let n = write_bytes(out, 0, mark)?;
        return write_bytes(out, n, b"\n");
    }
    let mut n = 0;
    for i in 0..count {
        let name = read_survey_ssid(page, i)?;
        let index = u32::try_from(i.checked_add(1)?).ok()?;
        n = write_dec(out, n, index)?;
        n = write_bytes(out, n, b" ")?;
        n = write_bytes(out, n, name)?;
        n = write_bytes(out, n, b"\n")?;
    }
    Some(n)
}

/// True when `cmd` is `setup-wifi` in either ASCII case.
///
/// The keyboard stores an uppercase letter when Shift or Caps Lock is set,
/// and the shell echoes those stored bytes. The survey command is that name.
#[must_use]
pub fn is_setup_wifi(cmd: &[u8]) -> bool {
    cmd.eq_ignore_ascii_case(b"setup-wifi")
}

fn write_bytes(out: &mut [u8], at: usize, bytes: &[u8]) -> Option<usize> {
    let end = at.checked_add(bytes.len())?;
    if end > out.len() {
        return None;
    }
    out[at..end].copy_from_slice(bytes);
    Some(end)
}

fn write_dec(out: &mut [u8], at: usize, value: u32) -> Option<usize> {
    let mut digits = [0u8; 10];
    let mut v = value;
    let mut n = 0;
    loop {
        digits[n] = b'0' + u8::try_from(v % 10).ok()?;
        n += 1;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    let end = at.checked_add(n)?;
    if end > out.len() {
        return None;
    }
    for i in 0..n {
        out[at + i] = digits[n - 1 - i];
    }
    Some(end)
}

/// The printed row when `line` is a live number, otherwise a listed name.
///
/// The row is chosen first. A beacon named `2` must not capture the second
/// row's passphrase. A line that is neither is `None`.
#[must_use]
pub fn survey_choice<'a>(page: &'a [u8], line: &[u8]) -> Option<&'a [u8]> {
    let n = read_survey_count(page)?;
    if let Some(index) = menu_index(line) {
        if index >= n {
            return None;
        }
        return survey_name(page, index);
    }
    for i in 0..n {
        let name = survey_name(page, i)?;
        if name == line {
            return Some(name);
        }
    }
    None
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

/// `mt7921_mcu_set_chan_info` body for a 20 MHz channel.
///
/// `rx_path` is `SET_RX_PATH`: the receive field is the antenna mask.
/// A channel switch stores the number of streams there instead.
/// `switch_reason` stays 0, `CH_SWITCH_NORMAL`.
pub fn write_chan(out: &mut [u8], channel: u8, band: u8, antenna_mask: u8, rx_path: bool) -> bool {
    if out.len() < 76 {
        return false;
    }
    out[..76].fill(0);
    let streams = u8::try_from(antenna_mask.count_ones()).unwrap_or(0);
    out[0] = channel;
    out[1] = channel;
    out[3] = streams;
    out[4] = if rx_path { antenna_mask } else { streams };
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

/// `MT_EE_HW_TYPE`. Linux reads this byte before it sends CLC.
pub const EE_HW_TYPE: u32 = 0x55b;
/// Bytes in `mt7921_mcu_eeprom_info`.
pub const EFUSE_BLOCK_LEN: usize = 24;
/// Wireless-table index Linux assigns to the first station after registration.
pub const WTBL_RESERVED: u32 = 19;
/// Same index as [`WTBL_RESERVED`], stored in the BSS TLV's `u16` fields.
const VIF_WTBL: u16 = 19;
const _: () = assert!(WTBL_RESERVED == VIF_WTBL as u32);
/// First peer. Index 0 is the global management entry from `mt792x_init_wcid`.
pub const PEER_WTBL: u32 = PEER_INDEX as u32;
const PEER_INDEX: u8 = 1;

/// `MCU_EXT_QUERY(EFUSE_ACCESS)` for the 16-byte block that holds `offset`.
pub fn write_efuse_block(out: &mut [u8], offset: u32) -> bool {
    if out.len() < EFUSE_BLOCK_LEN {
        return false;
    }
    out[..EFUSE_BLOCK_LEN].fill(0);
    let addr = offset & !0xf;
    out[..4].copy_from_slice(&addr.to_le_bytes());
    true
}

/// One byte from an efuse-access reply. The data starts eight bytes after the RXD.
#[must_use]
pub fn efuse_block_byte(buf: &[u8], offset: u32) -> Option<u8> {
    let index = usize::try_from(offset & 0xf).ok()?;
    buf.get(RXD_LEN + 8 + index).copied()
}

/// `MCU_CE_CMD(FWLOG_2_HOST)` with control 1, the value Linux sends after CLC.
pub fn write_fw_log(out: &mut [u8]) -> bool {
    if out.len() < 4 {
        return false;
    }
    out[..4].fill(0);
    out[0] = 1;
    true
}

/// `mt76_connac_mcu_set_deep_sleep`. Deep sleep on writes `KeepFullPwr 0`.
pub fn write_keep_full_power(out: &mut [u8], deep_sleep: bool) -> bool {
    const LEN: usize = 328;
    if out.len() < LEN {
        return false;
    }
    out[..LEN].fill(0);
    let text: &[u8] = if deep_sleep {
        b"KeepFullPwr 0"
    } else {
        b"KeepFullPwr 1"
    };
    out[8..8 + text.len()].copy_from_slice(text);
    true
}

/// Streams and address from `GET_NIC_CAPAB`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NicCap {
    /// `(1 << nss) - 1`, from the PHY element. `1` when that element is absent.
    pub antenna_mask: u8,
    pub mac: Option<[u8; 6]>,
    pub has_6g: bool,
    /// `MT792x_CHIP_CAP_CLC_EVT_EN` is bit 0 of the chip-capability element.
    pub clc_event: bool,
}

const NIC_MAC: u32 = 7;
const NIC_PHY: u32 = 8;
const NIC_6G: u32 = 0x18;
const NIC_CHIP: u32 = 0x20;

fn note_nic_tlv(cap: &mut NicCap, kind: u32, data: &[u8]) {
    match kind {
        NIC_MAC if data.len() >= 6 => {
            let mut mac = [0u8; 6];
            mac.copy_from_slice(&data[..6]);
            cap.mac = Some(mac);
        }
        NIC_PHY if data.len() >= 5 => {
            let nss = data[4];
            if nss > 0 && nss < 8 {
                cap.antenna_mask = (1u8 << nss) - 1;
            }
        }
        NIC_6G if !data.is_empty() => cap.has_6g = data[0] != 0,
        NIC_CHIP if data.len() >= 8 => {
            let bytes = data[..8].try_into().unwrap_or([0; 8]);
            cap.clc_event = u64::from_le_bytes(bytes) & 1 != 0;
        }
        _ => {}
    }
}

/// Parse a `GET_NIC_CAPAB` reply. The event body follows the connac2 RXD.
#[must_use]
pub fn parse_nic_cap(buf: &[u8]) -> Option<NicCap> {
    let mut body = buf.get(RXD_LEN..)?;
    if body.len() < 4 {
        return None;
    }
    let n_element = u16::from_le_bytes([body[0], body[1]]);
    body = &body[4..];
    let mut cap = NicCap {
        antenna_mask: 1,
        mac: None,
        has_6g: false,
        clc_event: false,
    };
    for _ in 0..n_element {
        if body.len() < 8 {
            break;
        }
        let kind = u32::from_le_bytes(body[..4].try_into().ok()?);
        let len = u32::from_le_bytes(body[4..8].try_into().ok()?) as usize;
        body = &body[8..];
        if body.len() < len {
            break;
        }
        note_nic_tlv(&mut cap, kind, &body[..len]);
        body = &body[len..];
    }
    Some(cap)
}

/// The non-download CLC region of a RAM image, when the trailer names one.
#[must_use]
pub fn clc_image(image: &[u8]) -> Option<&[u8]> {
    if image.len() < RAM_TRAILER {
        return None;
    }
    let trailer = image.len() - RAM_TRAILER;
    let n = usize::from(image[trailer + 2]);
    if n == 0 || n > MAX_REGIONS {
        return None;
    }
    let mut data = 0usize;
    for i in 0..n {
        let at = trailer - (n - i) * RAM_REGION;
        let region = image.get(at..at + RAM_REGION)?;
        let len = u32::from_le_bytes(region[20..24].try_into().ok()?) as usize;
        let feature = region[24];
        let kind = region[25];
        let end = data.checked_add(len)?;
        if end > trailer {
            return None;
        }
        if feature & (1 << 6) != 0 && kind == 2 {
            return image.get(data..end);
        }
        data = end;
    }
    None
}

/// Header of one `SET_CLC` command, before the country rule.
pub const CLC_HDR: usize = 76;

/// `MT792x_CLC_POWER`. The encapsulation bit is checked only for this index.
const CLC_POWER: u8 = 0;
/// `MT792x_CLC_REGD`. The loader stores it; `mt7921_mcu_set_clc` does not submit it.
const CLC_REGD: u8 = 3;
/// `MT792x_CLC_MAX_NUM`. `mt7921_load_clc` leaves a record at or past this index unused.
const CLC_MAX: u8 = 4;

/// World-domain (`00`) rules from a CLC region.
///
/// `mt7921_load_clc` stores a record only while its index is below
/// `MT792x_CLC_MAX_NUM`. `mt7921_mcu_set_clc` then skips `MT792x_CLC_REGD`.
/// Index 0 is the power block, and its encapsulation bit has to match `hw_encap`.
/// A record length the published parser rejects stops the walk.
pub fn each_world_rule(
    region: &[u8],
    hw_encap: u8,
    mut visit: impl FnMut(u8, [u8; 2], [u8; 2], &[u8]) -> bool,
) -> bool {
    let mut at = 0usize;
    while at + 16 <= region.len() {
        let len = u32::from_le_bytes(region[at..at + 4].try_into().unwrap_or([0; 4])) as usize;
        if len < 16 || at.checked_add(len).is_none_or(|end| end > region.len()) {
            return false;
        }
        let idx = region[at + 4];
        let encap = region[at + 7] & 1;
        // A newer firmware may add a record this driver does not store.
        let skip =
            idx >= CLC_MAX || idx == CLC_REGD || (idx == CLC_POWER && encap != (hw_encap & 1));
        if !skip {
            let body = &region[at + 16..at + len];
            let mut rule = 0usize;
            while body.len() - rule > 16 {
                if rule + 6 > body.len() {
                    return false;
                }
                let data_len = usize::from(u16::from_le_bytes([body[rule + 4], body[rule + 5]]));
                let next = rule + 6 + data_len;
                if next > body.len() {
                    return false;
                }
                if body[rule] == b'0' && body[rule + 1] == b'0' {
                    let kind = [body[rule + 2], body[rule + 3]];
                    if !visit(idx, [b'0', b'0'], kind, &body[rule + 6..next]) {
                        return false;
                    }
                }
                rule = next;
            }
        }
        at += len;
    }
    true
}

/// `MCU_CE_CMD(SET_CLC)` for one country rule. `event` sets `CLC_CAP_EVT_EN`.
#[must_use]
pub fn write_clc(
    out: &mut [u8],
    idx: u8,
    alpha: [u8; 2],
    kind: [u8; 2],
    data: &[u8],
    event: bool,
) -> Option<usize> {
    let total = CLC_HDR.checked_add(data.len())?;
    if out.len() < total || total > usize::from(u16::MAX) {
        return None;
    }
    out[..total].fill(0);
    out[0] = 1;
    out[2..4].copy_from_slice(&u16::try_from(total).ok()?.to_le_bytes());
    out[4] = idx;
    out[5] = 1;
    if event {
        out[7] = 1;
    }
    out[8] = alpha[0];
    out[9] = alpha[1];
    out[10] = kind[0];
    out[11] = kind[1];
    out[CLC_HDR..total].copy_from_slice(data);
    Some(total)
}

/// Channels in one `SET_RATE_TX_POWER` message on the 7921.
pub const TX_POWER_BATCH: usize = 8;
/// `sizeof(mt76_connac_tx_power_limit_tlv)`.
pub const TX_POWER_HDR: usize = 44;
/// Channel byte plus `MT_SKU_POWER_LIMIT` (161).
pub const SKU_LEN: usize = 162;

/// 2.4 GHz centers in `mt76_connac_mcu_rate_txpower_band`.
pub const POWER_2G: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];

/// 5 GHz centers in `mt76_connac_mcu_rate_txpower_band`.
pub const POWER_5G: &[u8] = &[
    36, 38, 40, 42, 44, 46, 48, 50, 52, 54, 56, 58, 60, 62, 64, 100, 102, 104, 106, 108, 110, 112,
    114, 116, 118, 120, 122, 124, 126, 128, 132, 134, 136, 138, 140, 142, 144, 149, 151, 153, 155,
    157, 159, 161, 165, 169, 173, 177,
];

/// 6 GHz centers in `mt76_connac_mcu_rate_txpower_band`.
pub const POWER_6G: &[u8] = &[
    1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29, 33, 35, 37, 39, 41, 43, 45, 47, 49, 51,
    53, 55, 57, 59, 61, 65, 67, 69, 71, 73, 75, 77, 79, 81, 83, 85, 87, 89, 91, 93, 97, 99, 101,
    103, 105, 107, 109, 111, 113, 115, 117, 119, 121, 123, 125, 129, 131, 133, 135, 137, 139, 141,
    143, 145, 147, 149, 151, 153, 155, 157, 161, 163, 165, 167, 169, 171, 173, 175, 177, 179, 181,
    183, 185, 187, 189, 193, 195, 197, 199, 201, 203, 205, 207, 209, 211, 213, 215, 217, 219, 221,
    225, 227, 229, 233,
];

/// One batch of `SET_RATE_TX_POWER`. Every rate limit is 127, Linux's uncapped value.
#[must_use]
pub fn write_tx_power_batch(
    out: &mut [u8],
    band: u8,
    channels: &[u8],
    batch: usize,
    global_last: u8,
) -> Option<usize> {
    let start = batch.checked_mul(TX_POWER_BATCH)?;
    if start >= channels.len() {
        return None;
    }
    let end = (start + TX_POWER_BATCH).min(channels.len());
    let slice = &channels[start..end];
    let total = TX_POWER_HDR + slice.len() * SKU_LEN;
    if out.len() < total {
        return None;
    }
    out[..total].fill(0);
    out[4] = u8::try_from(slice.len()).ok()?;
    out[5] = band;
    if slice.last().copied() == Some(global_last) {
        out[6] = 1;
    }
    out[8] = b'0';
    out[9] = b'0';
    for (i, channel) in slice.iter().enumerate() {
        let at = TX_POWER_HDR + i * SKU_LEN;
        out[at] = *channel;
        out[at + 1..at + SKU_LEN].fill(127);
    }
    Some(total)
}

/// One register update from `mt792x_mac_init_band`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegOp {
    /// OR these bits.
    Or(u32, u32),
    /// Clear these bits.
    Clear(u32, u32),
    /// `mt76_rmw_field` of `MT_DMA_DCR0_MAX_RX_LEN`, value in bytes.
    RxLen(u32, u32),
    /// Replace `mask` with `value`. `value` is already shifted.
    Field(u32, u32, u32),
}

const fn band_base(band: u32, even: u32, odd: u32) -> u32 {
    if band == 0 {
        even
    } else {
        odd
    }
}

/// Register updates for one band. Band 0 is the even base.
#[must_use]
pub fn band_init_ops(band: u32) -> [RegOp; 8] {
    let tmac = band_base(band, 0x820e_4000, 0x820f_4000) + 0x0f4;
    let dma = band_base(band, 0x820e_7000, 0x820f_7000);
    let rscr = band_base(band, 0x820e_9000, 0x820f_9000) + 0x008;
    let mib = band_base(band, 0x820e_d000, 0x820f_d000) + 0x004;
    let time = band_base(band, 0x820e_5000, 0x820f_5000) + 0x03c4;
    let air = band_base(band, 0x820e_5000, 0x820f_5000) + 0x0380;
    let rcpi = (3 << 30) | (3 << 24);
    [
        RegOp::Field(tmac, 0x3f, 0x3f),
        RegOp::Or(tmac, (1 << 17) | (1 << 18)),
        RegOp::Or(time, 1 << 30),
        RegOp::Or(air, 1 << 30),
        RegOp::Or(mib, (1 << 8) | (1 << 9)),
        RegOp::RxLen(dma, 1536),
        RegOp::Clear(dma, 1 << 23),
        RegOp::Field(rscr, rcpi, 3 << 24),
    ]
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
        let n = Stage::AuthTx.paint(&mut out);
        assert_eq!(&out[..n], b"mt7921: auth tx");
        let n = Stage::AuthRx.paint(&mut out);
        assert_eq!(&out[..n], b"mt7921: auth rx");
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
        assert!(!join_up(&page));
        assert!(write_join(&mut page, true));
        assert!(join_up(&page));
        assert!(write_join(&mut page, false));
        assert!(!join_up(&page));
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
        assert_eq!(buf[1185], 1);
        assert!(write_scan(&mut buf, b"", 3));
        assert_eq!(buf[2], 1);
        assert_eq!(buf[3], 1);
        assert_eq!(buf[4], 0);
        assert_eq!(buf[12], 0);
        assert_eq!(buf[1185], 0);
    }

    #[test]
    fn channel_domain_lists_the_scan_channels() {
        let mut buf = [0u8; CHANNEL_DOMAIN_LEN];
        let n = write_channel_domain(&mut buf).unwrap();
        let n2 = SCAN_CHANNELS.iter().filter(|(band, _)| *band == 1).count();
        let n5 = SCAN_CHANNELS.iter().filter(|(band, _)| *band == 2).count();
        assert_eq!(n, CHANNEL_DOMAIN_LEN);
        assert_eq!(&buf[..2], b"00");
        assert_eq!(buf[5], 3);
        assert_eq!(buf[6], 3);
        assert_eq!(buf[8], u8::try_from(n2).unwrap());
        assert_eq!(buf[9], u8::try_from(n5).unwrap());
        assert_eq!(
            u16::from_le_bytes([buf[12], buf[13]]),
            u16::from(SCAN_CHANNELS[0].1)
        );
        let at = 12 + n2 * 8;
        assert_eq!(
            u16::from_le_bytes([buf[at], buf[at + 1]]),
            u16::from(SCAN_CHANNELS[n2].1)
        );
        assert!(write_channel_domain(&mut [0u8; 8]).is_none());
    }

    #[test]
    fn uni_header_is_shorter_than_the_mcu_header() {
        let mut buf = [0u8; 64];
        let cmd = uni_cmd(0x01);
        assert!(is_uni_cmd(cmd));
        assert_eq!(mcu_header_len(cmd), UNI_TXD_LEN);
        assert!(write_mcu_header(&mut buf, cmd, 4, DEV_INFO_LEN));
        let len = u16::from_le_bytes([buf[32], buf[33]]);
        assert_eq!(
            len,
            u16::try_from(UNI_TXD_LEN - TXD_LEN + DEV_INFO_LEN).unwrap()
        );
        assert_eq!(u16::from_le_bytes([buf[34], buf[35]]), 1);
        assert_eq!(buf[37], 0xa0);
        assert_eq!(buf[39], 4);
        assert_eq!(buf[43], 0x07);
        assert!(!write_uni_txd(&mut [0u8; 16], cmd, 1, 0));
    }

    #[test]
    fn station_registration_is_bss_zero() {
        let mac = [0x02, 0x11, 0x22, 0x33, 0x44, 0x55];
        let mut dev = [0u8; DEV_INFO_LEN];
        assert!(write_dev_info(&mut dev, &mac));
        assert_eq!(u16::from_le_bytes([dev[6], dev[7]]), 12);
        assert_eq!(dev[8], 1);
        assert_eq!(&dev[10..16], &mac);
        let mut bss = [0u8; BSS_INFO_LEN];
        assert!(write_bss_info(&mut bss));
        assert_eq!(u16::from_le_bytes([bss[6], bss[7]]), 32);
        assert_eq!(bss[8], 1);
        assert_eq!(
            u32::from_le_bytes([bss[12], bss[13], bss[14], bss[15]]),
            0x1_0001
        );
        assert_eq!(bss[16], 1);
        assert!(!write_dev_info(&mut [0u8; 8], &mac));
        assert!(!write_bss_info(&mut [0u8; 8]));
    }

    #[test]
    fn auth_txd_is_cut_through_on_altx() {
        let mut frame = [0u8; 64];
        let bssid = [0x02, 0x00, 0x00, 0x00, 0x00, 0x01];
        let sta = [0x02, 0x11, 0x22, 0x33, 0x44, 0x55];
        let n = crate::wpa::write_auth(&mut frame, &bssid, &sta, 1);
        let mut txwi = [0u8; TXD_LEN + TXP_LEN];
        assert!(write_txwi(&mut txwi, &frame[..n], 0x01f6_8000, true));
        let tx0 = u32::from_le_bytes(txwi[0..4].try_into().expect("txd0"));
        let tx1 = u32::from_le_bytes(txwi[4..8].try_into().expect("txd1"));
        let tx6 = u32::from_le_bytes(txwi[24..28].try_into().expect("txd6"));
        assert_eq!((tx0 >> 23) & 0x3, 0, "packet format");
        assert_eq!(tx0 >> 25, 0x10, "queue");
        assert_eq!(tx1 & 0x3ff, PEER_WTBL, "wireless index");
        assert_eq!((tx1 >> 11) & 0x1f, 12, "management header words");
        assert_eq!((tx6 >> 16) & 0x3fff, 0x40, "5 GHz rate");
        assert_eq!(u16::from_le_bytes([txwi[32], txwi[33]]), 0x8000);
        assert!(write_txwi(&mut txwi, &frame[..n], 0x01f6_8000, false));
        let tx6 = u32::from_le_bytes(txwi[24..28].try_into().expect("txd6"));
        assert_eq!((tx6 >> 16) & 0x3fff, 0, "2.4 GHz rate");
    }

    #[test]
    fn qos_data_header_is_thirteen_words() {
        let mut frame = [0u8; 64];
        let bssid = [0x02, 0x00, 0x00, 0x00, 0x00, 0x01];
        let sta = [0x02, 0x11, 0x22, 0x33, 0x44, 0x55];
        let n = crate::wpa::write_qos_data(&mut frame, &bssid, &sta, &bssid, b"hi", 1, false)
            .expect("qos frame");
        let mut txwi = [0u8; TXD_LEN + TXP_LEN];
        assert!(write_txwi(&mut txwi, &frame[..n], 0x01f6_8000, false));
        let tx1 = u32::from_le_bytes(txwi[4..8].try_into().expect("txd1"));
        assert_eq!((tx1 >> 11) & 0x1f, 13, "qos header words");
    }

    #[test]
    fn auth_miss_counts_each_ring() {
        let mut out = [0u8; 64];
        let n = write_auth_miss(
            &mut out,
            &AuthMiss {
                rx: 0,
                ev: 0,
                skip: 0,
                tr: 0,
                fc: None,
            },
        );
        assert_eq!(&out[..n], b"mt7921: auth rx fc -- rx 0 ev 0 skip 0");
        let n = write_auth_miss(
            &mut out,
            &AuthMiss {
                rx: 2,
                ev: 1,
                skip: 4,
                tr: 9,
                fc: Some(0xb0),
            },
        );
        assert_eq!(&out[..n], b"mt7921: auth rx fc b0 rx 2 ev 1 skip 4");
    }

    #[test]
    fn eapol_miss_counts_header_translation() {
        let mut out = [0u8; 64];
        let n = write_eapol_miss(
            &mut out,
            &AuthMiss {
                rx: 0,
                ev: 0,
                skip: 0,
                tr: 0,
                fc: None,
            },
        );
        assert_eq!(&out[..n], b"mt7921: eapol fc -- rx 0 ev 0 skip 0 tr 0");
        let n = write_eapol_miss(
            &mut out,
            &AuthMiss {
                rx: 9,
                ev: 10,
                skip: 1,
                tr: 19,
                fc: Some(0x08),
            },
        );
        assert_eq!(&out[..n], b"mt7921: eapol fc 08 rx 9 ev 10 skip 1 tr 19");
    }

    #[test]
    fn fill_rx_names_header_translation() {
        let mut buf = [0u8; 24];
        buf[9] = 1 << 5;
        let info = rx_layout(&buf);
        assert!(info.translated());
        assert!(!info.rejected());
        buf[6] = 0x80;
        assert!(rx_layout(&buf).rejected());
        buf[6] = 1;
        assert!(rx_layout(&buf).plain());
        assert!(!rx_layout(&buf).rejected());
        buf[7] = 1;
        assert!(!rx_layout(&buf).plain());
        buf[16] = 1;
        assert!(rx_layout(&buf).amsdu());
        assert!(!rx_layout(&[]).translated());
    }

    #[test]
    fn sta_rec_names_the_ap() {
        let bssid = [0x10, 0x20, 0x30, 0x40, 0x50, 0x60];
        let mut body = [0u8; STA_REC_LEN];
        assert_eq!(
            write_sta_rec(&mut body, &bssid, false, false, 0),
            Some(STA_REC_LEN)
        );
        assert_eq!(body[1], 1);
        assert_eq!(u16::from_le_bytes([body[2], body[3]]), 5);
        assert_eq!(body[4], 1);
        assert_eq!(
            u32::from_le_bytes([body[12], body[13], body[14], body[15]]),
            CONN_INFRA_AP
        );
        assert_eq!(body[16], 2);
        assert_eq!(u16::from_le_bytes([body[18], body[19]]), 0);
        assert_eq!(&body[20..26], &bssid);
        assert_eq!(u16::from_le_bytes([body[26], body[27]]), 3);
        assert_eq!(body[34], PHY_HR_DSSS | PHY_ERP);
        assert_eq!(body[64], 0);
        assert_eq!(u16::from_le_bytes([body[96], body[97]]), 0);
        let mut five = [0u8; STA_REC_LEN];
        assert_eq!(
            write_sta_rec(&mut five, &bssid, true, true, 0x123),
            Some(STA_REC_LEN)
        );
        assert_eq!(five[34], PHY_OFDM);
        assert_eq!(u16::from_le_bytes([five[18], five[19]]), 0x123);
        assert_eq!(u16::from_le_bytes([five[26], five[27]]), 1);
        assert_eq!(five[64], 2);
        assert_eq!(u16::from_le_bytes([five[96], five[97]]), 0x123);
        assert!(write_sta_rec(&mut [0u8; 8], &bssid, false, false, 0).is_none());
    }

    #[test]
    fn bss_connect_names_the_ap() {
        let bssid = [0x10, 0x20, 0x30, 0x40, 0x50, 0x60];
        let mut body = [0u8; BSS_CONNECT_LEN];
        assert!(write_bss_connect(&mut body, &bssid, false));
        assert_eq!(u16::from_le_bytes([body[6], body[7]]), 32);
        assert_eq!(body[8], 1);
        assert_eq!(
            u32::from_le_bytes([body[12], body[13], body[14], body[15]]),
            CONN_INFRA_STA
        );
        assert_eq!(body[16], 0);
        assert_eq!(&body[18..24], &bssid);
        assert_eq!(u16::from_le_bytes([body[24], body[25]]), 19);
        assert_eq!(body[29], PHY_MODE_B | PHY_MODE_G);
        assert_eq!(u16::from_le_bytes([body[30], body[31]]), 19);
        assert_eq!(u16::from_le_bytes([body[32], body[33]]), 3);
        assert_eq!(u16::from_le_bytes([body[36], body[37]]), 15);
        assert_eq!(u16::from_le_bytes([body[38], body[39]]), 8);
        assert_eq!(body[40], 0);
        assert!(write_bss_connect(&mut body, &bssid, true));
        assert_eq!(body[29], PHY_MODE_A);
        assert_eq!(
            u16::from_le_bytes([body[32], body[33]]),
            u16::from(PHY_OFDM)
        );
        assert!(!write_bss_connect(&mut [0u8; 8], &bssid, false));
    }

    #[test]
    fn bss_rlm_is_twenty_megahertz() {
        let mut body = [0u8; BSS_RLM_LEN];
        assert!(write_bss_rlm(&mut body, 6, 0, 0b11));
        assert_eq!(u16::from_le_bytes([body[4], body[5]]), 2);
        assert_eq!(u16::from_le_bytes([body[6], body[7]]), 16);
        assert_eq!(body[8], 6);
        assert_eq!(body[9], 6);
        assert_eq!(body[10], 0);
        assert_eq!(body[11], 0);
        assert_eq!(body[12], 2);
        assert_eq!(body[13], 0b11);
        assert_eq!(body[14], 1);
        assert_eq!(body[15], 0);
        assert_eq!(body[17], 0);
        assert!(write_bss_rlm(&mut body, 36, 1, 1));
        assert_eq!(body[8], 36);
        assert_eq!(body[12], 1);
        assert_eq!(body[13], 1);
        assert_eq!(body[17], 1);
        assert!(!write_bss_rlm(&mut [0u8; 8], 6, 0, 1));
    }

    #[test]
    fn rx_filter_enable_is_mode_one() {
        let mut buf = [0u8; RX_FILTER_LEN];
        assert!(write_rx_filter_enable(&mut buf));
        assert_eq!(buf[4], 1);
        assert_eq!(
            u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            1 << 31
        );
        assert_eq!(buf[12], 0);
        assert_eq!(buf[16], 0);
        assert!(!write_rx_filter_enable(&mut [0u8; 8]));
    }

    #[test]
    fn other_beacon_drop_is_cleared() {
        let mut buf = [0u8; RX_FILTER_LEN];
        assert!(write_clear_other_beacon(&mut buf));
        assert_eq!(buf[4], 2);
        assert_eq!(
            u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]),
            1 << 11
        );
        assert_eq!(buf[16], 1 << 1);
        assert_eq!(u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]), 0);
        assert!(!write_clear_other_beacon(&mut [0u8; 8]));
    }

    #[test]
    fn channel_command_uses_the_antenna_mask() {
        let mut buf = [0u8; 76];
        assert!(write_chan(&mut buf, 6, 0, 0b11, true));
        assert_eq!(buf[0], 6);
        assert_eq!(buf[3], 2);
        assert_eq!(buf[4], 0b11);
        assert_eq!(buf[5], 0);
        assert!(write_chan(&mut buf, 36, 1, 0b11, false));
        assert_eq!(buf[3], 2);
        assert_eq!(buf[4], 2);
        assert_eq!(buf[10], 1);
    }

    #[test]
    fn nic_capability_reads_streams_and_the_clc_bit() {
        let mut buf = [0u8; RXD_LEN + 4 + 8 + 12 + 8 + 8];
        buf[RXD_LEN] = 2;
        let phy = RXD_LEN + 4;
        buf[phy..phy + 4].copy_from_slice(&8u32.to_le_bytes());
        buf[phy + 4..phy + 8].copy_from_slice(&12u32.to_le_bytes());
        buf[phy + 8 + 4] = 2;
        let chip = phy + 8 + 12;
        buf[chip..chip + 4].copy_from_slice(&0x20u32.to_le_bytes());
        buf[chip + 4..chip + 8].copy_from_slice(&8u32.to_le_bytes());
        buf[chip + 8] = 1;
        let cap = parse_nic_cap(&buf).unwrap();
        assert_eq!(cap.antenna_mask, 0b11);
        assert!(cap.clc_event);
        assert!(parse_nic_cap(&[0; 8]).is_none());
    }

    #[test]
    fn clc_walk_sends_the_world_rule() {
        let mut region = [0u8; 16 + 6 + 12];
        let len = u32::try_from(region.len()).unwrap();
        region[..4].copy_from_slice(&len.to_le_bytes());
        region[7] = 0;
        region[16] = b'0';
        region[17] = b'0';
        region[18] = b'-';
        region[19] = b'0';
        region[20..22].copy_from_slice(&12u16.to_le_bytes());
        region[22] = 9;
        region[23] = 8;
        let mut seen = 0u8;
        assert!(each_world_rule(&region, 0, |idx, alpha, kind, data| {
            assert_eq!(idx, 0);
            assert_eq!(alpha, [b'0', b'0']);
            assert_eq!(kind, [b'-', b'0']);
            assert_eq!(&data[..2], &[9, 8]);
            seen += 1;
            true
        }));
        assert_eq!(seen, 1);
        assert!(each_world_rule(&region, 1, |_, _, _, _| {
            seen = 9;
            true
        }));
        assert_eq!(seen, 1);
        let mut cmd = [0u8; 128];
        let n = write_clc(
            &mut cmd,
            0,
            [b'0', b'0'],
            [b'-', b'0'],
            &region[22..34],
            false,
        )
        .unwrap();
        assert_eq!(n, CLC_HDR + 12);
        assert_eq!(cmd[0], 1);
        assert_eq!(cmd[5], 1);
        assert_eq!(&cmd[8..12], b"00-0");
        assert_eq!(cmd[CLC_HDR], 9);
        assert_eq!(cmd[CLC_HDR + 1], 8);
    }

    fn put_clc(out: &mut [u8], idx: u8) -> usize {
        let len = 16 + 6 + 12;
        out[..len].fill(0);
        out[..4].copy_from_slice(&u32::try_from(len).unwrap().to_le_bytes());
        out[4] = idx;
        out[16] = b'0';
        out[17] = b'0';
        out[18] = b'-';
        out[19] = b'0';
        out[20..22].copy_from_slice(&12u16.to_le_bytes());
        out[22] = 9;
        len
    }

    #[test]
    fn clc_walk_skips_records_the_driver_does_not_store() {
        let mut region = [0u8; 34 * 5];
        let mut at = 0usize;
        for idx in [CLC_POWER, 1, CLC_REGD, 2, CLC_MAX] {
            at += put_clc(&mut region[at..], idx);
        }
        let mut seen = [0u8; 4];
        let mut n = 0usize;
        assert!(each_world_rule(&region[..at], 0, |idx, _, _, _| {
            seen[n] = idx;
            n += 1;
            true
        }));
        assert_eq!(&seen[..n], &[CLC_POWER, 1, 2]);
    }

    #[test]
    fn band_init_sets_the_dma_receive_length() {
        let ops = band_init_ops(0);
        assert!(matches!(ops[5], RegOp::RxLen(0x820e_7000, 1536)));
        assert!(matches!(ops[6], RegOp::Clear(0x820e_7000, bit23) if bit23 == 1 << 23));
        let odd = band_init_ops(1);
        assert!(matches!(odd[5], RegOp::RxLen(0x820f_7000, 1536)));
    }

    #[test]
    fn tx_power_batch_marks_the_last_channel() {
        let mut buf = [0u8; TX_POWER_HDR + 8 * SKU_LEN];
        let n = write_tx_power_batch(&mut buf, 1, POWER_2G, 0, 14).unwrap();
        assert_eq!(n, TX_POWER_HDR + 8 * SKU_LEN);
        assert_eq!(buf[4], 8);
        assert_eq!(buf[5], 1);
        assert_eq!(buf[6], 0);
        assert_eq!(buf[TX_POWER_HDR], 1);
        assert_eq!(buf[TX_POWER_HDR + 1], 127);
        let n = write_tx_power_batch(&mut buf, 1, POWER_2G, 1, 14).unwrap();
        assert_eq!(buf[4], 6);
        assert_eq!(buf[6], 1);
        assert_eq!(buf[TX_POWER_HDR], 9);
        assert!(n > TX_POWER_HDR);
        assert_eq!(POWER_2G.len(), 14);
        assert_eq!(POWER_5G.last(), Some(&177));
        assert_eq!(POWER_6G.last(), Some(&233));
        let mut word = [0u8; 328];
        assert!(write_keep_full_power(&mut word, true));
        assert_eq!(&word[8..21], b"KeepFullPwr 0");
    }

    #[test]
    fn uni_event_status_zero_matches_the_command() {
        let mut buf = [0u8; RXD_LEN + 8];
        assert!(!uni_event_ok(&buf, 1));
        buf[RXD_LEN] = 1;
        assert!(uni_event_ok(&buf, 1));
        assert!(!uni_event_ok(&buf, 2));
        buf[RXD_LEN + 4] = 1;
        assert!(!uni_event_ok(&buf, 1));
    }

    #[test]
    fn scan_done_reads_channels_and_beacons() {
        let mut buf = [0u8; RXD_LEN + 12];
        assert!(read_scan_done(&buf).is_none());
        buf[..4].copy_from_slice(&(7u32 << 27).to_le_bytes());
        buf[28] = SCAN_DONE;
        buf[RXD_LEN + 4] = 21;
        buf[RXD_LEN + 8..RXD_LEN + 12].copy_from_slice(&4u32.to_le_bytes());
        let done = read_scan_done(&buf).unwrap();
        assert_eq!(done.channels, 21);
        assert_eq!(done.beacons, 4);
        buf[28] = 0;
        assert!(read_scan_done(&buf).is_none());
        buf[28] = SCAN_DONE;
        let short = read_scan_done(&buf[..32]).unwrap();
        assert_eq!(short.channels, 0);
        assert_eq!(short.beacons, 0);
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
        assert_eq!(survey_choice(&page, b"1"), Some(&b"2"[..]));
        assert_eq!(survey_choice(&page, b"2"), Some(&b"alpha"[..]));
        assert_eq!(survey_choice(&page, b"3"), Some(&b"zeta"[..]));
        assert_eq!(survey_choice(&page, b"alpha"), Some(&b"alpha"[..]));
        assert!(survey_choice(&page, b"02").is_none());
        assert!(survey_choice(&page, b"other").is_none());
        assert!(write_survey_list(&mut page, &[]));
        assert_eq!(read_survey_count(&page), Some(0));
        assert!(write_key(&mut page, b"alpha", b"password"));
        assert!(read_survey_count(&page).is_none());
    }

    fn counts(
        rx: u32,
        ev: u32,
        ok: u32,
        done: bool,
        fc: Option<u8>,
        ch: u8,
        bcn: u32,
    ) -> SurveyCounts {
        SurveyCounts {
            rx,
            ev,
            ok,
            done,
            fc,
            channels: ch,
            beacons: bcn,
        }
    }

    #[test]
    fn survey_mark_records_an_empty_scan() {
        let mut out = [0u8; SURVEY_MARK_MAX];
        let n = format_survey_mark(&mut out, &counts(0, 0, 0, true, None, 0, 0));
        assert_eq!(&out[..n], b"rx 0 ev 0 ok 0 done 1 ch 0 bcn 0");
        let n = format_survey_mark(&mut out, &counts(4, 0, 0, true, Some(0x08), 21, 4));
        assert_eq!(&out[..n], b"rx 4 ev 0 ok 0 done 1 fc 08 ch 21 bcn 4");
        let n = format_survey_mark(&mut out, &counts(4, 0, 0, false, None, 21, 4));
        assert_eq!(&out[..n], b"rx 4 ev 0 ok 0 done 0 fc --");
        let n = format_survey_mark(&mut out, &counts(2, 0, 2, true, Some(0x80), 13, 2));
        assert_eq!(&out[..n], b"rx 2 ev 0 ok 2 done 1 ch 13 bcn 2");
        let n = format_survey_mark(&mut out, &counts(0, 3, 0, true, Some(0x80), 21, 12));
        assert_eq!(&out[..n], b"rx 0 ev 3 ok 0 done 1 fc 80 ch 21 bcn 12");
        let max = counts(u32::MAX, u32::MAX, u32::MAX, false, Some(0xff), 0, 0);
        let n = format_survey_mark(&mut out, &max);
        assert!(n > 0 && n <= SURVEY_MARK_MAX);
        let max = counts(u32::MAX, u32::MAX, 0, true, Some(0xff), 255, u32::MAX);
        let n = format_survey_mark(&mut out, &max);
        assert!(n > 0 && n <= SURVEY_MARK_MAX);
        let mut page = [0u8; 0x1000];
        assert!(write_survey(&mut page));
        assert!(read_survey_mark(&page).is_none());
        assert!(write_survey_list(&mut page, &[]));
        assert!(write_survey_mark(
            &mut page,
            &counts(4, 0, 0, true, Some(0x80), 21, 4)
        ));
        assert_eq!(read_survey_count(&page), Some(0));
        assert_eq!(
            read_survey_mark(&page),
            Some(&b"rx 4 ev 0 ok 0 done 1 fc 80 ch 21 bcn 4"[..])
        );
    }

    #[test]
    fn frames_are_recognized_on_any_ring() {
        let word = |kind: u32, flag: u32| ((kind << 27) | (flag << 16) | 0x40).to_le_bytes();
        assert!(is_rx_frame(&word(2, 0)));
        assert!(is_rx_frame(&word(7, 1)));
        assert!(is_rx_frame(&word(8, 0)));
        assert!(!is_rx_frame(&word(7, 0)));
        assert!(!is_rx_frame(&word(6, 0)));
        assert!(!is_rx_frame(&[0u8; 3]));
    }

    #[test]
    fn setup_wifi_matches_either_ascii_case() {
        assert!(is_setup_wifi(b"setup-wifi"));
        assert!(is_setup_wifi(b"SETUP-WIFI"));
        assert!(is_setup_wifi(b"Setup-Wifi"));
        assert!(!is_setup_wifi(b"setup-wifi "));
        assert!(!is_setup_wifi(b"wifi"));
    }

    #[test]
    fn survey_report_lists_names_in_byte_order() {
        let mut page = [0u8; 0x1000];
        assert!(write_survey_list(&mut page, &[b"zeta", b"alpha", b"2"]));
        assert!(write_survey_mark(
            &mut page,
            &counts(3, 0, 3, true, None, 21, 3)
        ));
        let mut out = [0u8; SURVEY_REPORT_MAX];
        let n = format_survey_report(&page, &mut out).unwrap();
        assert_eq!(&out[..n], b"1 2\n2 alpha\n3 zeta\n");
    }

    #[test]
    fn survey_report_prints_the_mark_when_no_name_is_stored() {
        let mut page = [0u8; 0x1000];
        assert!(write_survey(&mut page));
        let mut out = [0u8; SURVEY_REPORT_MAX];
        assert!(format_survey_report(&page, &mut out).is_none());
        assert!(write_survey_list(&mut page, &[]));
        assert!(format_survey_report(&page, &mut out).is_none());
        assert!(write_survey_mark(
            &mut page,
            &counts(0, 0, 0, true, None, 0, 0)
        ));
        let n = format_survey_report(&page, &mut out).unwrap();
        assert_eq!(&out[..n], b"rx 0 ev 0 ok 0 done 1 ch 0 bcn 0\n");
        assert!(write_survey_mark(
            &mut page,
            &counts(4, 0, 0, true, Some(0x80), 21, 4)
        ));
        let n = format_survey_report(&page, &mut out).unwrap();
        assert_eq!(&out[..n], b"rx 4 ev 0 ok 0 done 1 fc 80 ch 21 bcn 4\n");
        assert!(write_survey_mark(
            &mut page,
            &counts(4, 0, 0, false, None, 0, 0)
        ));
        let n = format_survey_report(&page, &mut out).unwrap();
        assert_eq!(&out[..n], b"rx 4 ev 0 ok 0 done 0 fc --\n");
    }
}
