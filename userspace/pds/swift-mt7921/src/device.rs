//! MT7921 MAC from Linux v6.8 `mt7921/pci.c`, `mt792x_dma.c`, and `mt76_connac_mcu.c`.
//!
//! The DMA window is physical `0x1f60000`. Rings are eight descriptors because
//! that window is 256 KiB. One transmit is in flight, so the bounce buffer is
//! reused only after `DMA_DONE`.

use core::sync::atomic::{fence, Ordering};

use lerux_swift::{
    mt7921::{
        ce_cmd, desc_ctrl, download_cmd, efuse_mac, ext_cmd, ext_query, firmware_kind, map_reg,
        patch_regions, patch_status, prefetch, ram_regions, ring_regs, write_chan,
        write_eeprom_mode, write_efuse_read, write_mac_enable, write_mcu_txd, write_rts,
        write_scan, write_txwi, wtbl_word, FirmwareKind, Map, Region, Stage, CONN_ON_LPCTL,
        CONN_ON_MISC, DMASHDL_BYPASS, DMASHDL_ENABLE, DMASHDL_SW, DMA_DONE, DUMMY_CR, FW_N9_RDY,
        FW_SCATTER, FW_START, GLO_CFG, GLO_CFG_CHAIN, GLO_CFG_CLK, GLO_CFG_EXT0, GLO_CFG_LE,
        GLO_CFG_OMIT_RX, GLO_CFG_OMIT_RX_PFET, GLO_CFG_OMIT_TX, GLO_CFG_RX_BUSY, GLO_CFG_RX_EN,
        GLO_CFG_TX_BUSY, GLO_CFG_TX_EN, GLO_CFG_TX_WB, HIF_REMAP_L1, HOST_CLR_OWN, HOST_INT_ENA,
        HOST_OWN_SYNC, HOST_SET_OWN, HW_CHIPID, HW_REV, IRQ_MASK, L0S_DIS, LAST_SEC0, LPCR_DRV_OWN,
        LPCR_FW_OWN, MCU2HOST, MCU_TXD_LEN, MCU_WAKE, MDP_DAMSDU, MDP_DCR0, MDP_DCR1,
        MDP_HDR_TRANS, NEED_REINIT, PATCH_FINISH, PATCH_IS_DL, PATCH_NOT_DL, PATCH_REL, PATCH_SEM,
        PCIE_MAC_INT, PCIE_MAC_PM, PRI_DLY, RST_DMASHDL, RST_DTX_PTR, RST_LOGIC, RXD_LEN,
        RX_DATA_BASE, RX_EVENT_BASE, RX_WA_BASE, SCAN_REQ_LEN, SWDEF_MODE, TOP_LPCR, TXD_LEN,
        TXP_LEN, TX_RING_BASE, WFDMA_RST, WFSYS_INIT_DONE, WFSYS_RST, WFSYS_RST_B, WTBL_BUSY,
        WTBL_UPDATE,
    },
    CLIENT_MAC,
};

const DMA_PHYS: u32 = 0x01f6_0000;
const RING_N: u32 = 8;
const BUF: usize = 2048;
const BOUNCE: usize = 4096;

const OFF_TX0: usize = 0x0000;
const OFF_TX16: usize = 0x0100;
const OFF_TX17: usize = 0x0200;
const OFF_RX_EVT: usize = 0x0300;
const OFF_RX_DATA: usize = 0x0400;
const OFF_RX_WA: usize = 0x0500;
const OFF_BOUNCE: usize = 0x1000;
const OFF_RX0_BUFS: usize = 0x2000;
const OFF_RX_DATA_BUFS: usize = 0x5800;
const OFF_RX_WA_BUFS: usize = 0x9000;
const OFF_TXWI: usize = 0xd000;
const OFF_TX_FRAME: usize = 0xd100;

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
const ASSOC_SPIN: u32 = 40_000_000;

