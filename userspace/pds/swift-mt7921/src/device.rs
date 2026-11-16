//! MT7921 MAC from Linux v6.8 `mt7921/pci.c`, `mt792x_dma.c`, and `mt76_connac_mcu.c`.
//!
//! The DMA window is physical `0x1f60000`. Each ring starts on its own page.
//! The MCU ring is 256 descriptors, one page, matching `MT7921_TX_MCU_RING_SIZE`.
//! One transmit is in flight, so the bounce buffer is reused only after the
//! DMA index moves.

use core::sync::atomic::{fence, Ordering};

use lerux_swift::{
    dump::WORDS_PER_ROW,
    mt7921::{
        band_init_ops, ce_cmd, clc_image, desc_ctrl, desc_len, download_cmd, each_world_rule,
        efuse_block_byte, efuse_mac, ext_cmd, ext_query, firmware_kind, is_uni_cmd, map_reg,
        mcu_header_len, parse_nic_cap, patch_regions, patch_status, prefetch, ram_regions,
        remap_l1, ring_regs, uni_cmd, uni_event_ok, write_bss_info, write_chan,
        write_channel_domain, write_clc, write_clear_other_beacon, write_dev_info,
        write_eeprom_mode, write_efuse_block, write_efuse_read, write_fw_log, write_mac_enable,
        write_mcu_header, write_mcu_txd, write_rts, write_rx_filter_enable, write_scan,
        write_sta_rec, write_txwi, wtbl_word, FirmwareKind, Map, RegOp, Region, Sem, Stage, TxStop,
        BSS_INFO_LEN, CHANNEL_DOMAIN_LEN, CONN_ON_LPCTL, CONN_ON_MISC, DEV_INFO_LEN,
        DMASHDL_BYPASS, DMASHDL_ENABLE, DMASHDL_SW, DMA_DONE, DUMMY_CR, DUMP_PREFETCH, DUMP_REGS,
        DUMP_RINGS, EE_HW_TYPE, EFUSE_BLOCK_LEN, FW_N9_RDY, FW_SCATTER, FW_START, GLO_CFG,
        GLO_CFG_CHAIN, GLO_CFG_CLK, GLO_CFG_EXT0, GLO_CFG_LE, GLO_CFG_OMIT_RX,
        GLO_CFG_OMIT_RX_PFET, GLO_CFG_OMIT_TX, GLO_CFG_RX_BUSY, GLO_CFG_RX_EN, GLO_CFG_TX_BUSY,
        GLO_CFG_TX_EN, GLO_CFG_TX_WB, HIF_REMAP_L1, HOST_CLR_OWN, HOST_INT_ENA, HOST_OWN_SYNC,
        HOST_SET_OWN, HW_CHIPID, HW_REV, IRQ_MASK, L0S_DIS, LAST_SEC0, LPCR_DRV_OWN, LPCR_FW_OWN,
        MCU2HOST, MCU_TXD_LEN, MCU_WAKE, MDP_DAMSDU, MDP_DCR0, MDP_DCR1, MDP_HDR_TRANS,
        NEED_REINIT, PATCH_FINISH, PATCH_IS_DL, PATCH_NOT_DL, PATCH_REL, PATCH_SEM, PCIE_MAC_INT,
        PCIE_MAC_PM, PEER_WTBL, PRI_DLY, RST_DMASHDL, RST_DTX_PTR, RST_LOGIC, RXD_LEN,
        RX_DATA_BASE, RX_EVENT_BASE, RX_FILTER_LEN, RX_WA_BASE, SCAN_REQ_LEN, STA_REC_LEN,
        SWDEF_MODE, TOP_LPCR, TXD_LEN, TXP_LEN, TX_RING_BASE, WFDMA_RST, WFSYS_INIT_DONE,
        WFSYS_RST, WFSYS_RST_B, WTBL_BUSY, WTBL_RESERVED, WTBL_UPDATE,
    },
    CLIENT_MAC,
};

use crate::ether::Link;

pub(crate) const DMA_PHYS: u32 = 0x01f6_0000;
const RING_N: u32 = 8;
/// `MT7921_TX_MCU_RING_SIZE`. One page holds 256 descriptors.
const MCU_N: u32 = 256;
const BUF: usize = 2048;
const BOUNCE: usize = 4096;
/// Words of the bounce buffer the dump rows show: the 68-byte semaphore command.
pub(crate) const BOUNCE_WORDS: usize = 17;

const OFF_TX0: usize = 0x0000;
const OFF_TX16: usize = 0x1000;
pub(crate) const OFF_TX17: usize = 0x2000;
pub(crate) const OFF_RX_EVT: usize = 0x3000;
const OFF_RX_DATA: usize = 0x4000;
const OFF_RX_WA: usize = 0x5000;
pub(crate) const OFF_BOUNCE: usize = 0x6000;
const OFF_RX0_BUFS: usize = 0x7000;
const OFF_RX_DATA_BUFS: usize = 0xb000;
const OFF_RX_WA_BUFS: usize = 0xf000;
const OFF_TXWI: usize = 0x1_3000;
const OFF_TX_FRAME: usize = 0x1_4000;

const TX_DATA: usize = 0;
const TX_FWDL: usize = 1;
const TX_MCU: usize = 2;
const RX_EVT: usize = 0;
const RX_DATA: usize = 1;
const RX_WA: usize = 2;

/// Linux polls ownership for 50 ms, ten times (`MT792x_DRV_OWN_RETRY_COUNT`).
const OWN_SPIN: u32 = 8_000_000;
const OWN_TRIES: u32 = 10;
/// Linux polls the DMA busy bits for 100 ms.
const DMA_SPIN: u32 = 8_000_000;
/// Linux sleeps 50 ms, then polls init-done for 500 ms.
const WFSYS_GAP: u32 = 8_000_000;
const WFSYS_SPIN: u32 = 40_000_000;
/// Linux polls N9 ready for 1500 ms.
const N9_SPIN: u32 = 80_000_000;
/// Linux waits 3 s for an MCU response (`mcu.timeout` in `pci_mcu.c`).
const MCU_SPIN: u32 = 40_000_000;
/// A scan walks 2.4 GHz and 5 GHz. The completion event is the bound.
const SCAN_SPIN: u32 = 200_000_000;
/// Extra data-ring polls after scan-done. Linux keeps that ring running.
const DRAIN_SPIN: u32 = 2_000_000;
const ASSOC_SPIN: u32 = 40_000_000;

