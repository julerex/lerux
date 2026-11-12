//! MT7921 at `03:00.0`. The MAC follows the Linux v6.8 mt76 sequence.
//!
//! Init paints a moving `mt7921 .` while a poll is running. The semaphore
//! transmit then paints `cpu: write cmd`, `ram: write desc`, `wfdma: doorbell`,
//! and `wfdma: fetch` on the rows above the stage line. A stuck transmit also
//! paints `slot` on the row above those four: the posted index, the buffer
//! address, and the done bit. The stage line is `own`,
//! `fw sem`, `fw patch`, `fw scatter`, `fw ram`, `fw start`, `fw chip`, or
//! `mac` when that step fails, and `wait` when the
//! firmware has reached the normal state. The shell then asks for a survey.
//! This domain writes the names it hears and, after the passphrase, associates.
//! An empty name list still replaces row 22 with `rx`, `ok`, and `done`.
//! The shell notifies channel 1 for both. Channel 0 stays the MSI.
//! Association paints `scan`, `auth`, `assoc`, and `eapol`, then returns to
//! `wait`. The shell owns the address and the ping. Received Ethernet frames
//! are enqueued on the standard receive queue, and transmit frames are
//! dequeued from the standard transmit queue.
//! Rows 51 to 65 are the register, ring, descriptor, and command dump that
//! `dump` paints once the boot attempt has returned, on success or failure.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg_attr(test, allow(dead_code))]
mod ether;

#[cfg(not(test))]
mod device;
#[cfg(not(test))]
mod dump;
#[cfg(not(test))]
mod join;

#[cfg(not(test))]
use core::sync::atomic::{AtomicU8, Ordering};

#[cfg(not(test))]
use sel4_microkit::{
    memory_region_symbol, protection_domain, Channel, ChannelSet, Handler, Infallible,
};

#[cfg(not(test))]
use lerux_desk::FRAMEBUFFER_BYTES;
#[cfg(not(test))]
use lerux_sddf::{net_driver_config_t, net_image::net_config_from_bytes, SDDF_NET_MAGIC};
#[cfg(not(test))]
use lerux_swift::{
    bar_line, class_code,
    dump::PANEL_COLS,
    format_report, load_header, memory_bar,
    mt7921::{read_survey_mark, survey_requested, write_slot_line, Sem, Stage},
    paint_row, read_key, vendor_device, working_line, WaitMark, WifiReport, MT7921_BAR, MT7921_BDF,
    MT7921_IDS,
};

#[cfg(not(test))]
use device::{Images, Mac};
#[cfg(not(test))]
use ether::Net;

#[cfg(not(test))]
const ROW: usize = 22;
#[cfg(not(test))]
const ROW_SLOT: usize = 17;
#[cfg(not(test))]
const ROW_CMD: usize = 18;
#[cfg(not(test))]
const ROW_DESC: usize = 19;
#[cfg(not(test))]
const ROW_BELL: usize = 20;
#[cfg(not(test))]
const ROW_FETCH: usize = 21;
#[cfg(not(test))]
const PHASE_FETCH: u8 = 4;
#[cfg(not(test))]
static MARK: WaitMark = WaitMark::new();
#[cfg(not(test))]
static PHASE: AtomicU8 = AtomicU8::new(0);
#[cfg(not(test))]
const IRQ: Channel = Channel::new(0);
#[cfg(not(test))]
const KEY: Channel = Channel::new(1);

#[cfg(not(test))]
const PATCH: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patch.bin"));
#[cfg(not(test))]
const PATCH_1A: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patch_1a.bin"));
#[cfg(not(test))]
const RAM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mt7921.bin"));
#[cfg(not(test))]
const RAM_1A: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/ram_1a.bin"));
#[cfg(not(test))]
const NET_CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/net_driver.bin"));

#[cfg(not(test))]
struct HandlerImpl {
    mac: Option<Mac>,
    net: Net,
    rx_id: u8,
    tx_id: u8,
}

