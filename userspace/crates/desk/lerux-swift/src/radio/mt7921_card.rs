//! Host model of the Swift MT7921 (`14c3:7961`).
//!
//! The protection domain's `Mac` writes the BAR and the DMA window. This type
//! is the card on the other side of those accesses: ownership and reset bits,
//! the WFDMA rings, firmware download, MCU events, and 802.11 frames.
//!
//! Sources, Linux v6.8 `mt76` (the tree `userspace/pds/board/swift/swift-mt7921` follows):
//! `mt7921/pci.c` (BAR map, chip id, ownership), `mt792x_dma.c` and `dma.c`
//! (ring registers, descriptor `DMA_DONE`, `mt76_dma_rx_process`),
//! `mt7921/mac.c` (`mt7921_queue_rx_skb`: type 2, or type 7 with flag 1),
//! `mt76_connac_mcu.c` and `mt7921/mcu.c` (download, events, scan-done).
//! The register addresses are the ones [`crate::mt7921`] already uses.
//! The product identity is
//! [`docs/hardware/swift/MT7921LEN-Product_Brief_202308.pdf`](../../../../../docs/hardware/swift/MT7921LEN-Product_Brief_202308.pdf):
//! MT7921LEN, 802.11a/b/g/n/ac/ax, 2T2R, Bluetooth 5.2. The Swift PCI device
//! is `14c3:7961`. Frame delivery follows
//! [`docs/hardware/swift/swift-wifi-survey-retrospective.md`](../../../../../docs/hardware/swift/swift-wifi-survey-retrospective.md):
//! scan frames are read from every ring, and most of them arrive on an MCU ring.
//!
//! Completions are published before the doorbell write returns. `Mac` polls,
//! so the model does not raise the MSI.

use crate::{
    mt7921::{
        desc_ctrl, desc_len, map_reg, Map, CONN_ON_LPCTL, CONN_ON_MISC, DMA_DONE, EXT_CID,
        FW_N9_RDY, FW_START, GLO_CFG, GLO_CFG_RX_BUSY, GLO_CFG_TX_BUSY, HIF_REMAP_L1, HOST_CLR_OWN,
        HOST_OWN_SYNC, HOST_SET_OWN, HW_CHIPID, HW_REV, LPCR_DRV_OWN, LPCR_FW_OWN, MCU_TXD_LEN,
        PATCH_FINISH, PATCH_IS_DL, PATCH_NOT_DL, PATCH_REL, PATCH_SEM, RING_STRIDE, RXD_LEN,
        RX_DATA_BASE, RX_EVENT_BASE, RX_WA_BASE, SCAN_DONE, TOP_LPCR, TX_RING_BASE,
        WFSYS_INIT_DONE, WFSYS_RST, WFSYS_RST_B, WTBL_BUSY, WTBL_UPDATE,
    },
    wpa::{derive_ptk, pmk, write_message1, write_message3},
};

/// BAR the Swift image maps. `__mt7921_reg_addr` covers a 2 MiB window.
pub const BAR_LEN: usize = 0x20_0000;
/// Bytes at physical [`DMA_PHYS`]. The driver programs every ring inside it.
pub const DMA_LEN: usize = 0x4_0000;
/// Physical base of the Swift DMA window (`swift-mt7921` `DMA_PHYS`).
pub const DMA_PHYS: u32 = 0x01f6_0000;
/// `MT_HW_CHIPID` for PCI `14c3:7961`. The panel read this as `00007961`.
pub const CHIP_ID: u32 = 0x7961;
/// Reset value of `MT_HW_REV`. The brief does not publish it. The low byte is
/// not a chip id [`crate::mt7921::firmware_kind`] rejects.
pub const HW_REV_RESET: u32 = 0x8a00;
/// Station address the efuse and `GET_NIC_CAPAB` both return.
pub const STA_MAC: [u8; 6] = [0x00, 0x0c, 0xe7, 0x79, 0x61, 0x01];
/// `complete_channel_num` for the 21-channel domain in `write_scan`.
pub const SURVEY_CHANNELS: u8 = 21;
/// `beacon_scan_num`. The firmware count includes beacons the host never saw.
pub const SURVEY_BEACONS: u32 = 15;
/// Second survey name. It arrives on the WA ring.
pub const NEIGHBOR_SSID: &[u8] = b"north";
/// Third survey name. It arrives on the WA ring.
pub const OTHER_SSID: &[u8] = b"south";