#[derive(Clone, Copy)]
struct TxRing {
    regs: u32,
    desc: usize,
    count: u32,
    head: u32,
    posted: u32,
}

#[derive(Clone, Copy)]
struct RxRing {
    regs: u32,
    desc: usize,
    bufs: usize,
    head: u32,
    tail: u32,
}

pub struct Mac {
    regs: *mut u8,
    dma: *mut u8,
    tx: [TxRing; 3],
    rx: [RxRing; 3],
    seq: u8,
    n9: bool,
    ready: bool,
    stage: Stage,
    sta: [u8; 6],
    /// Antenna mask from `GET_NIC_CAPAB`. One stream until that reply arrives.
    antenna: u8,
    /// Bit 0 is 6 GHz capability. Bit 1 makes `SET_CLC` wait for the event.
    nic: u8,
    /// `bss_idx` 0 is registered. The firmware command carries wireless index 0.
    bss: bool,
    last_len: usize,
    pn: u64,
    wifi_seq: u16,
    /// Set only after the 4-way handshake. The firmware path leaves this empty,
    /// so it never loads the network queues.
    link: Option<Link>,
    /// Read just before the first MCU command. The panel no longer paints it.
    before: [u32; DUMP_REGS.len()],
    /// Read when `start` returns. The panel no longer paints it.
    after: Snapshot,
}

/// Registers, ring blocks, and DMA words read around the firmware load.
/// The panel does not paint them. `regs` follows `DUMP_REGS`, `rings` follows
/// `DUMP_RINGS`, `prefetch` follows `DUMP_PREFETCH`.
struct Snapshot {
    regs: [u32; DUMP_REGS.len()],
    rings: [[u32; 4]; DUMP_RINGS.len()],
    prefetch: [[u32; WORDS_PER_ROW]; DUMP_PREFETCH.len()],
    tx_desc: [u32; WORDS_PER_ROW],
    bounce: [u32; BOUNCE_WORDS],
    rx_desc: [u32; WORDS_PER_ROW],
}

impl Snapshot {
    const fn empty() -> Self {
        Self {
            regs: [0; DUMP_REGS.len()],
            rings: [[0; 4]; DUMP_RINGS.len()],
            prefetch: [[0; WORDS_PER_ROW]; DUMP_PREFETCH.len()],
            tx_desc: [0; WORDS_PER_ROW],
            bounce: [0; BOUNCE_WORDS],
            rx_desc: [0; WORDS_PER_ROW],
        }
    }
}

struct Scratch {
    rx: [u8; BUF],
    cmd: [u8; BOUNCE],
}

/// The root thread is the only caller. There is no cothread in this domain.
struct SharedScratch(core::cell::UnsafeCell<Scratch>);

// Safety: init and `notified` run on the root thread, and this domain has no worker.
unsafe impl Sync for SharedScratch {}

static SCRATCH: SharedScratch = SharedScratch(core::cell::UnsafeCell::new(Scratch {
    rx: [0; BUF],
    cmd: [0; BOUNCE],
}));

fn scratch() -> &'static mut Scratch {
    unsafe { &mut *SCRATCH.0.get() }
}

fn rx_slice(n: usize) -> &'static [u8] {
    unsafe { &(&(*SCRATCH.0.get()).rx)[..n] }
}

fn rx_byte(index: usize) -> u8 {
    unsafe { (&(*SCRATCH.0.get()).rx)[index] }
}

