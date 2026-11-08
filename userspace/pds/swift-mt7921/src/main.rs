//! MT7921 at `03:00.0`. The MAC follows the Linux v6.8 mt76 sequence.
//!
//! Init paints a moving `mt7921 .` while a poll is running. The semaphore
//! transmit then paints `cpu: write cmd`, `ram: write desc`, `wfdma: doorbell`,
//! and `wfdma: fetch` on the rows above the stage line. A stuck transmit also
//! paints `slot` on the row above those four: the posted index, the buffer
//! address, and the done bit. Rows 13 and 14 read the full ring base, the
//! full DMA config, the transmit reset, and the scheduler control. The stage
//! line is `own`,
//! `fw sem`, `fw patch`, `fw scatter`, `fw ram`, `fw start`, `fw chip`, or
//! `mac` when that step fails, and `wait` when the
//! firmware has reached the normal state. The shell then writes the network
//! name and passphrase and notifies channel 1. Channel 0 stays the MSI.
//! Association paints `scan`, `auth`, `assoc`, `eapol`, `dhcp`, and `ping`.
//! Rows 52 to 66 are the register, ring, descriptor, and command dump that
//! `dump` paints once the boot attempt has returned, on success or failure.

#![no_std]
#![no_main]

mod device;
mod dump;
mod join;

use core::sync::atomic::{AtomicU8, Ordering};

use sel4_microkit::{
    memory_region_symbol, protection_domain, Channel, ChannelSet, Handler, Infallible,
};

use lerux_desk::FRAMEBUFFER_BYTES;
use lerux_swift::{
    bar_line, class_code,
    dump::PANEL_COLS,
    format_report, load_header, memory_bar,
    mt7921::{write_base_line, write_hold_line, write_slot_line, Sem, Stage},
    paint_row, read_key, vendor_device, working_line, WaitMark, WifiReport, MT7921_BAR, MT7921_BDF,
    MT7921_IDS,
};

use device::{Images, Mac};

const ROW: usize = 22;
const ROW_BASE: usize = 13;
const ROW_HOLD: usize = 14;
const ROW_SLOT: usize = 17;
const ROW_CMD: usize = 18;
const ROW_DESC: usize = 19;
const ROW_BELL: usize = 20;
const ROW_FETCH: usize = 21;
const PHASE_FETCH: u8 = 4;
static MARK: WaitMark = WaitMark::new();
static PHASE: AtomicU8 = AtomicU8::new(0);
const IRQ: Channel = Channel::new(0);
const KEY: Channel = Channel::new(1);

const PATCH: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patch.bin"));
const PATCH_1A: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patch_1a.bin"));
const RAM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mt7921.bin"));
const RAM_1A: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/ram_1a.bin"));

struct HandlerImpl {
    mac: Option<Mac>,
}

#[protection_domain]
fn init() -> HandlerImpl {
    let page = memory_region_symbol!(pci_headers_vaddr: *mut u8).as_ptr();
    let page = unsafe { core::slice::from_raw_parts(page, 0x1000) };
    let (bus, dev, func) = MT7921_BDF;
    let header = load_header(page, bus, dev, func);
    let present = vendor_device(&header).is_some_and(|id| MT7921_IDS.contains(&id));
    let _ = class_code(&header);
    let images = Images {
        patch: PATCH,
        patch_1a: PATCH_1A,
        ram: RAM,
        ram_1a: RAM_1A,
    };
    let mut line = [0u8; 40];
    let bar = memory_bar(&header);
    if !present {
        let n = format_report(&WifiReport::Absent, &mut line);
        paint(&line[..n]);
        return HandlerImpl { mac: None };
    }
    if bar != Some(MT7921_BAR) {
        let n = bar_line(bar, &mut line);
        paint(&line[..n]);
        return HandlerImpl { mac: None };
    }
    if !images.any() {
        let n = format_report(&WifiReport::FirmwareErr, &mut line);
        paint(&line[..n]);
        return HandlerImpl { mac: None };
    }
    let regs = memory_region_symbol!(mt7921_regs_vaddr: *mut u8).as_ptr();
    let dma = memory_region_symbol!(wifi_dma_vaddr: *mut u8).as_ptr();
    beat_arm();
    let mac = Mac::boot(regs, dma, &images);
    beat_disarm();
    paint_stage(mac.stage());
    dump::paint_mac_dump(mac.before(), mac.after());
    HandlerImpl { mac: Some(mac) }
}