const NORTH_BSSID: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x21];
const SOUTH_BSSID: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x36];
const PLAIN_OPTION: u8 = 3;
const UNI_MARK: u8 = 0x07;
const NIC_MAC_TLV: u32 = 7;
const NIC_PHY_TLV: u32 = 8;
const SNAP: [u8; 6] = [0xaa, 0xaa, 0x03, 0x00, 0x00, 0x00];
const REMAP_CAPTURE: u32 = 0x1845_1844;

/// Access point the model answers scans and the handshake for.
#[derive(Clone, Copy)]
pub struct Air {
    bssid: [u8; 6],
    ssid: [u8; 32],
    ssid_len: u8,
    channel: u8,
    psk: [u8; 63],
    psk_len: u8,
}

impl Air {
    /// `ssid` is 1 to 32 bytes. `psk` is 8 to 63 bytes.
    #[must_use]
    pub fn new(bssid: [u8; 6], ssid: &[u8], channel: u8, psk: &[u8]) -> Option<Self> {
        if ssid.is_empty() || ssid.len() > 32 || !(8..=63).contains(&psk.len()) {
            return None;
        }
        let mut out = Self {
            bssid,
            ssid: [0; 32],
            ssid_len: fit_u8(ssid.len()),
            channel,
            psk: [0; 63],
            psk_len: fit_u8(psk.len()),
        };
        out.ssid[..ssid.len()].copy_from_slice(ssid);
        out.psk[..psk.len()].copy_from_slice(psk);
        Some(out)
    }

    fn ssid(&self) -> &[u8] {
        &self.ssid[..usize::from(self.ssid_len)]
    }

    fn psk(&self) -> &[u8] {
        &self.psk[..usize::from(self.psk_len)]
    }
}

#[derive(Clone, Copy)]
enum TxKind {
    Data,
    Fw,
    Mcu,
}

/// One simulated MT7921. The pointers are the BAR and the DMA window.
pub struct Card {
    regs: *mut u8,
    dma: *mut u8,
    air: Air,
    page: u16,
    n9: bool,
    patch_dl: bool,
    fw_bytes: usize,
    anonce: [u8; 32],
    chip: u32,
    rev: u32,
    wfsys: u32,
    lpcr: u32,
    /// Next descriptor the driver will read, one cursor per receive ring.
    rx_next: [u32; 3],
}

impl Card {
    /// `regs` is [`BAR_LEN`] bytes. `dma` is [`DMA_LEN`] bytes.
    ///
    /// # Safety
    ///
    /// Both pointers stay valid and uniquely owned for the life of this value.
    /// Register offsets the driver touches are 4-byte aligned.
    #[must_use]
    pub unsafe fn bind(regs: *mut u8, dma: *mut u8, air: Air) -> Self {
        let card = Self {
            regs,
            dma,
            air,
            page: u16::try_from(REMAP_CAPTURE & 0xffff).unwrap_or(0),
            n9: false,
            patch_dl: false,
            fw_bytes: 0,
            anonce: [0x3c; 32],
            chip: CHIP_ID,
            rev: HW_REV_RESET,
            wfsys: 0,
            lpcr: LPCR_FW_OWN,
            rx_next: [0; 3],
        };
        card.poke(HIF_REMAP_L1, REMAP_CAPTURE);
        card
    }

    /// Firmware bytes the download ring copied. Patch and RAM both count.
    #[must_use]
    pub fn fw_bytes(&self) -> usize {
        self.fw_bytes
    }

    /// Fill a remapped window before the driver reads it.
    pub fn before_read(&mut self, off: u32) {
        if !(0x4_0000..0x5_0000).contains(&off) {
            return;
        }
        let Some(value) = self.remap_get(off - 0x4_0000) else {
            return;
        };
        self.poke(off, value);
    }

    /// Apply the side effect of a store the driver just performed.
    pub fn after_write(&mut self, off: u32, value: u32) {
        if off == HIF_REMAP_L1 {
            self.page = u16::try_from(value & 0xffff).unwrap_or(0);
            return;
        }
        if off == direct(CONN_ON_LPCTL) {
            self.poke(off, lpctl_value(value));
            return;
        }
        if off == direct(GLO_CFG) {
            let busy = GLO_CFG_TX_BUSY | GLO_CFG_RX_BUSY;
            self.poke(off, value & !busy);
            return;
        }
        if off == direct(WTBL_UPDATE) {
            self.poke(off, value & !WTBL_BUSY);
            return;
        }
        if (0x4_0000..0x5_0000).contains(&off) {
            self.on_window(off, value);
            return;
        }
        if let Some((ring, kind)) = tx_cpu(off) {
            self.drain_tx(ring, kind);
        }
    }

