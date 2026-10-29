//! MT7921 at `03:00.0`. Firmware and the pre-shared key stay out of git.
//!
//! `bring_up` prints `ping ok` only after association, DHCP, and an echo reply.
//! An empty firmware blob paints `mt7921: firmware err`.

#![no_std]
#![no_main]

use sel4::with_ipc_buffer_mut;
use sel4_microkit::{
    memory_region_symbol, protection_domain, var, Channel, ChannelSet, Handler, Infallible,
};

use lerux_desk::FRAMEBUFFER_BYTES;
use lerux_swift::{
    bring_up, class_code, config_address, credentials, format_report, paint_row, vendor_device,
    Nic, MT7921_BDF, MT7921_IDS,
};

const BASE_IOPORT_CAP: u64 = 394;
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
    let cap_id = *var!(pci_ioport_id: usize = usize::MAX) as u64;
    let addr = *var!(pci_ioport_addr: usize = usize::MAX) as u16;
    let (bus, dev, func) = MT7921_BDF;
    let header = read_header(BASE_IOPORT_CAP + cap_id, addr, bus, dev, func);
    let present = vendor_device(&header).is_some_and(|id| MT7921_IDS.contains(&id));
    let _ = class_code(&header);
    let (ssid, pass) = credentials(WIFI).unwrap_or((b"".as_slice(), b"".as_slice()));
    let mut radio = Radio {
        present,
        firmware: !FIRMWARE.is_empty(),
    };
    let report = bring_up(&mut radio, ssid, pass);
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

fn read_header(cap: u64, addr: u16, bus: u8, dev: u8, func: u8) -> [u8; 64] {
    let mut header = [0u8; 64];
    let data = addr.wrapping_add(4);
    for reg in (0u8..64).step_by(4) {
        out32(cap, addr, config_address(bus, dev, func, reg));
        let word = in32(cap, data);
        let offset = usize::from(reg);
        header[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
    }
    header
}

fn out32(cap: u64, port: u16, value: u32) {
    with_ipc_buffer_mut(|ipc| {
        ipc.inner_mut()
            .seL4_X86_IOPort_Out32(cap, u64::from(port), u64::from(value));
    });
}

fn in32(cap: u64, port: u16) -> u32 {
    with_ipc_buffer_mut(|ipc| {
        let ret = ipc.inner_mut().seL4_X86_IOPort_In32(cap, port);
        ret.result
    })
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