impl Mac {
    pub fn boot(regs: *mut u8, dma: *mut u8, images: &Images) -> Self {
        let mut mac = Self {
            regs,
            dma,
            tx: tx_rings(),
            rx: rx_rings(),
            seq: 0,
            n9: false,
            ready: false,
            stage: Stage::Own,
            sta: CLIENT_MAC,
            antenna: 1,
            nic: 0,
            bss: false,
            last_len: 0,
            pn: 1,
            wifi_seq: 1,
            link: None,
            before: [0; DUMP_REGS.len()],
            after: Snapshot::empty(),
        };
        let outcome = mac.start(images);
        // The panel no longer paints this readback. The read stays in the
        // firmware sequence, so the value is not discarded.
        mac.after = core::hint::black_box(mac.snapshot());
        mac.stage = match outcome {
            Ok(()) => Stage::Wait,
            Err(stage) => {
                // A descriptor the engine never finishes stays a bus read. The
                // NVMe admin command is the next device on this root port.
                mac.quiesce();
                stage
            }
        };
        mac.ready = mac.stage == Stage::Wait;
        mac
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    fn read_regs(&mut self) -> [u32; DUMP_REGS.len()] {
        let mut out = [0u32; DUMP_REGS.len()];
        for (slot, (_, addr)) in out.iter_mut().zip(DUMP_REGS) {
            *slot = self.rr(addr);
        }
        out
    }

    fn snapshot(&mut self) -> Snapshot {
        let mut snap = Snapshot::empty();
        snap.regs = self.read_regs();
        for (block, (_, regs)) in snap.rings.iter_mut().zip(DUMP_RINGS) {
            for (i, word) in block.iter_mut().enumerate() {
                *word = self.rr(regs + 4 * u32_from(i));
            }
        }
        for (row, base) in snap.prefetch.iter_mut().zip(DUMP_PREFETCH) {
            for (i, word) in row.iter_mut().enumerate() {
                *word = self.rr(base + 4 * u32_from(i));
            }
        }
        self.dma_words(OFF_TX17, &mut snap.tx_desc);
        self.dma_words(OFF_BOUNCE, &mut snap.bounce);
        self.dma_words(OFF_RX_EVT, &mut snap.rx_desc);
        snap
    }

    fn dma_words(&self, off: usize, out: &mut [u32]) {
        for (i, word) in out.iter_mut().enumerate() {
            *word = dma_u32_read(self.dma, off + 4 * i);
        }
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn sta(&self) -> [u8; 6] {
        self.sta
    }

    /// Remember the pairwise key. Counters continue from the handshake.
    pub(crate) fn open_link(&mut self, bssid: [u8; 6], tk: [u8; 16]) {
        self.link = Some(Link {
            bssid,
            sta: self.sta,
            tk,
            pn: self.pn,
            seq: self.wifi_seq,
            quiet: 0,
        });
    }

    pub(crate) fn link(&mut self) -> Option<&mut Link> {
        self.link.as_mut()
    }

    fn start(&mut self, images: &Images) -> Result<(), Stage> {
        self.own()?;
        let chip = self.chip();
        let Some((patch, ram)) = images.pick(chip) else {
            return Err(Stage::FwChip);
        };
        self.wfsys()?;
        self.wr(HOST_INT_ENA, 0);
        self.wr(PCIE_MAC_INT, 0xff);
        self.dma_disable()?;
        unsafe { core::ptr::write_bytes(self.dma, 0, 0x4_0000) };
        let tx = self.tx.map(|ring| (ring.regs, ring.desc, ring.count));
        let rx = self.rx.map(|ring| (ring.regs, ring.desc, ring.bufs));
        for (regs, desc, count) in tx {
            self.program(regs, desc, count);
        }
        for (regs, desc, bufs) in rx {
            self.program(regs, desc, RING_N);
            self.fill_rx(desc, bufs, regs);
        }
        for ring in &mut self.rx {
            ring.head = RING_N - 1;
        }
        self.dma_enable();
        self.wr(SWDEF_MODE, 0);
        self.driver_own()?;
        // The pointer reset and driver-own run after the first programming.
        // Write the base again so the doorbell still names these rings.
        self.set(DMASHDL_SW, DMASHDL_BYPASS);
        self.rebind();
        self.set(PCIE_MAC_PM, L0S_DIS);
        self.before = core::hint::black_box(self.read_regs());
        self.load_patch(patch)?;
        self.load_ram(ram)?;
        if !self.poll(CONN_ON_MISC, FW_N9_RDY, FW_N9_RDY, N9_SPIN) {
            return Err(Stage::Mac);
        }
        self.n9 = true;
        self.mac_init(ram)
    }

    fn own(&mut self) -> Result<(), Stage> {
        if !self.own_bit(HOST_SET_OWN, HOST_OWN_SYNC) {
            return Err(Stage::Own);
        }
        if !self.own_bit(HOST_CLR_OWN, 0) {
            return Err(Stage::Own);
        }
        Ok(())
    }

    fn own_bit(&mut self, bit: u32, expect: u32) -> bool {
        for _ in 0..OWN_TRIES {
            self.wr(CONN_ON_LPCTL, bit);
            if self.poll(CONN_ON_LPCTL, HOST_OWN_SYNC, expect, OWN_SPIN) {
                return true;
            }
        }
        false
    }

    fn chip(&mut self) -> u32 {
        let id = self.rr(HW_CHIPID);
        let rev = self.rr(HW_REV);
        (id << 16) | (rev & 0xff)
    }

    fn wfsys(&mut self) -> Result<(), Stage> {
        self.clear(WFSYS_RST, WFSYS_RST_B);
        spin(WFSYS_GAP);
        self.set(WFSYS_RST, WFSYS_RST_B);
        if self.poll(WFSYS_RST, WFSYS_INIT_DONE, WFSYS_INIT_DONE, WFSYS_SPIN) {
            Ok(())
        } else {
            Err(Stage::Mac)
        }
    }

    fn dma_disable(&mut self) -> Result<(), Stage> {
        let bits = GLO_CFG_TX_EN
            | GLO_CFG_RX_EN
            | GLO_CFG_CHAIN
            | GLO_CFG_OMIT_TX
            | GLO_CFG_OMIT_RX
            | GLO_CFG_OMIT_RX_PFET;
        self.clear(GLO_CFG, bits);
        if !self.poll(GLO_CFG, GLO_CFG_TX_BUSY | GLO_CFG_RX_BUSY, 0, DMA_SPIN) {
            return Err(Stage::Mac);
        }
        self.clear(GLO_CFG_EXT0, DMASHDL_ENABLE);
        self.set(DMASHDL_SW, DMASHDL_BYPASS);
        self.clear(WFDMA_RST, RST_DMASHDL | RST_LOGIC);
        self.set(WFDMA_RST, RST_DMASHDL | RST_LOGIC);
        Ok(())
    }

    fn program(&mut self, regs: u32, desc: usize, count: u32) {
        for i in 0..count {
            self.desc_write(desc, i, 0, DMA_DONE, 0, 0);
        }
        self.publish(desc, usize_from(count) * 16);
        self.wr(regs + 8, 0);
        self.wr(regs + 12, 0);
        let phys = DMA_PHYS.wrapping_add(u32_from(desc));
        self.wr(regs, phys);
        self.wr(regs + 4, count);
    }

    fn fill_rx(&mut self, desc: usize, bufs: usize, regs: u32) {
        for i in 0..RING_N - 1 {
            let phys = DMA_PHYS.wrapping_add(u32_from(bufs)) + i * u32_from(BUF);
            self.desc_write(desc, i, phys, desc_ctrl(u32_from(BUF), false), 0, 0);
        }
        self.kick(regs, RING_N - 1);
        self.publish(desc, usize_from(RING_N) * 16);
    }

    fn rebind(&mut self) {
        for ring in self.tx {
            self.bind(ring.regs, ring.desc, ring.head, ring.count);
        }
        for ring in self.rx {
            self.bind(ring.regs, ring.desc, ring.head, RING_N);
        }
    }

    fn bind(&mut self, regs: u32, desc: usize, head: u32, count: u32) {
        let phys = DMA_PHYS.wrapping_add(u32_from(desc));
        self.wr(regs, phys);
        self.wr(regs + 4, count);
        self.wr(regs + 8, head);
    }

    fn dma_enable(&mut self) {
        for (addr, value) in prefetch() {
            self.wr(addr, value);
        }
        self.wr(RST_DTX_PTR, u32::MAX);
        // A sticky reset holds every transmit index at 0.
        self.wr(RST_DTX_PTR, 0);
        self.wr(PRI_DLY, 0);
        let cfg = GLO_CFG_TX_WB
            | GLO_CFG_LE
            | GLO_CFG_CLK
            | GLO_CFG_OMIT_TX
            | GLO_CFG_CHAIN
            | GLO_CFG_OMIT_RX_PFET;
        self.set(GLO_CFG, cfg);
        self.set(GLO_CFG, GLO_CFG_TX_EN | GLO_CFG_RX_EN);
        self.set(DUMMY_CR, NEED_REINIT);
        self.wr(HOST_INT_ENA, IRQ_MASK);
        self.set(MCU2HOST, MCU_WAKE);
    }

    fn driver_own(&mut self) -> Result<(), Stage> {
        self.wr(TOP_LPCR, LPCR_DRV_OWN);
        if self.poll(TOP_LPCR, LPCR_FW_OWN, 0, WFSYS_SPIN) {
            Ok(())
        } else {
            Err(Stage::Own)
        }
    }

    fn load_patch(&mut self, image: &[u8]) -> Result<(), Stage> {
        let status = self.sem(true).map_err(Stage::FwSem)?;
        if status == PATCH_IS_DL {
            return Ok(());
        }
        if status != PATCH_NOT_DL {
            return Err(Stage::FwSem(Sem::Status(status)));
        }
        let sent = self.transfer_patch(image);
        self.release_sem().and(sent)
    }

    fn transfer_patch(&mut self, image: &[u8]) -> Result<(), Stage> {
        let mut regions = [Region {
            addr: 0,
            off: 0,
            len: 0,
            mode: 0,
            skip: false,
            override_addr: false,
        }; 8];
        let Some(n) = patch_regions(image, &mut regions) else {
            return Err(Stage::FwPatch);
        };
        for region in regions.iter().take(n) {
            self.download(image, region, Stage::FwPatch)?;
        }
        if !self.command(PATCH_FINISH, &[0; 4], true) {
            return Err(Stage::FwPatch);
        }
        if patch_status(self.response()) != Some(0) {
            return Err(Stage::FwPatch);
        }
        Ok(())
    }

    fn release_sem(&mut self) -> Result<(), Stage> {
        match self.sem(false) {
            Ok(PATCH_REL) => Ok(()),
            Ok(status) => Err(Stage::FwSem(Sem::Status(status))),
            Err(why) => Err(Stage::FwSem(why)),
        }
    }

    fn load_ram(&mut self, image: &[u8]) -> Result<(), Stage> {
        let mut regions = [Region {
            addr: 0,
            off: 0,
            len: 0,
            mode: 0,
            skip: false,
            override_addr: false,
        }; 8];
        let Some((n, addr, option)) = ram_regions(image, &mut regions) else {
            return Err(Stage::FwRam);
        };
        for region in regions.iter().take(n) {
            if region.skip {
                continue;
            }
            self.download(image, region, Stage::FwRam)?;
        }
        let mut req = [0u8; 8];
        req[..4].copy_from_slice(&option.to_le_bytes());
        req[4..].copy_from_slice(&addr.to_le_bytes());
        if self.command(FW_START, &req, true) {
            Ok(())
        } else {
            Err(Stage::FwStart)
        }
    }

    fn download(&mut self, image: &[u8], region: &Region, setup: Stage) -> Result<(), Stage> {
        let mut req = [0u8; 12];
        req[..4].copy_from_slice(&region.addr.to_le_bytes());
        req[4..8].copy_from_slice(&u32_from(region.len).to_le_bytes());
        req[8..].copy_from_slice(&region.mode.to_le_bytes());
        if !self.command(download_cmd(region.addr), &req, true) {
            return Err(setup);
        }
        let mut off = 0;
        while off < region.len {
            let n = (region.len - off).min(BOUNCE);
            let chunk = image
                .get(region.off + off..region.off + off + n)
                .ok_or(setup)?;
            if !self.scatter(chunk) {
                return Err(Stage::FwScatter);
            }
            off += n;
        }
        Ok(())
    }

    fn sem(&mut self, get: bool) -> Result<u8, Sem> {
        let op = u32::from(get).to_le_bytes();
        self.exchange(PATCH_SEM, &op)?;
        patch_status(self.response()).ok_or(Sem::Rx)
    }

    fn mac_init(&mut self, ram: &[u8]) -> Result<(), Stage> {
        self.nic_cap()?;
        self.send_clc(ram)?;
        self.fw_log()?;
        let mut body = [0u8; 16];
        write_eeprom_mode(&mut body);
        if !self.command(ext_cmd(0x21), &body[..4], true) {
            return Err(Stage::Mac);
        }
        self.rmw_len(MDP_DCR1, 1536);
        self.set(MDP_DCR0, MDP_DAMSDU | MDP_HDR_TRANS);
        for i in 0..20 {
            self.wr(WTBL_UPDATE, wtbl_word(i));
            let _ = self.poll(WTBL_UPDATE, WTBL_BUSY, 0, DMA_SPIN);
        }
        for band in 0..2 {
            for op in band_init_ops(band) {
                self.apply(op);
            }
        }
        write_rts(&mut body);
        if !self.command(ext_cmd(0x3e), &body[..12], true) {
            return Err(Stage::Mac);
        }
        write_mac_enable(&mut body);
        if !self.command(ext_cmd(0x46), &body[..4], true) {
            return Err(Stage::Mac);
        }
        // `__mt7921_start` sends the channel domain, then `SET_RX_PATH`.
        if !self.channel_domain() || !self.set_channel(1, 0) {
            return Err(Stage::Mac);
        }
        write_efuse_read(&mut body);
        if self.command(ext_query(0x01), &body[..4], true)
            && let Some(mac) = efuse_mac(self.response())
        {
            self.sta = mac;
        }
        // The boot that reached `wait` and then printed `wifi err` had posted
        // the rate-power table and `KeepFullPwr 0` here, with no reply. The
        // survey is the next command that waits.
        Ok(())
    }

    fn nic_cap(&mut self) -> Result<(), Stage> {
        if !self.command(ce_cmd(0x8a), &[], true) {
            return Err(Stage::Mac);
        }
        let Some(cap) = parse_nic_cap(self.response()) else {
            return Err(Stage::Mac);
        };
        self.antenna = cap.antenna_mask;
        self.nic = u8::from(cap.has_6g) | (u8::from(cap.clc_event) << 1);
        if let Some(mac) = cap.mac
            && mac != [0; 6]
            && mac != [0xff; 6]
        {
            self.sta = mac;
        }
        Ok(())
    }

    fn send_clc(&mut self, ram: &[u8]) -> Result<(), Stage> {
        let mut body = [0u8; EFUSE_BLOCK_LEN];
        if !write_efuse_block(&mut body, EE_HW_TYPE) || !self.command(ext_query(0x01), &body, true)
        {
            return Err(Stage::Mac);
        }
        let Some(hw) = efuse_block_byte(self.response(), EE_HW_TYPE) else {
            return Err(Stage::Mac);
        };
        let Some(region) = clc_image(ram) else {
            return Ok(());
        };
        let event = self.nic & 2 != 0;
        let ok = each_world_rule(region, hw & 1, |idx, alpha, kind, data| {
            let mut cmd = [0u8; 2048];
            let Some(n) = write_clc(&mut cmd, idx, alpha, kind, data, event) else {
                return false;
            };
            self.command(ce_cmd(0x5c), &cmd[..n], event)
        });
        if ok {
            Ok(())
        } else {
            Err(Stage::Mac)
        }
    }

    fn fw_log(&mut self) -> Result<(), Stage> {
        let mut body = [0u8; 4];
        if write_fw_log(&mut body) && self.command(ce_cmd(0xc5), &body, false) {
            Ok(())
        } else {
            Err(Stage::Mac)
        }
    }

    fn apply(&mut self, op: RegOp) {
        match op {
            RegOp::Or(addr, bits) => self.set(addr, bits),
            RegOp::Clear(addr, bits) => self.clear(addr, bits),
            RegOp::RxLen(addr, value) => self.rmw_len(addr, value),
            RegOp::Field(addr, mask, value) => {
                let cur = self.rr(addr);
                self.wr(addr, (cur & !mask) | (value & mask));
            }
        }
    }

    pub(crate) fn set_channel(&mut self, channel: u8, band: u8) -> bool {
        let mut rx_path = [0u8; 76];
        let mut switch = [0u8; 76];
        if !write_chan(&mut rx_path, channel, band, self.antenna, true)
            || !write_chan(&mut switch, channel, band, self.antenna, false)
        {
            return false;
        }
        self.command(ext_cmd(0x4e), &rx_path, true) && self.command(ext_cmd(0x08), &switch, true)
    }

    pub(crate) fn command(&mut self, cmd: u32, payload: &[u8], wait: bool) -> bool {
        if cmd & 0xff == FW_SCATTER {
            return self.scatter(payload);
        }
        if !wait {
            return self.post_mcu(cmd, payload);
        }
        self.exchange(cmd, payload).is_ok()
    }

    fn exchange(&mut self, cmd: u32, payload: &[u8]) -> Result<(), Sem> {
        if !self.post_mcu(cmd, payload) {
            return Err(Sem::Tx(self.tx_stop(TX_MCU)));
        }
        let seq = self.seq;
        if !self.wait_seq(seq) {
            return Err(Sem::Rx);
        }
        if is_uni_cmd(cmd) && !uni_event_ok(self.response(), (cmd & 0xff) as u8) {
            return Err(Sem::Rx);
        }
        Ok(())
    }

    fn post_mcu(&mut self, cmd: u32, payload: &[u8]) -> bool {
        let seq = self.next_seq();
        let hdr = mcu_header_len(cmd);
        let total = hdr + payload.len();
        if total > BOUNCE {
            return false;
        }
        {
            let cmd_buf = &mut scratch().cmd;
            if !write_mcu_header(cmd_buf, cmd, seq, payload.len()) {
                return false;
            }
            cmd_buf[hdr..total].copy_from_slice(payload);
        }
        crate::step_cmd();
        self.tx_bounce(TX_MCU, total)
    }

    fn scatter(&mut self, chunk: &[u8]) -> bool {
        if chunk.len() > BOUNCE {
            return false;
        }
        dma_write(self.dma, OFF_BOUNCE, chunk);
        self.post_tx(TX_FWDL, DMA_PHYS + u32_from(OFF_BOUNCE), chunk.len())
    }

    fn tx_bounce(&mut self, ring: usize, len: usize) -> bool {
        dma_write(self.dma, OFF_BOUNCE, &scratch().cmd[..len]);
        self.post_tx(ring, DMA_PHYS + u32_from(OFF_BOUNCE), len)
    }

    fn post_tx(&mut self, ring: usize, phys: u32, len: usize) -> bool {
        let head = self.tx[ring].head;
        let desc = self.tx[ring].desc;
        let regs = self.tx[ring].regs;
        self.desc_write(desc, head, phys, desc_ctrl(u32_from(len), true), 0, 0);
        if ring == TX_MCU {
            crate::step_desc();
        }
        self.publish(desc + head as usize * 16, 16);
        self.publish(phys.wrapping_sub(DMA_PHYS) as usize, len);
        let next = (head + 1) % self.tx[ring].count;
        self.tx[ring].head = next;
        self.tx[ring].posted = head;
        let index = self.dma_idx(regs);
        self.kick(regs, next);
        if ring == TX_MCU {
            crate::step_bell();
        }
        // `mt76_dma_tx_cleanup` waits on the DMA index. The DDONE bit is the RX
        // signal; `OMIT_TX_INFO` does not have to write it back.
        self.wait_done(desc, head, regs, index, MCU_SPIN)
    }

    fn wait_done(&mut self, desc: usize, idx: u32, regs: u32, index: u8, spins: u32) -> bool {
        if regs == self.tx[TX_MCU].regs {
            crate::step_fetch();
        }
        for spin in 0..spins {
            // A write-back line filled by an earlier read hides the device's store.
            if spin.is_multiple_of(64) {
                self.publish(desc + idx as usize * 16, 16);
            }
            if self.desc_word(desc, idx, 1) & DMA_DONE != 0 || self.dma_idx(regs) != index {
                return true;
            }
            core::hint::spin_loop();
        }
        false
    }

    fn dma_idx(&mut self, regs: u32) -> u8 {
        u8::try_from(self.rr(regs + 12) & 0xff).expect("dma index")
    }

    fn tx_stop(&mut self, ring: usize) -> TxStop {
        let regs = self.tx[ring].regs;
        let posted = self.tx[ring].posted;
        let desc = self.tx[ring].desc;
        self.publish(desc + posted as usize * 16, 16);
        let ctrl = self.desc_word(desc, posted, 1);
        TxStop {
            dma: self.dma_idx(regs),
            cpu: u8::try_from(self.rr(regs + 8) & 0xff).expect("cpu index"),
            size: u16::try_from(self.rr(regs + 4) & 0xffff).expect("ring size"),
            base: u16::try_from(self.rr(regs) & 0xffff).expect("ring base"),
            cfg: u8::try_from(self.rr(GLO_CFG) & 0xff).expect("dma config"),
            slot: u8::try_from(posted & 0xff).expect("slot"),
            buf: self.desc_word(desc, posted, 0),
            done: ctrl & DMA_DONE != 0,
        }
    }

    fn quiesce(&mut self) {
        self.clear(GLO_CFG, GLO_CFG_TX_EN | GLO_CFG_RX_EN);
    }

    fn wait_seq(&mut self, seq: u8) -> bool {
        for _ in 0..MCU_SPIN {
            crate::beat();
            if let Some(n) = self.poll_mcu()
                && n >= RXD_LEN
                && rx_byte(29) == seq
            {
                self.last_len = n;
                return true;
            }
            core::hint::spin_loop();
        }
        false
    }

    pub(crate) fn poll_mcu(&mut self) -> Option<usize> {
        if self.n9 {
            self.poll_rx(RX_WA).or_else(|| self.poll_rx(RX_EVT))
        } else {
            self.poll_rx(RX_EVT)
        }
    }

    pub(crate) fn poll_data(&mut self) -> Option<usize> {
        self.poll_rx(RX_DATA)
    }

    pub(crate) fn rx_bytes(n: usize) -> &'static [u8] {
        rx_slice(n.min(BUF))
    }

    /// Active scan. An empty name is the wildcard survey. The completion is
    /// `MCU_EVENT_SCAN_DONE`, not a command ack.
    ///
    /// The station is registered once, then the channel domain goes out,
    /// then the mode 1 receive filter, then the other-beacon drop is cleared.
    ///
    /// A boot that printed `rx 0 ok 0 done 1 ch 21 bcn 14` had the DMA length
    /// and the mode 1 filter. The firmware counted the beacons and copied none.
    /// `mt7921_mcu_set_beacon_filter(false)` clears that drop for an
    /// unassociated station.
    pub(crate) fn start_scan(&mut self, ssid: &[u8]) -> bool {
        if !self.add_station()
            || !self.channel_domain()
            || !self.rx_filter()
            || !self.pass_beacons()
        {
            return false;
        }
        let seq = self.next_seq();
        {
            let cmd = &mut scratch().cmd;
            let payload = MCU_TXD_LEN + SCAN_REQ_LEN;
            if !write_scan(&mut cmd[MCU_TXD_LEN..payload], ssid, seq) {
                return false;
            }
            if !write_mcu_txd(&mut cmd[..MCU_TXD_LEN], ce_cmd(0x03), seq, SCAN_REQ_LEN) {
                return false;
            }
        }
        self.tx_bounce(TX_MCU, MCU_TXD_LEN + SCAN_REQ_LEN)
    }

    /// `mt76_connac_mcu_uni_add_dev` for one station. Both replies must
    /// report status 0. A later scan keeps the registration.
    fn add_station(&mut self) -> bool {
        if self.bss {
            return true;
        }
        let mut dev = [0u8; DEV_INFO_LEN];
        let mut bss = [0u8; BSS_INFO_LEN];
        if !write_dev_info(&mut dev, &self.sta) || !write_bss_info(&mut bss) {
            return false;
        }
        if !self.command(uni_cmd(0x01), &dev, true) || !self.command(uni_cmd(0x02), &bss, true) {
            return false;
        }
        self.bss = true;
        self.wr(WTBL_UPDATE, wtbl_word(WTBL_RESERVED));
        let _ = self.poll(WTBL_UPDATE, WTBL_BUSY, 0, DMA_SPIN);
        true
    }

    /// `mt7921_mac_sta_add` for the access point named by the beacon.
    /// The authentication frame uses `PEER_WTBL`.
    pub(crate) fn add_peer(&mut self, bssid: [u8; 6], band5: bool) -> bool {
        self.wr(WTBL_UPDATE, wtbl_word(PEER_WTBL));
        let _ = self.poll(WTBL_UPDATE, WTBL_BUSY, 0, DMA_SPIN);
        let mut body = [0u8; STA_REC_LEN];
        let Some(n) = write_sta_rec(&mut body, &bssid, band5) else {
            return false;
        };
        self.command(uni_cmd(0x03), &body[..n], true)
    }

    /// `mt7921_configure_filter` before a scan. Linux does not wait.
    fn rx_filter(&mut self) -> bool {
        let mut body = [0u8; RX_FILTER_LEN];
        if !write_rx_filter_enable(&mut body) {
            return false;
        }
        self.command(ce_cmd(0x0a), &body, false)
    }

    /// `mt7921_mcu_set_beacon_filter(false)`. Linux does not wait.
    fn pass_beacons(&mut self) -> bool {
        let mut body = [0u8; RX_FILTER_LEN];
        if !write_clear_other_beacon(&mut body) {
            return false;
        }
        self.command(ce_cmd(0x0a), &body, false)
    }

    /// `MCU_CE_CMD(SET_CHAN_DOMAIN)`. Linux sends this from `__mt7921_start`
    /// and does not wait for a reply.
    fn channel_domain(&mut self) -> bool {
        let mut body = [0u8; CHANNEL_DOMAIN_LEN];
        let Some(n) = write_channel_domain(&mut body) else {
            return false;
        };
        self.command(ce_cmd(0x0f), &body[..n], false)
    }

    fn poll_rx(&mut self, which: usize) -> Option<usize> {
        let tail = self.rx[which].tail;
        let desc = self.rx[which].desc;
        let ctrl = self.desc_word(desc, tail, 1);
        if ctrl & DMA_DONE == 0 {
            return None;
        }
        let len = desc_len(ctrl);
        let phys = self.desc_word(desc, tail, 0);
        let off = phys.wrapping_sub(DMA_PHYS) as usize;
        let n = len.min(BUF);
        dma_read(self.dma, off, &mut scratch().rx[..n]);
        let head = self.rx[which].head;
        let regs = self.rx[which].regs;
        self.desc_write(desc, head, phys, desc_ctrl(u32_from(BUF), false), 0, 0);
        self.rx[which].head = (head + 1) % RING_N;
        self.rx[which].tail = (tail + 1) % RING_N;
        self.kick(regs, self.rx[which].head);
        Some(n)
    }

    pub(crate) fn tx_80211(&mut self, frame: &[u8]) -> bool {
        if frame.len() + TXD_LEN + TXP_LEN > BUF {
            return false;
        }
        let phys = DMA_PHYS + u32_from(OFF_TX_FRAME);
        {
            let txwi = &mut scratch().cmd[..TXD_LEN + TXP_LEN];
            if !write_txwi(txwi, frame, phys) {
                return false;
            }
        }
        dma_write(self.dma, OFF_TXWI, &scratch().cmd[..TXD_LEN + TXP_LEN]);
        dma_write(self.dma, OFF_TX_FRAME, frame);
        self.post_tx(TX_DATA, DMA_PHYS + u32_from(OFF_TXWI), TXD_LEN + TXP_LEN)
    }

    pub(crate) fn next_seq_wifi(&mut self) -> u16 {
        let seq = self.wifi_seq;
        self.wifi_seq = self.wifi_seq.wrapping_add(1);
        seq
    }

    fn response(&self) -> &'static [u8] {
        rx_slice(self.last_len)
    }