    fn on_window(&mut self, off: u32, value: u32) {
        let logical = off - 0x4_0000;
        if self.page == page_of(WFSYS_RST) && logical == win_of(WFSYS_RST) {
            self.wfsys = wfsys_value(value);
            self.poke(off, self.wfsys);
        } else if self.page == page_of(TOP_LPCR) && logical == win_of(TOP_LPCR) {
            self.lpcr = lpcr_value(value);
            self.poke(off, self.lpcr);
        } else if self.page == page_of(HW_CHIPID) && logical == win_of(HW_CHIPID) {
            self.chip = value;
        } else if self.page == page_of(HW_REV) && logical == win_of(HW_REV) {
            self.rev = value;
        }
    }

    fn remap_get(&self, logical: u32) -> Option<u32> {
        if self.page == page_of(WFSYS_RST) && logical == win_of(WFSYS_RST) {
            return Some(self.wfsys);
        }
        if self.page == page_of(TOP_LPCR) && logical == win_of(TOP_LPCR) {
            return Some(self.lpcr);
        }
        if self.page == page_of(HW_CHIPID) && logical == win_of(HW_CHIPID) {
            return Some(self.chip);
        }
        if self.page == page_of(HW_REV) && logical == win_of(HW_REV) {
            return Some(self.rev);
        }
        None
    }

    fn drain_tx(&mut self, ring: u32, kind: TxKind) {
        let base = self.r32(ring);
        let count = self.r32(ring + 4) & 0xffff;
        if count == 0 {
            return;
        }
        let cpu = self.r32(ring + 8) & 0xff;
        let mut dma_i = self.r32(ring + 12);
        let mut guard = 0;
        while (dma_i & 0xff) != cpu && guard < count {
            let slot = dma_i & 0xff;
            self.complete(base.wrapping_add(slot.wrapping_mul(16)), kind);
            dma_i = (dma_i + 1) % count;
            self.poke(ring + 12, dma_i);
            guard += 1;
        }
    }

    fn complete(&mut self, desc: u32, kind: TxKind) {
        let Some(off) = dma_off(desc) else {
            return;
        };
        if off + 16 > DMA_LEN {
            return;
        }
        let buf = self.dma_u32(off);
        let ctrl = self.dma_u32(off + 4);
        let len = desc_len(ctrl);
        self.dma_set(off + 4, ctrl | DMA_DONE);
        match kind {
            TxKind::Fw => self.fw_bytes = self.fw_bytes.saturating_add(len),
            TxKind::Mcu => self.run_mcu(buf, len),
            TxKind::Data => self.run_wifi(buf, len),
        }
    }

    fn run_mcu(&mut self, phys: u32, len: usize) {
        let mut buf = [0u8; 1536];
        let n = self.copy_dma(phys, len, &mut buf);
        if n >= 44 {
            self.on_mcu(&buf[..n]);
        }
    }

    fn run_wifi(&mut self, phys: u32, len: usize) {
        let mut txwi = [0u8; 64];
        let n = self.copy_dma(phys, len, &mut txwi);
        if n >= 46 {
            self.on_wifi(&txwi[..n]);
        }
    }

    fn on_mcu(&mut self, pkt: &[u8]) {
        if pkt[43] == UNI_MARK {
            self.on_uni(pkt);
        } else if pkt[38] == PLAIN_OPTION {
            self.on_plain(pkt);
        } else if pkt[36] == low(EXT_CID) {
            self.on_ext(pkt);
        } else {
            self.on_ce(pkt);
        }
    }

    fn on_uni(&mut self, pkt: &[u8]) {
        let mut out = [0u8; 48];
        event_hdr(&mut out, pkt[39], 0);
        out[RXD_LEN] = pkt[34];
        let _ = self.post(self.mcu_ring(), &out);
    }

