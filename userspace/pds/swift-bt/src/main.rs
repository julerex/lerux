//! Bluetooth HCI on the AMD xHCI controllers at `04:00.3` and `04:00.4`.
//!
//! The radio is the USB interface class `0xE0` / `0x01` / `0x01`. Wi-Fi firmware
//! is a different file. `bt: reset ok` is painted only when that interface's
//! command-complete event acknowledges HCI Reset after the Bluetooth firmware
//! is present. This image has no transfer yet, so a live controller with an
//! empty blob paints `bt: firmware err`. A capability word outside
//! `0x0100..=0x0120` paints `bt: ver` and that word.

#![no_std]
#![no_main]

use sel4_microkit::{
    memory_region_symbol, protection_domain, Channel, ChannelSet, Handler, Infallible,
};

use lerux_desk::FRAMEBUFFER_BYTES;
use lerux_swift::{
    bt_bring_up, bt_line, class_code, hci_reset, load_header, vendor_device, version_line,
    BtReport, XHCI_BDFS, XHCI_CLASS,
};

const ROW: usize = 24;
const FIRMWARE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mt7921-bt.bin"));

struct HandlerImpl {
    irq0: Channel,
    irq1: Channel,
}

#[protection_domain]
fn init() -> HandlerImpl {
    let page = memory_region_symbol!(pci_headers_vaddr: *mut u8).as_ptr();
    let page = unsafe { core::slice::from_raw_parts(page, 0x1000) };
    let regs = [
        memory_region_symbol!(xhci0_regs_vaddr: *mut u8).as_ptr(),
        memory_region_symbol!(xhci1_regs_vaddr: *mut u8).as_ptr(),
    ];
    let _dma = memory_region_symbol!(xhci_dma_vaddr: *mut u8).as_ptr();
    let mut live = false;
    let mut foreign = None;
    for (index, (bus, dev, func)) in XHCI_BDFS.iter().copied().enumerate() {
        let header = load_header(page, bus, dev, func);
        if vendor_device(&header).is_some() && class_code(&header) == Some(XHCI_CLASS) {
            let version = xhci_version(regs[index]);
            if (0x0100..=0x0120).contains(&version) {
                live = true;
            } else if foreign.is_none() {
                foreign = Some(version);
            }
        }
    }
    // Staged until a control transfer can carry it. The acknowledgement parser
    // rejects every other event.
    let _packet = hci_reset();
    let mut line = [0u8; 24];
    let n = if live {
        copy_line(
            &mut line,
            bt_line(&bt_bring_up(true, !FIRMWARE.is_empty(), None)),
        )
    } else if let Some(version) = foreign {
        version_line(version, &mut line)
    } else {
        copy_line(&mut line, bt_line(&BtReport::Absent))
    };
    paint(&line[..n]);
    HandlerImpl {
        irq0: Channel::new(0),
        irq1: Channel::new(1),
    }
}

fn xhci_version(regs: *mut u8) -> u16 {
    unsafe { regs.add(2).cast::<u16>().read_volatile() }
}

fn copy_line(dst: &mut [u8], src: &[u8]) -> usize {
    let n = src.len().min(dst.len());
    dst[..n].copy_from_slice(&src[..n]);
    n
}

fn paint(text: &[u8]) {
    let mode = memory_region_symbol!(mode_record_vaddr: *mut u8).as_ptr();
    let frame = memory_region_symbol!(fb_vaddr: *mut u8).as_ptr();
    let record = unsafe { core::slice::from_raw_parts(mode, 40) };
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, FRAMEBUFFER_BYTES as usize) };
    lerux_swift::paint_row(record, frame, ROW, text);
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(self.irq0) {
            self.irq0.irq_ack().expect("ack xhci0");
        }
        if channels.contains(self.irq1) {
            self.irq1.irq_ack().expect("ack xhci1");
        }
        Ok(())
    }
}