    fn next_seq(&mut self) -> u8 {
        self.seq = (self.seq + 1) & 0xf;
        if self.seq == 0 {
            self.seq = 1;
        }
        self.seq
    }

    fn rmw_len(&mut self, addr: u32, value: u32) {
        let mask = 0xfff8;
        let cur = self.rr(addr);
        self.wr(addr, (cur & !mask) | ((value << 3) & mask));
    }

    fn poll(&mut self, addr: u32, mask: u32, expect: u32, spins: u32) -> bool {
        for _ in 0..spins {
            if self.rr(addr) & mask == expect {
                return true;
            }
            core::hint::spin_loop();
        }
        false
    }

    fn set(&mut self, addr: u32, bits: u32) {
        let value = self.rr(addr) | bits;
        self.wr(addr, value);
    }

    fn clear(&mut self, addr: u32, bits: u32) {
        let value = self.rr(addr) & !bits;
        self.wr(addr, value);
    }

    fn rr(&mut self, addr: u32) -> u32 {
        crate::beat();
        match map_reg(addr) {
            Map::Direct(off) => rd(self.regs, off),
            Map::Remap { base, window } => {
                self.arm_remap(base);
                rd(self.regs, window)
            }
            Map::Unsupported => u32::MAX,
        }
    }

    fn wr(&mut self, addr: u32, value: u32) {
        match map_reg(addr) {
            Map::Direct(off) => wr(self.regs, off, value),
            Map::Remap { base, window } => {
                self.arm_remap(base);
                wr(self.regs, window, value);
            }
            Map::Unsupported => {}
        }
    }

