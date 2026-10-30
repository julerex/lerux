//! MT7921 at `03:00.0`. Firmware and the pre-shared key stay out of git.
//!
//! `bring_up` prints `ping ok` only after association, DHCP, and an echo reply.
//! An empty firmware blob paints `mt7921: firmware err`. The image maps
//! `0x82000000`. A header that names another address paints `mt7921: bar`.
//! This window still has no MediaTek MAC program, so a present blob does not
//! accept a frame.

#![no_std]
#![no_main]

use sel4_microkit::{
    memory_region_symbol, protection_domain, Channel, ChannelSet, Handler, Infallible,
};

use lerux_desk::FRAMEBUFFER_BYTES;
use lerux_swift::{
    bring_up, class_code, credentials, format_report, load_header, memory_bar, paint_row,
    vendor_device, Nic, WifiReport, MT7921_BAR, MT7921_BDF, MT7921_IDS,
};

const ROW: usize = 22;
const WIFI: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/wifi.txt"));
const FIRMWARE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mt7921.bin"));

struct HandlerImpl {
    irq: Channel,
}

struct Radio {
    present: bool,
    firmware: bool,
}

impl Nic for Radio {
    fn present(&self) -> bool {
        self.present
    }
    fn firmware(&self) -> bool {
        self.firmware
    }
    fn tx(&mut self, _frame: &[u8]) -> bool {
        // The sDDF enqueue is in `enqueue_frame`, called by a radio that accepts
        // the frame. This BAR has no MediaTek MAC program, so the frame stays off
        // the queue and the panel does not claim a ping.
        false
    }
    fn rx(&mut self) -> Option<&[u8]> {
        None
    }
}

#[protection_domain]
fn init() -> HandlerImpl {
    let page = memory_region_symbol!(pci_headers_vaddr: *mut u8).as_ptr();
    let page = unsafe { core::slice::from_raw_parts(page, 0x1000) };
    let (bus, dev, func) = MT7921_BDF;
    let header = load_header(page, bus, dev, func);
    let present = vendor_device(&header).is_some_and(|id| MT7921_IDS.contains(&id));
    let _ = class_code(&header);
    let (ssid, pass) = credentials(WIFI).unwrap_or((b"".as_slice(), b"".as_slice()));
    let mut radio = Radio {
        present,
        firmware: !FIRMWARE.is_empty(),
    };
    let report = if present && memory_bar(&header) != Some(MT7921_BAR) {
        WifiReport::Bar
    } else {
        bring_up(&mut radio, ssid, pass)
    };
    let mut line = [0u8; 40];
    let n = format_report(&report, &mut line);
    paint(&line[..n]);
    let _regs = memory_region_symbol!(mt7921_regs_vaddr: *mut u8).as_ptr();
    HandlerImpl {
        irq: Channel::new(0),
    }
}

fn paint(text: &[u8]) {
    let mode = memory_region_symbol!(mode_record_vaddr: *mut u8).as_ptr();
    let frame = memory_region_symbol!(fb_vaddr: *mut u8).as_ptr();
    let record = unsafe { core::slice::from_raw_parts(mode, 40) };
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, FRAMEBUFFER_BYTES as usize) };
    paint_row(record, frame, ROW, text);
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(self.irq) {
            self.irq.irq_ack().expect("ack mt7921");
        }
        Ok(())
    }
}