#[cfg(not(test))]
#[protection_domain]
fn init() -> HandlerImpl {
    let (net, rx_id, tx_id) = install_net();
    let page = memory_region_symbol!(pci_headers_vaddr: *mut u8).as_ptr();
    let page = unsafe { core::slice::from_raw_parts(page, 0x1000) };
    let (bus, dev, func) = MT7921_BDF;
    let header = load_header(page, bus, dev, func);
    let present = vendor_device(&header).is_some_and(|id| MT7921_IDS.contains(&id));
    let _ = class_code(&header);
    let images = firmware_images();
    let mut line = [0u8; 40];
    let bar = memory_bar(&header);
    if !present {
        let n = format_report(&WifiReport::Absent, &mut line);
        return stopped(net, rx_id, tx_id, &line[..n]);
    }
    if bar != Some(MT7921_BAR) {
        let n = bar_line(bar, &mut line);
        return stopped(net, rx_id, tx_id, &line[..n]);
    }
    if !images.any() {
        let n = format_report(&WifiReport::FirmwareErr, &mut line);
        return stopped(net, rx_id, tx_id, &line[..n]);
    }
    let regs = memory_region_symbol!(mt7921_regs_vaddr: *mut u8).as_ptr();
    let dma = memory_region_symbol!(wifi_dma_vaddr: *mut u8).as_ptr();
    beat_arm();
    let mac = Mac::boot(regs, dma, &images);
    beat_disarm();
    paint_stage(mac.stage());
    dump::paint_mac_dump(mac.before(), mac.after());
    HandlerImpl {
        mac: Some(mac),
        net,
        rx_id,
        tx_id,
    }
}

#[cfg(not(test))]
fn firmware_images() -> Images {
    Images {
        patch: PATCH,
        patch_1a: PATCH_1A,
        ram: RAM,
        ram_1a: RAM_1A,
    }
}

#[cfg(not(test))]
fn install_net() -> (Net, u8, u8) {
    // SAFETY: the build script wrote a zero-filled `net_driver_config_t`.
    // The struct has no `bool` fields. The queue addresses are stored and
    // not loaded until a handshake finishes.
    let config = unsafe { net_config_from_bytes::<net_driver_config_t>(NET_CONFIG) };
    assert_eq!(config.magic, SDDF_NET_MAGIC);
    let net = Net::from_driver(&config);
    (net, Net::rx_id(&config), Net::tx_id(&config))
}

#[cfg(not(test))]
fn stopped(net: Net, rx_id: u8, tx_id: u8, text: &[u8]) -> HandlerImpl {
    paint(text);
    HandlerImpl {
        mac: None,
        net,
        rx_id,
        tx_id,
    }
}

/// The status line is painted only after the poll. A moving dot fills that wait.
#[cfg(not(test))]
fn beat_arm() {
    MARK.arm();
    paint_working(0);
}

#[cfg(not(test))]
fn beat_disarm() {
    MARK.disarm();
}

#[cfg(not(test))]
pub(crate) fn beat() {
    let Some(tick) = MARK.beat() else {
        return;
    };
    match PHASE.load(Ordering::Relaxed) {
        0 => paint_working(tick),
        PHASE_FETCH => paint_fetch(tick),
        _ => {}
    }
}

#[cfg(not(test))]
pub(crate) fn step_cmd() {
    show(1, ROW_CMD, b"cpu: write cmd");
}

#[cfg(not(test))]
pub(crate) fn step_desc() {
    show(2, ROW_DESC, b"ram: write desc");
}

#[cfg(not(test))]
pub(crate) fn step_bell() {
    show(3, ROW_BELL, b"wfdma: doorbell");
}

#[cfg(not(test))]
pub(crate) fn step_fetch() {
    show(PHASE_FETCH, ROW_FETCH, b"wfdma: fetch");
}

#[cfg(not(test))]
fn show(phase: u8, row: usize, text: &[u8]) {
    if !MARK.is_armed() {
        return;
    }
    if phase == 1 {
        paint_at(ROW, b"");
    }
    PHASE.store(phase, Ordering::Relaxed);
    if phase == PHASE_FETCH {
        paint_fetch(0);
        return;
    }
    paint_at(row, text);
}

#[cfg(not(test))]
fn paint_fetch(tick: u32) {
    let mut line = [0u8; 24];
    let n = working_line(b"wfdma: fetch", tick, &mut line);
    paint_at(ROW_FETCH, &line[..n]);
}

#[cfg(not(test))]
fn paint_working(tick: u32) {
    let mut line = [0u8; 16];
    let n = working_line(b"mt7921", tick, &mut line);
    paint(&line[..n]);
}