    /// Linux reads `fe24c`, replaces bits 15:0 with the page, and reads the
    /// register back before touching the window. The read-back is what posts
    /// the write. Bits 31:16 stay as the card already holds them.
    fn arm_remap(&mut self, page: u32) {
        let current = rd(self.regs, HIF_REMAP_L1);
        wr(self.regs, HIF_REMAP_L1, remap_l1(current, page));
        let _posted = rd(self.regs, HIF_REMAP_L1);
    }

    fn kick(&mut self, regs: u32, head: u32) {
        fence(Ordering::SeqCst);
        self.wr(regs + 8, head);
    }

    /// The control word is last, matching `mt76_dma_add_buf`, then the line is
    /// flushed so the device cannot fetch the previous `DMA_DONE` value.
    fn desc_write(&mut self, desc: usize, idx: u32, buf0: u32, ctrl: u32, buf1: u32, info: u32) {
        let off = desc + idx as usize * 16;
        dma_u32(self.dma, off, buf0);
        dma_u32(self.dma, off + 8, buf1);
        dma_u32(self.dma, off + 12, info);
        dma_u32(self.dma, off + 4, ctrl);
    }

    fn publish(&self, off: usize, len: usize) {
        if len == 0 {
            return;
        }
        unsafe {
            let start = self.dma.add(off) as usize;
            let mut line = start & !63;
            let end = start + len;
            while line < end {
                core::arch::x86_64::_mm_clflush(line as *const u8);
                line += 64;
            }
            core::arch::x86_64::_mm_sfence();
        }
    }