#[derive(Clone, Copy)]
struct TxRing {
    regs: u32,
    desc: usize,
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
    last_len: usize,
    pn: u64,
    wifi_seq: u16,
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
    pub fn boot(regs: *mut u8, dma: *mut u8, images: Images) -> Self {
        let mut mac = Self {
            regs,
            dma,
            tx: [
                TxRing {
                    regs: ring_regs(TX_RING_BASE, 0),
                    desc: OFF_TX0,
                    head: 0,
                    posted: 0,
                },
                TxRing {
                    regs: ring_regs(TX_RING_BASE, 16),
                    desc: OFF_TX16,
                    head: 0,
                    posted: 0,
                },
                TxRing {
                    regs: ring_regs(TX_RING_BASE, 17),
                    desc: OFF_TX17,
                    head: 0,
                    posted: 0,
                },
            ],
            rx: [
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
            ],
            seq: 0,
            n9: false,
            ready: false,
            stage: Stage::Own,
            sta: CLIENT_MAC,
            last_len: 0,
            pn: 1,
            wifi_seq: 1,
        };
        mac.stage = match mac.start(images) {
            Ok(()) => Stage::Wait,
            Err(stage) => stage,
        };
        mac.ready = mac.stage == Stage::Wait;
        mac
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn sta(&self) -> [u8; 6] {
        self.sta
    }

    fn start(&mut self, images: Images) -> Result<(), Stage> {
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
        let tx = self.tx.map(|ring| (ring.regs, ring.desc));
        let rx = self.rx.map(|ring| (ring.regs, ring.desc, ring.bufs));
        for (regs, desc) in tx {
            self.program(regs, desc);
        }
        for (regs, desc, bufs) in rx {
            self.program(regs, desc);
            self.fill_rx(desc, bufs, regs);
        }
        for ring in &mut self.rx {
            ring.head = RING_N - 1;
        }
        self.dma_enable()?;
        self.wr(SWDEF_MODE, 0);
        self.driver_own()?;
        self.set(PCIE_MAC_PM, L0S_DIS);
        self.load_patch(patch)?;
        self.load_ram(ram)?;
        if !self.poll(CONN_ON_MISC, FW_N9_RDY, FW_N9_RDY, N9_SPIN) {
            return Err(Stage::Mac);
        }
        self.n9 = true;
        self.mac_init()
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

    fn program(&mut self, regs: u32, desc: usize) {
        for i in 0..RING_N {
            self.desc_write(desc, i, 0, DMA_DONE, 0, 0);
        }
        self.wr(regs + 8, 0);
        self.wr(regs + 12, 0);
        let phys = DMA_PHYS.wrapping_add(desc as u32);
        self.wr(regs, phys);
        self.wr(regs + 4, RING_N);
    }

    fn fill_rx(&mut self, desc: usize, bufs: usize, regs: u32) {
        for i in 0..RING_N - 1 {
            let phys = DMA_PHYS.wrapping_add(bufs as u32) + i * BUF as u32;
            self.desc_write(desc, i, phys, desc_ctrl(BUF as u32, false), 0, 0);
        }
        self.kick(regs, RING_N - 1);
    }

    fn dma_enable(&mut self) -> Result<(), Stage> {
        for (addr, value) in prefetch() {
            self.wr(addr, value);
        }
        self.wr(RST_DTX_PTR, u32::MAX);
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
        Ok(())
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
        let status = self.sem(true).ok_or(Stage::FwSem)?;
        if status == PATCH_IS_DL {
            return Ok(());
        }
        if status != PATCH_NOT_DL {
            return Err(Stage::FwSem);
        }
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
        if self.sem(false) != Some(PATCH_REL) {
            return Err(Stage::FwSem);
        }
        Ok(())
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
        req[4..8].copy_from_slice(&(region.len as u32).to_le_bytes());
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

    fn sem(&mut self, get: bool) -> Option<u8> {
        let op = u32::from(get).to_le_bytes();
        if !self.command(PATCH_SEM, &op, true) {
            return None;
        }
        patch_status(self.response())
    }

    fn mac_init(&mut self) -> Result<(), Stage> {
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
        write_rts(&mut body);
        if !self.command(ext_cmd(0x3e), &body[..12], true) {
            return Err(Stage::Mac);
        }
        write_mac_enable(&mut body);
        if !self.command(ext_cmd(0x46), &body[..4], true) {
            return Err(Stage::Mac);
        }
        if !self.set_channel(1, 0) {
            return Err(Stage::Mac);
        }
        write_efuse_read(&mut body);
        if self.command(ext_query(0x01), &body[..4], true)
            && let Some(mac) = efuse_mac(self.response())
        {
            self.sta = mac;
        }
        Ok(())
    }

    pub(crate) fn set_channel(&mut self, channel: u8, band: u8) -> bool {
        let mut body = [0u8; 76];
        if !write_chan(&mut body, channel, band) {
            return false;
        }
        self.command(ext_cmd(0x4e), &body, true) && self.command(ext_cmd(0x08), &body, true)
    }

    pub(crate) fn command(&mut self, cmd: u32, payload: &[u8], wait: bool) -> bool {
        if cmd & 0xff == FW_SCATTER {
            return self.scatter(payload);
        }
        let seq = self.next_seq();
        let total = MCU_TXD_LEN + payload.len();
        if total > BOUNCE {
            return false;
        }
        {
            let cmd_buf = &mut scratch().cmd;
            if !write_mcu_txd(cmd_buf, cmd, seq, payload.len()) {
                return false;
            }
            cmd_buf[MCU_TXD_LEN..total].copy_from_slice(payload);
        }
        if !self.tx_bounce(TX_MCU, total) {
            return false;
        }
        if !wait {
            return true;
        }
        self.wait_seq(seq)
    }

    fn scatter(&mut self, chunk: &[u8]) -> bool {
        if chunk.len() > BOUNCE {
            return false;
        }
        dma_write(self.dma, OFF_BOUNCE, chunk);
        self.post_tx(TX_FWDL, DMA_PHYS + OFF_BOUNCE as u32, chunk.len())
    }

    fn tx_bounce(&mut self, ring: usize, len: usize) -> bool {
        dma_write(self.dma, OFF_BOUNCE, &scratch().cmd[..len]);
        self.post_tx(ring, DMA_PHYS + OFF_BOUNCE as u32, len)
    }

    fn post_tx(&mut self, ring: usize, phys: u32, len: usize) -> bool {
        let head = self.tx[ring].head;
        let desc = self.tx[ring].desc;
        let regs = self.tx[ring].regs;
        self.desc_write(desc, head, phys, desc_ctrl(len as u32, true), 0, 0);
        let next = (head + 1) % RING_N;
        self.tx[ring].head = next;
        self.tx[ring].posted = head;
        self.kick(regs, next);
        self.wait_done(desc, head, MCU_SPIN)
    }

    fn wait_done(&mut self, desc: usize, idx: u32, spins: u32) -> bool {
        for _ in 0..spins {
            if self.desc_word(desc, idx, 1) & DMA_DONE != 0 {
                return true;
            }
            core::hint::spin_loop();
        }
        false
    }

    fn wait_seq(&mut self, seq: u8) -> bool {
        for _ in 0..MCU_SPIN {
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

    pub(crate) fn rx_bytes(&self, n: usize) -> &'static [u8] {
        rx_slice(n.min(BUF))
    }

    /// Active scan. The completion is `MCU_EVENT_SCAN_DONE`, not a command ack.
    pub(crate) fn start_scan(&mut self, ssid: &[u8]) -> bool {
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

    fn poll_rx(&mut self, which: usize) -> Option<usize> {
        let tail = self.rx[which].tail;
        let desc = self.rx[which].desc;
        let ctrl = self.desc_word(desc, tail, 1);
        if ctrl & DMA_DONE == 0 {
            return None;
        }
        let len = (ctrl & 0x3fff) as usize;
        let phys = self.desc_word(desc, tail, 0);
        let off = phys.wrapping_sub(DMA_PHYS) as usize;
        let n = len.min(BUF);
        dma_read(self.dma, off, &mut scratch().rx[..n]);
        let head = self.rx[which].head;
        let regs = self.rx[which].regs;
        self.desc_write(desc, head, phys, desc_ctrl(BUF as u32, false), 0, 0);
        self.rx[which].head = (head + 1) % RING_N;
        self.rx[which].tail = (tail + 1) % RING_N;
        self.kick(regs, self.rx[which].head);
        Some(n)
    }

    pub(crate) fn tx_80211(&mut self, frame: &[u8]) -> bool {
        if frame.len() + TXD_LEN + TXP_LEN > BUF {
            return false;
        }
        let phys = DMA_PHYS + OFF_TX_FRAME as u32;
        {
            let txwi = &mut scratch().cmd[..TXD_LEN + TXP_LEN];
            if !write_txwi(txwi, frame, phys) {
                return false;
            }
        }
        dma_write(self.dma, OFF_TXWI, &scratch().cmd[..TXD_LEN + TXP_LEN]);
        dma_write(self.dma, OFF_TX_FRAME, frame);
        self.post_tx(TX_DATA, DMA_PHYS + OFF_TXWI as u32, TXD_LEN + TXP_LEN)
    }

    pub(crate) fn next_seq_wifi(&mut self) -> u16 {
        let seq = self.wifi_seq;
        self.wifi_seq = self.wifi_seq.wrapping_add(1);
        seq
    }

    pub(crate) fn next_pn(&mut self) -> u64 {
        let pn = self.pn;
        self.pn = self.pn.wrapping_add(1);
        pn
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
        match map_reg(addr) {
            Map::Direct(off) => rd(self.regs, off),
            Map::Remap { base, window } => {
                wr(self.regs, HIF_REMAP_L1, base);
                rd(self.regs, window)
            }
            Map::Unsupported => u32::MAX,
        }
    }

    fn wr(&mut self, addr: u32, value: u32) {
        match map_reg(addr) {
            Map::Direct(off) => wr(self.regs, off, value),
            Map::Remap { base, window } => {
                wr(self.regs, HIF_REMAP_L1, base);
                wr(self.regs, window, value);
            }
            Map::Unsupported => {}
        }
    }

    fn kick(&mut self, regs: u32, head: u32) {
        fence(Ordering::SeqCst);
        self.wr(regs + 8, head);
    }

    fn desc_write(&mut self, desc: usize, idx: u32, buf0: u32, ctrl: u32, buf1: u32, info: u32) {
        let off = desc + idx as usize * 16;
        dma_u32(self.dma, off, buf0);
        dma_u32(self.dma, off + 4, ctrl);
        dma_u32(self.dma, off + 8, buf1);
        dma_u32(self.dma, off + 12, info);
    }

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

pub(crate) fn assoc_budget() -> u32 {
    ASSOC_SPIN
}

fn spin(n: u32) {
    for _ in 0..n {
        core::hint::spin_loop();
    }
}

fn rd(regs: *mut u8, off: u32) -> u32 {
    unsafe { regs.add(off as usize).cast::<u32>().read_volatile() }
}

fn wr(regs: *mut u8, off: u32, value: u32) {
    unsafe { regs.add(off as usize).cast::<u32>().write_volatile(value) }
}

fn dma_u32(dma: *mut u8, off: usize, value: u32) {
    unsafe { dma.add(off).cast::<u32>().write_volatile(value) }
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

const _: () = assert!(OFF_TX_FRAME + BUF < 0x4_0000);
const _: () = assert!(LAST_SEC0 == 1 << 30);
