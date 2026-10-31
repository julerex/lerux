//! MT7921 at `03:00.0`. The MAC follows the Linux v6.8 mt76 sequence.
//!
//! Init paints `own`, `fw sem`, `fw patch`, `fw scatter`, `fw ram`, `fw start`,
//! `fw chip`, or `mac` when that step fails, and `wait` when the
//! firmware has reached the normal state. The shell then writes the network
//! name and passphrase and notifies channel 1. Channel 0 stays the MSI.
//! Association paints `scan`, `auth`, `assoc`, `eapol`, `dhcp`, and `ping`.

#![no_std]
#![no_main]

mod device;
mod join;

use sel4_microkit::{
    memory_region_symbol, protection_domain, Channel, ChannelSet, Handler, Infallible,
};

use lerux_desk::FRAMEBUFFER_BYTES;
use lerux_swift::{
    bar_line, class_code, format_report, load_header, memory_bar, mt7921::Stage, paint_row,
    read_key, vendor_device, WifiReport, MT7921_BAR, MT7921_BDF, MT7921_IDS,
};

use device::{Images, Mac};

const ROW: usize = 22;
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
    let mac = Mac::boot(regs, dma, images);
    paint_stage(mac.stage());
    HandlerImpl { mac: Some(mac) }
}

fn paint_stage(stage: Stage) {
    let mut line = [0u8; 40];
    let n = stage.paint(&mut line);
    paint(&line[..n]);
}

fn paint(text: &[u8]) {
    let mode = memory_region_symbol!(mode_record_vaddr: *mut u8).as_ptr();
    let frame = memory_region_symbol!(fb_vaddr: *mut u8).as_ptr();
    let record = unsafe { core::slice::from_raw_parts(mode, 40) };
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, FRAMEBUFFER_BYTES as usize) };
    paint_row(record, frame, ROW, text);
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