    #[expect(clippy::cast_ptr_alignment, reason = "4-byte DMA descriptor")]
    fn desc_word(&self, desc: usize, idx: u32, word: usize) -> u32 {
        let off = desc + idx as usize * 16 + word * 4;
        unsafe { self.dma.add(off).cast::<u32>().read_volatile() }
    }
}

/// Both firmware pairs. v6.8 uses the non-`1a` image except on 0x7922 and 0x7925.
pub struct Images {
    pub patch: &'static [u8],
    pub patch_1a: &'static [u8],
    pub ram: &'static [u8],
    pub ram_1a: &'static [u8],
}

impl Images {
    fn pick(&self, rev: u32) -> Option<(&'static [u8], &'static [u8])> {
        let chip = rev >> 16;
        if firmware_kind(chip) != FirmwareKind::Mt7961 {
            return None;
        }
        let patch = if self.patch.is_empty() {
            self.patch_1a
        } else {
            self.patch
        };
        let ram = if self.ram.is_empty() {
            self.ram_1a
        } else {
            self.ram
        };
        if patch.is_empty() || ram.is_empty() {
            None
        } else {
            Some((patch, ram))
        }
    }

    pub fn any(&self) -> bool {
        (!self.patch.is_empty() || !self.patch_1a.is_empty())
            && (!self.ram.is_empty() || !self.ram_1a.is_empty())
    }
}