/// The status line is painted only after the poll. A moving dot fills that wait.
fn beat_arm() {
    MARK.arm();
    paint_working(0);
}

fn beat_disarm() {
    MARK.disarm();
}

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

pub(crate) fn step_cmd() {
    show(1, ROW_CMD, b"cpu: write cmd");
}

pub(crate) fn step_desc() {
    show(2, ROW_DESC, b"ram: write desc");
}

pub(crate) fn step_bell() {
    show(3, ROW_BELL, b"wfdma: doorbell");
}

pub(crate) fn step_fetch() {
    show(PHASE_FETCH, ROW_FETCH, b"wfdma: fetch");
}

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

fn paint_fetch(tick: u32) {
    let mut line = [0u8; 24];
    let n = working_line(b"wfdma: fetch", tick, &mut line);
    paint_at(ROW_FETCH, &line[..n]);
}

fn paint_working(tick: u32) {
    let mut line = [0u8; 16];
    let n = working_line(b"mt7921", tick, &mut line);
    paint(&line[..n]);
}

fn paint_stage(stage: Stage) {
    let mut line = [0u8; 40];
    let n = stage.paint(&mut line);
    paint(&line[..n]);
    if let Stage::FwSem(Sem::Tx(stop)) = stage {
        let n = write_slot_line(&mut line, stop);
        paint_at(ROW_SLOT, &line[..n]);
        let mut wide = [0u8; 32];
        let n = write_base_line(&mut wide, stop);
        paint_at(ROW_BASE, &wide[..n]);
        let n = write_hold_line(&mut wide, stop);
        paint_at(ROW_HOLD, &wide[..n]);
    }
}

fn paint(text: &[u8]) {
    paint_at(ROW, text);
}

fn paint_at(row: usize, text: &[u8]) {
    let mut line = [b' '; 48];
    let n = text.len().min(line.len());
    line[..n].copy_from_slice(&text[..n]);
    let width = n.max(24);
    paint_line(row, &line[..width]);
}

/// A dump row, padded to the panel width so a repaint erases the old row.
pub(crate) fn paint_full(row: usize, text: &[u8]) {
    let mut line = [b' '; PANEL_COLS];
    let n = text.len().min(PANEL_COLS);
    line[..n].copy_from_slice(&text[..n]);
    paint_line(row, &line);
}

fn paint_line(row: usize, line: &[u8]) {
    let mode = memory_region_symbol!(mode_record_vaddr: *mut u8).as_ptr();
    let frame = memory_region_symbol!(fb_vaddr: *mut u8).as_ptr();
    let record = unsafe { core::slice::from_raw_parts(mode, 40) };
    let bytes = usize::try_from(FRAMEBUFFER_BYTES).expect("framebuffer fits");
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, bytes) };
    paint_row(record, frame, row, line);
}

fn posted_key() -> Option<([u8; 32], usize, [u8; 64], usize)> {
    let ptr = memory_region_symbol!(wifi_key_vaddr: *mut u8).as_ptr();
    let page = unsafe { core::slice::from_raw_parts(ptr, 0x1000) };
    let (ssid, psk) = read_key(page)?;
    let mut name = [0u8; 32];
    let mut pass = [0u8; 64];
    name[..ssid.len()].copy_from_slice(ssid);
    pass[..psk.len()].copy_from_slice(psk);
    Some((name, ssid.len(), pass, psk.len()))
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(IRQ) {
            IRQ.irq_ack().expect("ack mt7921");
        }
        if channels.contains(KEY)
            && let Some(mac) = self.mac.as_mut()
        {
            if mac.ready()
                && let Some((name, name_len, pass, pass_len)) = posted_key()
            {
                let stage = mac.associate(&name[..name_len], &pass[..pass_len], paint_stage);
                paint_stage(stage);
            } else if !mac.ready() {
                paint_stage(mac.stage());
            }
        }
        Ok(())
    }
}
