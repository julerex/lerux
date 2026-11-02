//! Bluetooth HCI on the AMD xHCI controllers at `04:00.3` and `04:00.4`.
//!
//! The radio is the USB interface class `0xE0` / `0x01` / `0x01`. Wi-Fi firmware
//! is a different file. `bt: reset ok` is painted only when that interface's
//! command-complete event acknowledges HCI Reset. An empty blob paints
//! `bt: firmware err`. A controller that will not run paints `bt: halt`. No
//! addressed device paints `bt: port`, how many ports reported a connection, and
//! the furthest stage: `reset`, `slot`, `addr`, or `desc`. An addressed device
//! that is not the HCI interface paints `bt: id` with the
//! vendor, product, and class. A reset that does not complete paints
//! `bt: event`. A capability word outside `0x0100..=0x0120` paints `bt: ver`
//! and that word.

#![no_std]
#![no_main]

use sel4_microkit::{
    memory_region_symbol, protection_domain, Channel, ChannelSet, Handler, Infallible,
};

use lerux_desk::FRAMEBUFFER_BYTES;
use lerux_swift::{
    bt_bring_up, bt_line, class_code, fail_line, further, load_header, vendor_device, version_line,
    BtReport, XferFail, XHCI_BDFS, XHCI_CLASS,
};

mod xhci;

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
    let dma = memory_region_symbol!(xhci_dma_vaddr: *mut u8).as_ptr();
    let (live, foreign) = controllers_live(page, &regs);
    let mut text = [0u8; 24];
    let n = bring_up_text(&regs, dma, live, foreign, &mut text);
    paint(&text[..n]);
    HandlerImpl {
        irq0: Channel::new(0),
        irq1: Channel::new(1),
    }
}

fn controllers_live(page: &[u8], regs: &[*mut u8; 2]) -> (bool, Option<u16>) {
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
    (live, foreign)
}

fn bring_up_text(
    regs: &[*mut u8; 2],
    dma: *mut u8,
    live: bool,
    foreign: Option<u16>,
    text: &mut [u8],
) -> usize {
    if !live {
        if let Some(version) = foreign {
            return version_line(version, text);
        }
        return copy_line(text, bt_line(&BtReport::Absent));
    }
    if FIRMWARE.is_empty() {
        return copy_line(text, bt_line(&BtReport::FirmwareErr));
    }
    let mut event = [0u8; 16];
    match reset_either(regs, dma, &mut event) {
        Ok(n) => copy_line(text, bt_line(&bt_bring_up(true, true, Some(&event[..n])))),
        Err(fail) => fail_line(&fail, text),
    }
}

fn reset_either(regs: &[*mut u8; 2], dma: *mut u8, event: &mut [u8]) -> Result<usize, XferFail> {
    let mut fail = XferFail::Halt;
    for regs in *regs {
        match xhci::poll_reset(regs, dma, event) {
            Ok(n) => return Ok(n),
            Err(next) => fail = further(fail, next),
        }
    }
    Err(fail)
}

#[expect(clippy::cast_ptr_alignment, reason = "2-byte HCIVERSION")]
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
    let bytes = usize::try_from(FRAMEBUFFER_BYTES).expect("framebuffer fits");
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, bytes) };
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