pub(crate) fn scan_budget() -> u32 {
    SCAN_SPIN
}

pub(crate) fn drain_budget() -> u32 {
    DRAIN_SPIN
}

pub(crate) fn assoc_budget() -> u32 {
    ASSOC_SPIN
}

fn spin(n: u32) {
    for _ in 0..n {
        crate::beat();
        core::hint::spin_loop();
    }
}

fn u32_from(value: usize) -> u32 {
    u32::try_from(value).expect("length fits")
}

fn usize_from(value: u32) -> usize {
    usize::try_from(value).expect("count fits")
}

fn tx_rings() -> [TxRing; 3] {
    [
        TxRing {
            regs: ring_regs(TX_RING_BASE, 0),
            desc: OFF_TX0,
            count: RING_N,
            head: 0,
            posted: 0,
        },
        TxRing {
            regs: ring_regs(TX_RING_BASE, 16),
            desc: OFF_TX16,
            count: RING_N,
            head: 0,
            posted: 0,
        },
        TxRing {
            regs: ring_regs(TX_RING_BASE, 17),
            desc: OFF_TX17,
            count: MCU_N,
            head: 0,
            posted: 0,
        },
    ]
}

fn rx_rings() -> [RxRing; 3] {
    [
        RxRing {
            regs: ring_regs(RX_EVENT_BASE, 0),
            desc: OFF_RX_EVT,
            bufs: OFF_RX0_BUFS,
            head: 0,
            tail: 0,
        },
        RxRing {
            regs: ring_regs(RX_DATA_BASE, 0),
            desc: OFF_RX_DATA,
            bufs: OFF_RX_DATA_BUFS,
            head: 0,
            tail: 0,
        },
        RxRing {
            regs: ring_regs(RX_WA_BASE, 0),
            desc: OFF_RX_WA,
            bufs: OFF_RX_WA_BUFS,
            head: 0,
            tail: 0,
        },
    ]
}