    fn on_plain(&mut self, pkt: &[u8]) {
        let seq = pkt[39];
        match pkt[36] {
            id if id == low(PATCH_SEM) => self.on_sem(pkt, seq),
            id if id == low(PATCH_FINISH) => self.on_finish(seq),
            id if id == low(FW_START) => self.on_start(seq),
            _ => self.ack(seq, 0),
        }
    }

    fn on_sem(&mut self, pkt: &[u8], seq: u8) {
        let get = pkt.get(MCU_TXD_LEN).copied() == Some(1);
        let status = sem_status(get, self.patch_dl);
        self.ack(seq, status);
    }

    fn on_finish(&mut self, seq: u8) {
        self.patch_dl = true;
        self.ack(seq, 0);
    }

    fn on_start(&mut self, seq: u8) {
        self.ack(seq, 0);
        self.n9 = true;
        self.poke(direct(CONN_ON_MISC), FW_N9_RDY);
    }

    fn on_ext(&mut self, pkt: &[u8]) {
        let seq = pkt[39];
        if pkt[41] == 1 {
            self.on_efuse(pkt, seq);
        } else {
            self.ack(seq, 0);
        }
    }

    fn on_efuse(&mut self, pkt: &[u8], seq: u8) {
        if pkt.len().saturating_sub(MCU_TXD_LEN) >= 24 {
            self.ack_block(seq);
        } else {
            self.ack_mac(seq);
        }
    }

    fn on_ce(&mut self, pkt: &[u8]) {
        let seq = pkt[39];
        match pkt[36] {
            0x8a => self.ack_nic(seq),
            0x03 => self.on_scan(seq),
            _ => self.ack(seq, 0),
        }
    }

    fn on_scan(&mut self, seq: u8) {
        let bssid = self.air.bssid;
        let channel = self.air.channel;
        let mut ssid = [0u8; 32];
        let n = usize::from(self.air.ssid_len);
        ssid[..n].copy_from_slice(self.air.ssid());
        self.post_named(RX_DATA_BASE, &ssid[..n], channel, bssid, false);
        self.post_named(RX_WA_BASE, NEIGHBOR_SSID, 1, NORTH_BSSID, true);
        self.post_named(RX_WA_BASE, OTHER_SSID, 36, SOUTH_BSSID, true);
        self.post_done(seq);
    }

    fn on_wifi(&mut self, txwi: &[u8]) {
        let phys = u32::from_le_bytes([txwi[40], txwi[41], txwi[42], txwi[43]]);
        let raw = u16::from_le_bytes([txwi[44], txwi[45]]);
        self.on_frame(phys, usize::from(raw & 0x7fff));
    }

    fn on_frame(&mut self, phys: u32, len: usize) {
        let mut frame = [0u8; 400];
        let n = self.copy_dma(phys, len, &mut frame);
        if n < 24 {
            return;
        }
        match frame[0] & 0xfc {
            0xb0 => self.on_auth(&frame[..n]),
            0x00 => self.on_assoc(&frame[..n]),
            0x08 => self.on_data(&frame[..n]),
            _ => {}
        }
    }

    fn on_auth(&mut self, frame: &[u8]) {
        if frame.get(26) != Some(&1) {
            return;
        }
        self.post_mgmt(&auth_resp(addr_at(frame, 10), self.air.bssid));
    }

    fn on_assoc(&mut self, frame: &[u8]) {
        let sta = addr_at(frame, 10);
        self.post_mgmt(&assoc_resp(sta, self.air.bssid));
        self.post_m1(sta);
    }

    fn on_data(&mut self, frame: &[u8]) {
        let Some(eapol) = eapol_of(frame) else {
            return;
        };
        let Some(snonce) = message2_nonce(eapol) else {
            return;
        };
        self.post_m3(addr_at(frame, 10), snonce);
    }

    fn post_m1(&mut self, sta: [u8; 6]) {
        let mut eapol = [0u8; 128];
        let replay = replay_of(1);
        let Some(n) = write_message1(&mut eapol, &self.anonce, &replay) else {
            return;
        };
        self.post_eapol(sta, &eapol[..n]);
    }

    fn post_m3(&mut self, sta: [u8; 6], snonce: [u8; 32]) {
        let Some(key) = self.air_pmk() else {
            return;
        };
        let ptk = derive_ptk(&key, &self.air.bssid, &sta, &self.anonce, &snonce);
        let mut eapol = [0u8; 128];
        let replay = replay_of(2);
        let Some(n) = write_message3(&mut eapol, &self.anonce, &replay, &ptk.kck) else {
            return;
        };
        self.post_eapol(sta, &eapol[..n]);
    }

