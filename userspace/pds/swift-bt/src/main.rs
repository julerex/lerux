//! Bluetooth HCI on the AMD xHCI controllers at `04:00.3` and `04:00.4`.
//!
//! The radio is the USB interface class `0xE0` / `0x01` / `0x01`. Wi-Fi firmware
//! is a different file. `bt: reset ok` is painted only when that interface's
//! command-complete event acknowledges HCI Reset after the Bluetooth firmware
//! is present. This image has no transfer yet, so a live controller with an
//! empty blob paints `bt: firmware err`.

#![no_std]
#![no_main]

use sel4::with_ipc_buffer_mut;
use sel4_microkit::{
    memory_region_symbol, protection_domain, var, Channel, ChannelSet, Handler, Infallible,
};

use lerux_desk::FRAMEBUFFER_BYTES;
use lerux_swift::{
    bt_bring_up, bt_line, class_code, config_address, hci_reset, vendor_device, XHCI_BDFS,
    XHCI_CLASS,
};

const BASE_IOPORT_CAP: u64 = 394;
const ROW: usize = 24;
const BARS: [u32; 2] = [0xfa00_0000, 0xf900_0000];
const FIRMWARE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mt7921-bt.bin"));

struct HandlerImpl {
    irq0: Channel,
    irq1: Channel,
}

#[protection_domain]
fn init() -> HandlerImpl {
    let cap_id = *var!(pci_ioport_id: usize = usize::MAX) as u64;
    let port = *var!(pci_ioport_addr: usize = usize::MAX) as u16;
    let cap = BASE_IOPORT_CAP + cap_id;
    let regs = [
        memory_region_symbol!(xhci0_regs_vaddr: *mut u8).as_ptr(),
        memory_region_symbol!(xhci1_regs_vaddr: *mut u8).as_ptr(),
    ];
    let _dma = memory_region_symbol!(xhci_dma_vaddr: *mut u8).as_ptr();
    let mut present = false;
    for (index, (bus, dev, func)) in XHCI_BDFS.iter().copied().enumerate() {
        let header = read_header(cap, port, bus, dev, func);
        if vendor_device(&header).is_some() && class_code(&header) == Some(XHCI_CLASS) {
            program_bar(cap, port, bus, dev, func, BARS[index]);
            present |= xhci_alive(regs[index]);
        }
    }
    // Staged until a control transfer can carry it. The acknowledgement parser
    // rejects every other event.
    let _packet = hci_reset();
    let report = bt_bring_up(present, !FIRMWARE.is_empty(), None);
    paint(bt_line(&report));
    HandlerImpl {
        irq0: Channel::new(0),
        irq1: Channel::new(1),
    }
}

fn xhci_alive(regs: *mut u8) -> bool {
    let version = unsafe { regs.add(2).cast::<u16>().read_volatile() };
    (0x0100..=0x0120).contains(&version)
}

fn program_bar(cap: u64, port: u16, bus: u8, dev: u8, func: u8, bar: u32) {
    let data = port.wrapping_add(4);
    out32(cap, port, config_address(bus, dev, func, 0x10));
    out32(cap, data, bar);
    out32(cap, port, config_address(bus, dev, func, 0x04));
    let command = in32(cap, data);
    out32(cap, port, config_address(bus, dev, func, 0x04));
    out32(cap, data, command | 0x06);
}

fn paint(text: &[u8]) {
    let mode = memory_region_symbol!(mode_record_vaddr: *mut u8).as_ptr();
    let frame = memory_region_symbol!(fb_vaddr: *mut u8).as_ptr();
    let record = unsafe { core::slice::from_raw_parts(mode, 40) };
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, FRAMEBUFFER_BYTES as usize) };
    lerux_swift::paint_row(record, frame, ROW, text);
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
        if channels.contains(self.irq0) {
            self.irq0.irq_ack().expect("ack xhci0");
        }
        if channels.contains(self.irq1) {
            self.irq1.irq_ack().expect("ack xhci1");
        }
        Ok(())
    }
}