#[expect(clippy::cast_ptr_alignment, reason = "4-byte MMIO register")]
fn rd(regs: *mut u8, off: u32) -> u32 {
    unsafe { regs.add(off as usize).cast::<u32>().read_volatile() }
}

#[expect(clippy::cast_ptr_alignment, reason = "4-byte MMIO register")]
fn wr(regs: *mut u8, off: u32, value: u32) {
    unsafe { regs.add(off as usize).cast::<u32>().write_volatile(value) }
}

#[expect(clippy::cast_ptr_alignment, reason = "4-byte DMA descriptor")]
fn dma_u32(dma: *mut u8, off: usize, value: u32) {
    unsafe { dma.add(off).cast::<u32>().write_volatile(value) }
}

#[expect(clippy::cast_ptr_alignment, reason = "4-byte DMA descriptor")]
fn dma_u32_read(dma: *mut u8, off: usize) -> u32 {
    unsafe { dma.add(off).cast::<u32>().read_volatile() }
}

fn dma_write(dma: *mut u8, off: usize, bytes: &[u8]) {
    unsafe {
        let dst = dma.add(off);
        for (i, byte) in bytes.iter().enumerate() {
            dst.add(i).write_volatile(*byte);
        }
    }
}

fn dma_read(dma: *mut u8, off: usize, out: &mut [u8]) {
    unsafe {
        let src = dma.add(off);
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = src.add(i).read_volatile();
        }
    }
}

const RX_SPAN: usize = (RING_N as usize - 1) * BUF;
const _: () = assert!(MCU_N * 16 == 0x1000);
const _: () = assert!(OFF_RX_EVT == OFF_TX17 + 0x1000);
const _: () = assert!(OFF_TX17.trailing_zeros() >= 12);
const _: () = assert!(OFF_RX0_BUFS >= OFF_BOUNCE + BOUNCE);
const _: () = assert!(OFF_RX_DATA_BUFS >= OFF_RX0_BUFS + RX_SPAN);
const _: () = assert!(OFF_RX_WA_BUFS >= OFF_RX_DATA_BUFS + RX_SPAN);
const _: () = assert!(OFF_TXWI >= OFF_RX_WA_BUFS + RX_SPAN);
const _: () = assert!(OFF_TX_FRAME + BUF < 0x4_0000);
const _: () = assert!(LAST_SEC0 == 1 << 30);