    fn air_pmk(&self) -> Option<[u8; 32]> {
        let ssid = self.air.ssid();
        let pass = self.air.psk();
        if ssid.is_empty() || pass.len() < 8 {
            return None;
        }
        Some(pmk(pass, ssid))
    }

    fn post_named(&mut self, ring: u32, ssid: &[u8], channel: u8, bssid: [u8; 6], mcu: bool) {
        let mut frame = [0u8; 96];
        let n = beacon(&mut frame, bssid, ssid, channel);
        if n > 0 {
            self.post_frame(ring, mcu, &frame[..n]);
        }
    }

    fn post_mgmt(&mut self, frame: &[u8; 30]) {
        self.post_frame(RX_DATA_BASE, false, frame);
    }

    fn post_eapol(&mut self, sta: [u8; 6], eapol: &[u8]) {
        let mut frame = [0u8; 160];
        let n = eapol_frame(&mut frame, sta, self.air.bssid, eapol);
        if n > 0 {
            self.post_frame(RX_DATA_BASE, false, &frame[..n]);
        }
    }

    fn post_frame(&mut self, ring: u32, mcu: bool, frame: &[u8]) {
        let mut pkt = [0u8; 320];
        let n = wrap_frame(&mut pkt, mcu, frame);
        if n > 0 {
            let _ = self.post(ring, &pkt[..n]);
        }
    }

    fn post_done(&mut self, seq: u8) {
        let mut pkt = [0u8; 48];
        event_hdr(&mut pkt, seq, SCAN_DONE);
        pkt[RXD_LEN + 4] = SURVEY_CHANNELS;
        let beacons = SURVEY_BEACONS.to_le_bytes();
        pkt[RXD_LEN + 8..RXD_LEN + 12].copy_from_slice(&beacons);
        let _ = self.post(self.mcu_ring(), &pkt);
    }

    fn ack(&mut self, seq: u8, status: u8) {
        let mut pkt = [0u8; 48];
        event_hdr(&mut pkt, seq, 0);
        pkt[32] = status;
        let _ = self.post(self.mcu_ring(), &pkt);
    }

    fn ack_nic(&mut self, seq: u8) {
        let mut pkt = [0u8; 72];
        event_hdr(&mut pkt, seq, 0);
        pkt[RXD_LEN] = 2;
        put_u32(&mut pkt, 40, NIC_MAC_TLV);
        put_u32(&mut pkt, 44, 6);
        pkt[48..54].copy_from_slice(&STA_MAC);
        put_u32(&mut pkt, 54, NIC_PHY_TLV);
        put_u32(&mut pkt, 58, 5);
        pkt[66] = 2;
        let _ = self.post(self.mcu_ring(), &pkt[..67]);
    }

    fn ack_mac(&mut self, seq: u8) {
        let mut pkt = [0u8; 60];
        event_hdr(&mut pkt, seq, 0);
        pkt[48..54].copy_from_slice(&STA_MAC);
        let _ = self.post(self.mcu_ring(), &pkt);
    }

    fn ack_block(&mut self, seq: u8) {
        let mut pkt = [0u8; 60];
        event_hdr(&mut pkt, seq, 0);
        let _ = self.post(self.mcu_ring(), &pkt);
    }

    fn mcu_ring(&self) -> u32 {
        if self.n9 {
            RX_WA_BASE
        } else {
            RX_EVENT_BASE
        }
    }

    fn post(&mut self, ring: u32, bytes: &[u8]) -> bool {
        let base = self.r32(ring);
        let count = self.r32(ring + 4) & 0xffff;
        if count == 0 || bytes.is_empty() {
            return false;
        }
        // The driver consumes `tail` and only then refills a different slot.
        // The next free descriptor behind `tail` is not the one it will read.
        let index = rx_index(ring);
        let slot = self.rx_next[index] % count;
        if !self.fill_slot(base, slot, bytes) {
            return false;
        }
        let next = (slot + 1) % count;
        self.rx_next[index] = next;
        self.poke(ring + 12, next);
        true
    }