#[cfg(not(test))]
fn paint_stage(stage: Stage) {
    let mut line = [0u8; 40];
    let n = stage.paint(&mut line);
    paint(&line[..n]);
    if let Stage::FwSem(Sem::Tx(stop)) = stage {
        let n = write_slot_line(&mut line, stop);
        paint_at(ROW_SLOT, &line[..n]);
    }
}

/// Row 22 keeps the survey counts after `wait`. A menu scroll erases that row,
/// so the shell prints the same bytes.
#[cfg(not(test))]
fn paint_survey_mark() {
    let Some(mark) = read_survey_mark(key_page()) else {
        return;
    };
    let mut line = [b' '; 48];
    let prefix = b"mt7921: ";
    line[..prefix.len()].copy_from_slice(prefix);
    let n = mark.len().min(line.len() - prefix.len());
    line[prefix.len()..prefix.len() + n].copy_from_slice(&mark[..n]);
    paint(&line);
}

#[cfg(not(test))]
fn paint(text: &[u8]) {
    paint_at(ROW, text);
}

#[cfg(not(test))]
fn paint_at(row: usize, text: &[u8]) {
    let mut line = [b' '; 48];
    let n = text.len().min(line.len());
    line[..n].copy_from_slice(&text[..n]);
    let width = n.max(24);
    paint_line(row, &line[..width]);
}

/// A dump row, padded to the panel width so a repaint erases the old row.
#[cfg(not(test))]
pub(crate) fn paint_full(row: usize, text: &[u8]) {
    let mut line = [b' '; PANEL_COLS];
    let n = text.len().min(PANEL_COLS);
    line[..n].copy_from_slice(&text[..n]);
    paint_line(row, &line);
}

#[cfg(not(test))]
fn paint_line(row: usize, line: &[u8]) {
    let mode = memory_region_symbol!(mode_record_vaddr: *mut u8).as_ptr();
    let frame = memory_region_symbol!(fb_vaddr: *mut u8).as_ptr();
    let record = unsafe { core::slice::from_raw_parts(mode, 40) };
    let bytes = usize::try_from(FRAMEBUFFER_BYTES).expect("framebuffer fits");
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, bytes) };
    paint_row(record, frame, row, line);
}

#[cfg(not(test))]
fn key_page() -> &'static mut [u8] {
    let ptr = memory_region_symbol!(wifi_key_vaddr: *mut u8).as_ptr();
    unsafe { core::slice::from_raw_parts_mut(ptr, 0x1000) }
}

#[cfg(not(test))]
fn posted_key() -> Option<([u8; 32], usize, [u8; 64], usize)> {
    let page = key_page();
    let (ssid, psk) = read_key(page)?;
    let mut name = [0u8; 32];
    let mut pass = [0u8; 64];
    name[..ssid.len()].copy_from_slice(ssid);
    pass[..psk.len()].copy_from_slice(psk);
    Some((name, ssid.len(), pass, psk.len()))
}

#[cfg(not(test))]
impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(IRQ) {
            IRQ.irq_ack().expect("ack mt7921");
        }
        if channels.contains(KEY) {
            self.on_key();
        }
        self.pump();
        Ok(())
    }
}

#[cfg(not(test))]
impl HandlerImpl {
    fn on_key(&mut self) {
        let Some(mac) = self.mac.as_mut() else {
            return;
        };
        if !mac.ready() {
            paint_stage(mac.stage());
        } else if survey_requested(key_page()) {
            paint_stage(Stage::Scan);
            mac.survey(key_page());
            paint_stage(Stage::Wait);
            paint_survey_mark();
        } else if let Some((name, name_len, pass, pass_len)) = posted_key() {
            let stage = mac.associate(&name[..name_len], &pass[..pass_len], paint_stage);
            paint_stage(stage);
        }
    }

    fn pump(&mut self) {
        let Some(mac) = self.mac.as_mut() else {
            return;
        };
        let note = mac.pump(&self.net);
        notify(self.rx_id, note.rx);
        notify(self.tx_id, note.tx);
    }
}

#[cfg(not(test))]
fn notify(id: u8, want: bool) {
    if want {
        Channel::new(usize::from(id)).notify();
    }
}