    fn fill_slot(&mut self, base: u32, slot: u32, bytes: &[u8]) -> bool {
        let desc = base.wrapping_add(slot.wrapping_mul(16));
        let Some(off) = dma_off(desc) else {
            return false;
        };
        if off + 16 > DMA_LEN || bytes.len() > 2048 {
            return false;
        }
        let buf = self.dma_u32(off);
        if self.dma_u32(off + 4) & DMA_DONE != 0 || buf == 0 {
            return false;
        }
        let Some(buf_off) = dma_off(buf) else {
            return false;
        };
        if buf_off + bytes.len() > DMA_LEN {
            return false;
        }
        self.write_dma(buf_off, bytes);
        let len = u32::try_from(bytes.len()).unwrap_or(0);
        self.dma_set(off + 4, desc_ctrl(len, true) | DMA_DONE);
        true
    }

    fn r32(&self, off: u32) -> u32 {
        load(self.regs, off)
    }

    fn dma_u32(&self, off: usize) -> u32 {
        let Some(off) = u32::try_from(off).ok() else {
            return 0;
        };
        load(self.dma, off)
    }

    fn poke(&self, off: u32, value: u32) {
        store(self.regs, off, value);
    }

    fn dma_set(&self, off: usize, value: u32) {
        let Some(off) = u32::try_from(off).ok() else {
            return;
        };
        store(self.dma, off, value);
    }

    fn copy_dma(&self, phys: u32, len: usize, out: &mut [u8]) -> usize {
        let Some(off) = dma_off(phys) else {
            return 0;
        };
        let n = len.min(out.len());
        if off + n > DMA_LEN {
            return 0;
        }
        for (i, byte) in out[..n].iter_mut().enumerate() {
            // Safety: `off + i` is inside the DMA window.
            *byte = unsafe { self.dma.add(off + i).read_volatile() };
        }
        n
    }

    fn write_dma(&self, off: usize, bytes: &[u8]) {
        for (i, byte) in bytes.iter().enumerate() {
            // Safety: `off + i` is inside the DMA window. `fill_slot` checked it.
            unsafe { self.dma.add(off + i).write_volatile(*byte) }
        }
    }
}

fn load(base: *mut u8, off: u32) -> u32 {
    let Some(ptr) = aligned(base, off) else {
        return 0;
    };
    // Safety: `off` is 4-byte aligned and inside the caller's window.
    unsafe { ptr.read_volatile() }
}

fn store(base: *mut u8, off: u32, value: u32) {
    let Some(ptr) = aligned(base, off) else {
        return;
    };
    // Safety: `off` is 4-byte aligned and inside the caller's window.
    unsafe { ptr.write_volatile(value) }
}

fn aligned(base: *mut u8, off: u32) -> Option<*mut u32> {
    if !off.is_multiple_of(4) {
        return None;
    }
    let off = usize::try_from(off).ok()?;
    // Safety: the caller keeps `base` valid for this offset. The cast is the
    // 4-byte register the driver also uses.
    Some(unsafe { base.add(off).cast() })
}

fn rx_index(ring: u32) -> usize {
    if ring == RX_DATA_BASE {
        1
    } else if ring == RX_WA_BASE {
        2
    } else {
        0
    }
}

fn direct(addr: u32) -> u32 {
    match map_reg(addr) {
        Map::Direct(off) => off,
        Map::Remap { .. } | Map::Unsupported => addr,
    }
}

fn page_of(addr: u32) -> u16 {
    u16::try_from(addr >> 16).unwrap_or(0)
}

fn win_of(addr: u32) -> u32 {
    addr & 0xffff
}

fn tx_cpu(off: u32) -> Option<(u32, TxKind)> {
    let data = ring_at(0) + 8;
    let fw = ring_at(16) + 8;
    let mcu = ring_at(17) + 8;
    if off == data {
        Some((data - 8, TxKind::Data))
    } else if off == fw {
        Some((fw - 8, TxKind::Fw))
    } else if off == mcu {
        Some((mcu - 8, TxKind::Mcu))
    } else {
        None
    }
}

fn ring_at(index: u32) -> u32 {
    TX_RING_BASE + index * RING_STRIDE
}

fn dma_off(phys: u32) -> Option<usize> {
    let off = phys.checked_sub(DMA_PHYS)?;
    let off = usize::try_from(off).ok()?;
    (off < DMA_LEN).then_some(off)
}

fn lpctl_value(value: u32) -> u32 {
    let mut out = value;
    if value & HOST_SET_OWN != 0 {
        out |= HOST_OWN_SYNC;
    }
    if value & HOST_CLR_OWN != 0 {
        out &= !HOST_OWN_SYNC;
    }
    out
}

fn wfsys_value(value: u32) -> u32 {
    if value & WFSYS_RST_B == 0 {
        value & !WFSYS_INIT_DONE
    } else {
        value | WFSYS_INIT_DONE
    }
}

fn lpcr_value(value: u32) -> u32 {
    if value & LPCR_DRV_OWN == 0 {
        value
    } else {
        value & !LPCR_FW_OWN
    }
}

fn low(cmd: u32) -> u8 {
    u8::try_from(cmd & 0xff).unwrap_or(0)
}

fn sem_status(get: bool, downloaded: bool) -> u8 {
    if !get {
        PATCH_REL
    } else if downloaded {
        PATCH_IS_DL
    } else {
        PATCH_NOT_DL
    }
}

fn fit_u8(value: usize) -> u8 {
    u8::try_from(value).unwrap_or(0)
}

fn event_hdr(pkt: &mut [u8], seq: u8, eid: u8) {
    pkt.fill(0);
    let rxd0 = 7u32 << 27;
    pkt[..4].copy_from_slice(&rxd0.to_le_bytes());
    if pkt.len() > 29 {
        pkt[28] = eid;
        pkt[29] = seq;
    }
}

fn put_u32(out: &mut [u8], at: usize, value: u32) {
    if let Some(slot) = out.get_mut(at..at + 4) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

fn wrap_frame(out: &mut [u8], mcu: bool, frame: &[u8]) -> usize {
    let n = 24 + frame.len();
    if out.len() < n {
        return 0;
    }
    out[..n].fill(0);
    let kind = if mcu { 7 } else { 2 };
    let rxd0 = (kind << 27) | (u32::from(mcu) << 16);
    out[..4].copy_from_slice(&rxd0.to_le_bytes());
    out[24..n].copy_from_slice(frame);
    n
}

fn beacon(out: &mut [u8], bssid: [u8; 6], ssid: &[u8], channel: u8) -> usize {
    let n = 36 + 2 + ssid.len() + 5;
    if ssid.len() > 32 || out.len() < n {
        return 0;
    }
    out[..n].fill(0);
    out[0] = 0x80;
    out[4..10].fill(0xff);
    out[10..16].copy_from_slice(&bssid);
    out[16..22].copy_from_slice(&bssid);
    out[32] = 0x64;
    let mut at = 36;
    out[at] = 0;
    out[at + 1] = fit_u8(ssid.len());
    at += 2;
    out[at..at + ssid.len()].copy_from_slice(ssid);
    at += ssid.len();
    out[at] = 3;
    out[at + 1] = 1;
    out[at + 2] = channel;
    n
}

fn auth_resp(sta: [u8; 6], bssid: [u8; 6]) -> [u8; 30] {
    let mut out = [0u8; 30];
    out[0] = 0xb0;
    out[4..10].copy_from_slice(&sta);
    out[10..16].copy_from_slice(&bssid);
    out[16..22].copy_from_slice(&bssid);
    out[26] = 2;
    out
}

fn assoc_resp(sta: [u8; 6], bssid: [u8; 6]) -> [u8; 30] {
    let mut out = [0u8; 30];
    out[0] = 0x10;
    out[4..10].copy_from_slice(&sta);
    out[10..16].copy_from_slice(&bssid);
    out[16..22].copy_from_slice(&bssid);
    out[28] = 1;
    out
}

fn eapol_frame(out: &mut [u8], sta: [u8; 6], bssid: [u8; 6], eapol: &[u8]) -> usize {
    let n = 32 + eapol.len();
    if out.len() < n {
        return 0;
    }
    out[..n].fill(0);
    out[0] = 0x08;
    out[1] = 0x02;
    out[4..10].copy_from_slice(&sta);
    out[10..16].copy_from_slice(&bssid);
    out[16..22].copy_from_slice(&bssid);
    out[24..30].copy_from_slice(&SNAP);
    out[30] = 0x88;
    out[31] = 0x8e;
    out[32..n].copy_from_slice(eapol);
    n
}

fn eapol_of(frame: &[u8]) -> Option<&[u8]> {
    let rest = frame.get(24..)?.strip_prefix(&SNAP)?;
    rest.strip_prefix(&[0x88, 0x8e])
}

fn message2_nonce(eapol: &[u8]) -> Option<[u8; 32]> {
    if eapol.len() < 99 || eapol[1] != 3 {
        return None;
    }
    let info = u16::from_be_bytes([eapol[5], eapol[6]]);
    if info & (1 << 7) != 0 || info & (1 << 8) == 0 {
        return None;
    }
    if u16::from_be_bytes([eapol[97], eapol[98]]) == 0 {
        return None;
    }
    let mut snonce = [0u8; 32];
    snonce.copy_from_slice(&eapol[17..49]);
    Some(snonce)
}

fn addr_at(frame: &[u8], at: usize) -> [u8; 6] {
    let mut out = [0u8; 6];
    if let Some(bytes) = frame.get(at..at + 6) {
        out.copy_from_slice(bytes);
    }
    out
}

fn replay_of(counter: u8) -> [u8; 8] {
    let mut replay = [0u8; 8];
    replay[7] = counter;
    replay
}

/// One patch section the download path accepts. The bytes are not a vendor image.
#[must_use]
pub const fn patch_image() -> [u8; 164] {
    let mut image = [0u8; 164];
    image[47] = 1;
    image[99] = 2;
    image[103] = 160;
    image[109] = 0x20;
    image[115] = 4;
    image[116] = 0xff;
    image[117] = 0xff;
    image[118] = 0xff;
    image[119] = 0xff;
    image[160] = 0x11;
    image[161] = 0x22;
    image[162] = 0x33;
    image[163] = 0x44;
    image
}

/// One RAM region plus the 36-byte trailer. There is no CLC region.
#[must_use]
pub const fn ram_image() -> [u8; 80] {
    let mut image = [0u8; 80];
    image[0] = 0xaa;
    image[1] = 0xbb;
    image[2] = 0xcc;
    image[3] = 0xdd;
    image[22] = 0x10;
    image[24] = 4;
    image[46] = 1;
    image
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mt7921::{patch_regions, ram_regions, Region};

    fn word(buf: &[u8], off: u32) -> u32 {
        let i = usize::try_from(off).unwrap_or(0);
        let bytes = buf.get(i..i + 4).unwrap_or(&[0; 4]);
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    fn card() -> (Vec<u8>, Vec<u8>, Card) {
        let air = Air::new([2, 0, 0, 0, 0, 1], b"lerux", 6, b"password").expect("air");
        let mut regs = vec![0u8; BAR_LEN];
        let mut dma = vec![0u8; DMA_LEN];
        // Safety: `regs` and `dma` are the live windows, and this test does not alias them.
        let card = unsafe { Card::bind(regs.as_mut_ptr(), dma.as_mut_ptr(), air) };
        (regs, dma, card)
    }

    #[test]
    fn images_parse_as_one_region() {
        let patch = patch_image();
        let mut regions = [Region {
            addr: 0,
            off: 0,
            len: 0,
            mode: 0,
            skip: false,
            override_addr: false,
        }; 1];
        assert_eq!(patch_regions(&patch, &mut regions), Some(1));
        assert_eq!(regions[0].len, 4);
        assert_eq!(regions[0].addr, 0x20_0000);
        let ram = ram_image();
        assert_eq!(ram_regions(&ram, &mut regions).map(|item| item.0), Some(1));
        assert_eq!(regions[0].len, 4);
    }

    #[test]
    fn own_wfsys_and_chip_follow_the_writes() {
        let (regs, _dma, mut card) = card();
        let lp = direct(CONN_ON_LPCTL);
        card.after_write(lp, HOST_SET_OWN);
        assert_eq!(word(&regs, lp) & HOST_OWN_SYNC, HOST_OWN_SYNC);
        card.after_write(lp, HOST_CLR_OWN);
        assert_eq!(word(&regs, lp) & HOST_OWN_SYNC, 0);
        card.after_write(HIF_REMAP_L1, 0x1845_1800);
        let window = 0x4_0000 + win_of(WFSYS_RST);
        card.after_write(window, WFSYS_RST_B);
        card.before_read(window);
        assert_ne!(word(&regs, window) & WFSYS_INIT_DONE, 0);
        card.after_write(HIF_REMAP_L1, 0x1845_7001);
        let chip = 0x4_0000 + win_of(HW_CHIPID);
        card.before_read(chip);
        assert_eq!(word(&regs, chip), CHIP_ID);
    }
}
