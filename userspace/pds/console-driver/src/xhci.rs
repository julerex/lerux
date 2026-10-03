//! Intel xHCI host and one HID boot keyboard.
//!
//! The Z97 image maps the controller at `0xf7e20000` and a 256 KiB DMA window
//! at `0x62000000`. Bring-up takes the controller from firmware (legacy SMI
//! and, on Intel, the USB2/USB3 port-routing registers), then speaks the boot
//! protocol. Hubs are not walked. PS/2 stays in charge when this returns
//! without a keyboard.

use core::sync::atomic::{fence, Ordering};

use log::info;
use sel4::with_ipc_buffer_mut;
use sel4_microkit::{memory_region_symbol, var};

use crate::{
    hid::{self, HidState},
    usb::{self, BootKeyboard},
};

const DMA_PHYS: u64 = 0x6200_0000;
const MMIO_PHYS: u32 = 0xf7e2_0000;
const DMA_LEN: usize = 0x4_0000;
const BASE_IOPORT_CAP: u64 = 394;

/// Microkit SDF `vector` for the xHCI MSI. Delivery vector is [`usb::msi_data`].
const SDF_VECTOR: u32 = 50;
const MSI_ADDR: u32 = 0xfee0_0000;

const OFF_DCBAA: usize = 0x0000;
/// 32 scratchpad pointers. Kept clear of the event-ring segment table at `0x100`.
const OFF_SCRATCH: usize = 0x0800;
const OFF_ERST: usize = 0x0100;
const OFF_DATA: usize = 0x0200;
const OFF_REPORT: usize = 0x0400;
const OFF_OUT: usize = 0x1000;
const OFF_IN: usize = 0x2000;
const OFF_EV: usize = 0x3000;
const OFF_CMD: usize = 0x4000;
const OFF_EP0: usize = 0x5000;
const OFF_INTR: usize = 0x6000;
const OFF_SCRATCH_PAGE: usize = 0x1_0000;
const MAX_SCRATCH: u32 = 32;

const RING_LAST: u8 = 31;
const EV_LEN: u8 = 64;
const SPIN: u32 = 500_000;

const TRB_NORMAL: u32 = 1;
const TRB_SETUP: u32 = 2;
const TRB_DATA: u32 = 3;
const TRB_STATUS: u32 = 4;
const TRB_LINK: u32 = 6;
const TRB_ENABLE_SLOT: u32 = 9;
const TRB_DISABLE_SLOT: u32 = 10;
const TRB_ADDRESS: u32 = 11;
const TRB_CONFIGURE: u32 = 12;
const TRB_EVALUATE: u32 = 13;
const TRB_RESET_EP: u32 = 14;
const TRB_SET_DEQ: u32 = 16;
const TRB_TRANSFER: u32 = 32;
const TRB_COMMAND: u32 = 33;
const TRB_PORT_STATUS: u32 = 34;

const CC_SUCCESS: u32 = 1;
const CC_STALL: u32 = 6;
const CC_SHORT: u32 = 13;

const TC: u32 = 1 << 1;
const ISP: u32 = 1 << 2;
const CHAIN: u32 = 1 << 4;
const IOC: u32 = 1 << 5;
const IDT: u32 = 1 << 6;

const PORT_CHANGE: u32 = 0x7f << 17;

#[derive(Clone, Copy)]
enum Ring {
    Cmd,
    Ep0,
    Intr,
}

enum Ctl {
    Done(usize),
    Stall,
    Fail,
}

struct PciFn {
    dev: u8,
    func: u8,
    vendor: u16,
}

/// Armed boot keyboard, or a cold controller when [`Self::ready`] is false.
pub struct UsbKbd {
    regs: *mut u8,
    dma: *mut u8,
    pci_cap: u64,
    pci_port: u16,
    attached: bool,
    op: u32,
    db: u32,
    iman: u32,
    erdp: u32,
    ctx_bytes: usize,
    /// Implemented BAR length. qemu-xhci is 16 KiB; the Z97 controller is 64 KiB.
    mmio_len: u32,
    max_ports: u8,
    max_slots: u8,
    hcc: u32,
    hcs2: u32,
    slot: u8,
    dci: u8,
    keys: HidState,
    ready: bool,
    online: bool,
    broken: bool,
    cmd_i: u8,
    cmd_c: u8,
    ep0_i: u8,
    ep0_c: u8,
    intr_i: u8,
    intr_c: u8,
    ev_i: u8,
    ev_c: u8,
}

// Single-threaded PD. The pointers are this PD's register and DMA windows.
unsafe impl Send for UsbKbd {}

impl UsbKbd {
    pub fn bring_up() -> Self {
        let mut kbd = Self::from_system();
        if !kbd.attached {
            info!("console-driver: xhci none");
            return kbd;
        }
        if !kbd.start() {
            kbd.stop();
            return kbd;
        }
        if !kbd.enumerate() {
            info!("console-driver: xhci no keyboard");
            kbd.stop();
            return kbd;
        }
        kbd.ack_irq();
        kbd
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    /// Drain completion events into `out` as shell bytes.
    pub fn poll(&mut self, out: &mut [u8]) -> usize {
        if !self.ready {
            return 0;
        }
        let mut n = 0;
        for _ in 0..8 {
            let Some(ev) = self.pop_event() else {
                break;
            };
            let kind = trb_kind(ev);
            if kind == TRB_PORT_STATUS {
                self.ack_port_event(ev);
                continue;
            }
            if kind != TRB_TRANSFER || endpoint_id(ev) != u32::from(self.dci) {
                continue;
            }
            let code = completion(ev);
            if code == CC_SUCCESS || code == CC_SHORT {
                let mut report = [0u8; 8];
                self.read_dma(OFF_REPORT, &mut report);
                let mut decoded = [0u8; 6];
                let got = hid::decode(&mut self.keys, &report, &mut decoded);
                for &byte in decoded.iter().take(got) {
                    if n < out.len() {
                        out[n] = byte;
                        n += 1;
                    }
                }
            } else {
                self.recover_endpoint(Ring::Intr, self.dci);
            }
            if self.broken {
                break;
            }
            self.queue_intr();
        }
        n
    }

    /// Let the next MSI through. Safe to call when the ring was already drained.
    pub fn ack_irq(&mut self) {
        if !self.online || self.op == 0 {
            return;
        }
        // IP is write-1-to-clear. IE stays set.
        self.write32(self.iman, 0x3);
        // USBSTS.EINT only. Other write-1-to-clear bits stay 0.
        self.write32(self.op + 4, 1 << 3);
    }

    fn from_system() -> Self {
        let regs = memory_region_symbol!(xhci_regs_vaddr: *mut u8).as_ptr();
        let dma = memory_region_symbol!(xhci_dma_vaddr: *mut u8).as_ptr();
        let pci_id = *var!(pci_ioport_id: usize = usize::MAX);
        let pci_port = *var!(pci_ioport_addr: usize = usize::MAX);
        let attached =
            !regs.is_null() && !dma.is_null() && pci_id != usize::MAX && pci_port != usize::MAX;
        Self {
            regs,
            dma,
            pci_cap: BASE_IOPORT_CAP + pci_id as u64,
            pci_port: pci_port as u16,
            attached,
            op: 0,
            db: 0,
            iman: 0,
            erdp: 0,
            ctx_bytes: 32,
            mmio_len: 0,
            max_ports: 0,
            max_slots: 0,
            hcc: 0,
            hcs2: 0,
            slot: 0,
            dci: 0,
            keys: HidState::default(),
            ready: false,
            online: false,
            broken: false,
            cmd_i: 0,
            cmd_c: 1,
            ep0_i: 0,
            ep0_c: 1,
            intr_i: 0,
            intr_c: 1,
            ev_i: 0,
            ev_c: 1,
        }
    }

    fn start(&mut self) -> bool {
        let Some(pci) = self.find_xhci() else {
            info!("console-driver: xhci none");
            return false;
        };
        info!("console-driver: xhci 00:{:02x}.{}", pci.dev, pci.func);
        self.assign_bar(&pci);
        self.online = true;
        if pci.vendor == 0x8086 {
            self.intel_route(&pci);
        }
        if !self.read_caps() {
            info!("console-driver: xhci regs");
            return false;
        }
        self.legacy_handoff();
        if !self.halt() {
            info!("console-driver: xhci halt timeout");
            return false;
        }
        if !self.reset_controller() {
            info!("console-driver: xhci reset timeout");
            return false;
        }
        if self.read32(self.op + 8) != 1 {
            info!("console-driver: xhci pagesize");
            return false;
        }
        self.zero_dma();
        if !self.program_rings() {
            return false;
        }
        if !self.program_msi(&pci) {
            info!("console-driver: xhci no msi");
            return false;
        }
        // QEMU arms MSI-X only when interrupter 0's IE bit is written while
        // MSI-X is already enabled. `program_rings` sets IE earlier.
        self.write32(self.iman, 0x2);
        self.write32(self.op, (1 << 0) | (1 << 2));
        if self.wait_status(|status| status & 1 == 0) {
            return true;
        }
        info!("console-driver: xhci run timeout");
        false
    }

    fn stop(&mut self) {
        if !self.online || self.op == 0 {
            return;
        }
        let cmd = self.read32(self.op);
        self.write32(self.op, cmd & !1);
        self.ready = false;
    }

    fn enumerate(&mut self) -> bool {
        self.power_ports();
        self.wait_connect();
        for port in 1..=self.max_ports {
            if self.broken {
                break;
            }
            if self.try_port(port) {
                return true;
            }
        }
        false
    }

    fn try_port(&mut self, port: u8) -> bool {
        let raw = self.read32(self.portsc(port));
        if raw & 1 == 0 {
            return false;
        }
        let Some(speed) = self.reset_port(port) else {
            return false;
        };
        if !self.enable_slot() {
            return false;
        }
        if !self.address(port, speed) || !self.bind_keyboard(speed) {
            self.disable_slot();
            return false;
        }
        true
    }

    fn bind_keyboard(&mut self, speed: u8) -> bool {
        let Some(desc) = self.device_descriptor() else {
            return false;
        };
        let vid = u16::from(desc[8]) | (u16::from(desc[9]) << 8);
        let pid = u16::from(desc[10]) | (u16::from(desc[11]) << 8);
        let Some(kbd) = self.config_keyboard() else {
            return false;
        };
        if !self.set_boot_protocol(&kbd) {
            return false;
        }
        if !self.configure_interrupt(&kbd, speed) {
            return false;
        }
        self.dci = usb::endpoint_dci(kbd.ep_addr);
        self.queue_intr();
        self.ready = true;
        info!("console-driver: usb keyboard {vid:04x}:{pid:04x}");
        true
    }

    fn device_descriptor(&mut self) -> Option<[u8; 18]> {
        let n = self.ctrl_in(0x0100, 8)?;
        if n < 8 {
            return None;
        }
        let mut first = [0u8; 8];
        self.read_dma(OFF_DATA, &mut first);
        let mps = u16::from(first[7]);
        if mps != 0 && mps != self.ep_mps() && !self.evaluate_mps(mps) {
            return None;
        }
        let n = self.ctrl_in(0x0100, 18)?;
        if n < 18 {
            return None;
        }
        let mut desc = [0u8; 18];
        self.read_dma(OFF_DATA, &mut desc);
        Some(desc)
    }

    fn config_keyboard(&mut self) -> Option<BootKeyboard> {
        let n = self.ctrl_in(0x0200, 9)?;
        if n < 9 {
            return None;
        }
        let mut hdr = [0u8; 9];
        self.read_dma(OFF_DATA, &mut hdr);
        let total = usize::from(u16::from(hdr[2]) | (u16::from(hdr[3]) << 8)).clamp(9, 512);
        let n = self.ctrl_in(0x0200, u16::try_from(total).unwrap_or(9))?;
        if n < 9 {
            return None;
        }
        let mut cfg = [0u8; 512];
        let take = n.min(cfg.len());
        self.read_dma(OFF_DATA, &mut cfg[..take]);
        usb::find_boot_keyboard(&cfg[..take])
    }

    fn set_boot_protocol(&mut self, kbd: &BootKeyboard) -> bool {
        let iface = u16::from(kbd.interface);
        if !matches!(
            self.control(0x00, 9, u16::from(kbd.config_value), 0, 0),
            Ctl::Done(_)
        ) {
            return false;
        }
        if !matches!(self.control(0x21, 0x0b, 0, iface, 0), Ctl::Done(_)) {
            return false;
        }
        // Duration 0: report only when a key changes. A stall is normal.
        matches!(
            self.control(0x21, 0x0a, 0, iface, 0),
            Ctl::Done(_) | Ctl::Stall
        )
    }

    fn configure_interrupt(&mut self, kbd: &BootKeyboard, speed: u8) -> bool {
        let dci = usb::endpoint_dci(kbd.ep_addr);
        if dci == 0 || kbd.max_packet < 8 {
            return false;
        }
        self.zero_range(OFF_IN, 33 * self.ctx_bytes);
        self.copy_dev_to_input(0, 1);
        let dw0 = self.read_ctx(true, 1, 0);
        let dw0 = (dw0 & !(0x1f << 27)) | (u32::from(dci) << 27);
        self.write_ctx(true, 1, 0, dw0);
        self.intr_i = 0;
        self.intr_c = 1;
        self.zero_range(OFF_INTR, 32 * 16);
        let interval = usb::xhci_interval(speed, kbd.interval);
        self.write_ep(
            usize::from(dci) + 1,
            7,
            kbd.max_packet,
            interval,
            OFF_INTR,
            8,
        );
        self.write_ctx(true, 0, 1, (1 << 0) | (1 << dci));
        self.command_ok(TRB_CONFIGURE)
    }

    fn address(&mut self, port: u8, speed: u8) -> bool {
        self.zero_range(OFF_OUT, 32 * self.ctx_bytes);
        self.zero_range(OFF_IN, 33 * self.ctx_bytes);
        self.zero_range(OFF_EP0, 32 * 16);
        self.ep0_i = 0;
        self.ep0_c = 1;
        self.write_dma_u64(
            OFF_DCBAA + usize::from(self.slot) * 8,
            DMA_PHYS + OFF_OUT as u64,
        );
        self.write_ctx(true, 0, 1, 0b11);
        let dw0 = (u32::from(speed) << 20) | (1 << 27);
        self.write_ctx(true, 1, 0, dw0);
        self.write_ctx(true, 1, 1, u32::from(port) << 16);
        let mps: u16 = if speed >= 3 { 64 } else { 8 };
        self.write_ep(2, 4, mps, 0, OFF_EP0, 8);
        self.command_ok(TRB_ADDRESS)
    }

    fn evaluate_mps(&mut self, mps: u16) -> bool {
        self.zero_range(OFF_IN, 33 * self.ctx_bytes);
        self.copy_dev_to_input(1, 2);
        let dw1 = self.read_ctx(true, 2, 1);
        self.write_ctx(true, 2, 1, (dw1 & 0xffff) | (u32::from(mps) << 16));
        self.write_ctx(true, 0, 1, 1 << 1);
        self.command_ok(TRB_EVALUATE)
    }

    fn enable_slot(&mut self) -> bool {
        let Some(ev) = self.submit_cmd(0, 0, 0, trb_type(TRB_ENABLE_SLOT)) else {
            return false;
        };
        if completion(ev) != CC_SUCCESS {
            return false;
        }
        self.slot = (ev[3] >> 24) as u8;
        self.slot != 0
    }

    fn disable_slot(&mut self) {
        if self.slot == 0 {
            return;
        }
        let d3 = trb_type(TRB_DISABLE_SLOT) | (u32::from(self.slot) << 24);
        let _ = self.submit_cmd(0, 0, 0, d3);
        self.slot = 0;
    }

    fn command_ok(&mut self, kind: u32) -> bool {
        let ptr = DMA_PHYS + OFF_IN as u64;
        let d3 = trb_type(kind) | (u32::from(self.slot) << 24);
        let Some(ev) = self.submit_cmd(ptr as u32, (ptr >> 32) as u32, 0, d3) else {
            return false;
        };
        completion(ev) == CC_SUCCESS
    }

    fn ctrl_in(&mut self, value: u16, len: u16) -> Option<usize> {
        match self.control(0x80, 6, value, 0, len) {
            Ctl::Done(n) => Some(n),
            Ctl::Stall | Ctl::Fail => None,
        }
    }

    fn control(&mut self, bm: u8, req: u8, value: u16, index: u16, len: u16) -> Ctl {
        let trbs = if len == 0 { 2 } else { 3 };
        self.ensure_room(Ring::Ep0, trbs);
        let setup_d0 = u32::from(bm) | (u32::from(req) << 8) | (u32::from(value) << 16);
        let setup_d1 = u32::from(index) | (u32::from(len) << 16);
        let trt = if len == 0 {
            0
        } else if bm & 0x80 != 0 {
            3
        } else {
            2
        };
        let setup_d3 = CHAIN | IDT | trb_type(TRB_SETUP) | (trt << 16);
        self.push_trb(Ring::Ep0, setup_d0, setup_d1, 8, setup_d3);

        let mut data_trb = 0u64;
        if len > 0 {
            let buf = DMA_PHYS + OFF_DATA as u64;
            let dir = if bm & 0x80 != 0 { 1 << 16 } else { 0 };
            let isp = if bm & 0x80 != 0 { ISP } else { 0 };
            let data_d3 = isp | CHAIN | trb_type(TRB_DATA) | dir;
            data_trb = self.push_trb(
                Ring::Ep0,
                buf as u32,
                (buf >> 32) as u32,
                u32::from(len),
                data_d3,
            );
        }
        let status_dir = if len > 0 && bm & 0x80 != 0 {
            0
        } else {
            1 << 16
        };
        let status_d3 = IOC | trb_type(TRB_STATUS) | status_dir;
        let status_trb = self.push_trb(Ring::Ep0, 0, 0, 0, status_d3);
        self.doorbell(self.slot, 1);

        let mut actual = usize::from(len);
        for _ in 0..SPIN {
            let Some(ev) = self.pop_event() else {
                core::hint::spin_loop();
                continue;
            };
            let kind = trb_kind(ev);
            if kind == TRB_PORT_STATUS {
                self.ack_port_event(ev);
                continue;
            }
            if kind != TRB_TRANSFER {
                continue;
            }
            let ptr = event_ptr(ev) & !0xf;
            let code = completion(ev);
            if data_trb != 0 && ptr == data_trb & !0xf {
                if code == CC_SHORT {
                    let residual = usize::try_from(ev[2] & 0x00ff_ffff).unwrap_or(0);
                    actual = usize::from(len).saturating_sub(residual);
                } else if code != CC_SUCCESS {
                    return self.transfer_failed(code);
                }
                continue;
            }
            if ptr == status_trb & !0xf {
                if code == CC_STALL {
                    self.recover_endpoint(Ring::Ep0, 1);
                    return Ctl::Stall;
                }
                if code == CC_SUCCESS || code == CC_SHORT {
                    return Ctl::Done(actual);
                }
                return Ctl::Fail;
            }
        }
        Ctl::Fail
    }

    fn transfer_failed(&mut self, code: u32) -> Ctl {
        if code == CC_STALL {
            self.recover_endpoint(Ring::Ep0, 1);
            Ctl::Stall
        } else {
            Ctl::Fail
        }
    }

    fn recover_endpoint(&mut self, ring: Ring, dci: u8) {
        let slot = u32::from(self.slot) << 24;
        let ep = u32::from(dci) << 16;
        let reset = trb_type(TRB_RESET_EP) | ep | slot;
        if self.submit_cmd(0, 0, 0, reset).is_none() {
            return;
        }
        let index = self.ring_index(ring);
        let cycle = u32::from(self.ring_cycle(ring));
        let ptr = self.ring_phys(ring) + u64::from(index) * 16;
        let d0 = (ptr as u32 & !0xf) | cycle;
        let d3 = trb_type(TRB_SET_DEQ) | ep | slot;
        let _ = self.submit_cmd(d0, (ptr >> 32) as u32, 0, d3);
    }

    fn queue_intr(&mut self) {
        self.ensure_room(Ring::Intr, 1);
        self.zero_range(OFF_REPORT, 8);
        let buf = DMA_PHYS + OFF_REPORT as u64;
        let d3 = ISP | IOC | trb_type(TRB_NORMAL);
        self.push_trb(Ring::Intr, buf as u32, (buf >> 32) as u32, 8, d3);
        self.doorbell(self.slot, self.dci);
    }

    fn submit_cmd(&mut self, d0: u32, d1: u32, d2: u32, d3: u32) -> Option<[u32; 4]> {
        self.ensure_room(Ring::Cmd, 1);
        let phys = self.push_trb(Ring::Cmd, d0, d1, d2, d3);
        self.doorbell(0, 0);
        let ev = self.wait_completion(phys);
        if ev.is_none() {
            self.broken = true;
        }
        ev
    }

    fn wait_completion(&mut self, trb: u64) -> Option<[u32; 4]> {
        for _ in 0..SPIN {
            let Some(ev) = self.pop_event() else {
                core::hint::spin_loop();
                continue;
            };
            let kind = trb_kind(ev);
            if kind == TRB_PORT_STATUS {
                self.ack_port_event(ev);
                continue;
            }
            if kind == TRB_COMMAND && event_ptr(ev) & !0xf == trb & !0xf {
                return Some(ev);
            }
        }
        None
    }

    fn find_xhci(&self) -> Option<PciFn> {
        for dev in 0..32u8 {
            let id = self.pci_read(dev, 0, 0);
            if id == 0xffff_ffff {
                continue;
            }
            let header = (self.pci_read(dev, 0, 0x0c) >> 16) & 0xff;
            let functions: u8 = if header & 0x80 != 0 { 8 } else { 1 };
            for func in 0..functions {
                let id = self.pci_read(dev, func, 0);
                if id == 0xffff_ffff {
                    continue;
                }
                let class = self.pci_read(dev, func, 8) >> 8;
                if class == 0x0c_0330 {
                    return Some(PciFn {
                        dev,
                        func,
                        vendor: id as u16,
                    });
                }
            }
        }
        None
    }

    fn assign_bar(&mut self, pci: &PciFn) {
        let cmd = self.pci_read(pci.dev, pci.func, 4);
        self.pci_write(pci.dev, pci.func, 4, cmd & !0x3);
        let bar = self.pci_read(pci.dev, pci.func, 0x10);
        let is64 = bar & 0x6 == 0x4;
        self.pci_write(pci.dev, pci.func, 0x10, 0xffff_fff0);
        let lo = self.pci_read(pci.dev, pci.func, 0x10) & 0xffff_fff0;
        if is64 {
            self.pci_write(pci.dev, pci.func, 0x14, 0xffff_ffff);
            self.pci_write(pci.dev, pci.func, 0x14, 0);
        }
        let size = (!lo).wrapping_add(1);
        // A failed size read must not walk off the qemu-xhci window.
        self.mmio_len = if size.is_power_of_two() && (0x1000..=0x1_0000).contains(&size) {
            size
        } else {
            0x4000
        };
        self.pci_write(pci.dev, pci.func, 0x10, MMIO_PHYS);
        if is64 {
            self.pci_write(pci.dev, pci.func, 0x14, 0);
        }
        // Memory decode and bus master. Drop IO decode and interrupt-disable.
        self.pci_write(pci.dev, pci.func, 4, (cmd & !0x407) | 0x6);
    }

    fn intel_route(&self, pci: &PciFn) {
        // Keep USB2 ports on xHCI. HCRST does not clear these PCI bytes.
        let usb2 = self.pci_read(pci.dev, pci.func, 0xd4);
        let usb3 = self.pci_read(pci.dev, pci.func, 0xdc);
        if usb3 != 0xffff_ffff {
            self.pci_write(pci.dev, pci.func, 0xd8, usb3);
        }
        if usb2 != 0xffff_ffff {
            self.pci_write(pci.dev, pci.func, 0xd0, usb2);
        }
    }

    fn read_caps(&mut self) -> bool {
        let caplen = self.read32(0) & 0xff;
        if !(0x20..=0x80).contains(&caplen) || !caplen.is_multiple_of(4) || caplen >= self.mmio_len
        {
            return false;
        }
        let hcs1 = self.read32(4);
        let slots = (hcs1 & 0xff).min(8) as u8;
        let ports = ((hcs1 >> 24) & 0xff).min(30) as u8;
        if slots == 0 || ports == 0 {
            return false;
        }
        let hcc = self.read32(0x10);
        let db = self.read32(0x14) & !0x3;
        let rt = self.read32(0x18) & !0x1f;
        if db == 0 || rt == 0 || rt + 0x40 >= self.mmio_len || db + 4 >= self.mmio_len {
            return false;
        }
        self.op = caplen;
        self.db = db;
        self.iman = rt + 0x20;
        self.erdp = rt + 0x38;
        self.ctx_bytes = if hcc & (1 << 2) == 0 { 32 } else { 64 };
        self.max_slots = slots;
        self.max_ports = ports;
        self.hcc = hcc;
        self.hcs2 = self.read32(8);
        true
    }

    fn legacy_handoff(&self) {
        let mut off = (self.hcc >> 16) << 2;
        if off == 0 {
            return;
        }
        for _ in 0..16 {
            // qemu-xhci implements 16 KiB. A read past that BAR faults the PD.
            if off + 8 >= self.mmio_len {
                return;
            }
            let dw = self.read32(off);
            let id = dw & 0xff;
            let next = (dw >> 8) & 0xff;
            if id == 1 {
                self.write32(off, dw | (1 << 24));
                let mut released = false;
                for _ in 0..SPIN {
                    if self.read32(off) & (1 << 16) == 0 {
                        released = true;
                        break;
                    }
                    core::hint::spin_loop();
                }
                if !released {
                    let cur = self.read32(off);
                    self.write32(off, (cur | (1 << 24)) & !(1 << 16));
                }
                let mut smi = self.read32(off + 4);
                smi |= (0x7 << 1) | (0xff << 5) | (0x7 << 17);
                smi &= !(0x7 << 29);
                self.write32(off + 4, smi);
            }
            if next == 0 {
                break;
            }
            off += next << 2;
        }
    }

    fn halt(&self) -> bool {
        let cmd = self.read32(self.op);
        self.write32(self.op, cmd & !1);
        self.wait_status(|status| status & 1 != 0)
    }

    fn reset_controller(&self) -> bool {
        self.write32(self.op, 1 << 1);
        for _ in 0..SPIN {
            let cmd = self.read32(self.op);
            let status = self.read32(self.op + 4);
            if cmd & (1 << 1) == 0 && status & (1 << 11) == 0 {
                return true;
            }
            core::hint::spin_loop();
        }
        false
    }

    fn program_rings(&mut self) -> bool {
        let scratch = usb::scratchpad_count(self.hcs2);
        if scratch > MAX_SCRATCH {
            info!("console-driver: xhci scratchpad {scratch}");
            return false;
        }
        if scratch > 0 {
            let array = DMA_PHYS + OFF_SCRATCH as u64;
            for i in 0..scratch {
                let page = DMA_PHYS + OFF_SCRATCH_PAGE as u64 + u64::from(i) * 0x1000;
                self.write_dma_u64(OFF_SCRATCH + i as usize * 8, page);
            }
            self.write_dma_u64(OFF_DCBAA, array);
        }
        self.write_dma_u64(OFF_ERST, DMA_PHYS + OFF_EV as u64);
        self.write_dma_u32(OFF_ERST + 8, u32::from(EV_LEN));
        self.write32(self.op + 0x38, u32::from(self.max_slots));
        self.write64(self.op + 0x30, DMA_PHYS + OFF_DCBAA as u64);
        self.write64(self.op + 0x18, (DMA_PHYS + OFF_CMD as u64) | 1);
        self.write32(self.iman + 8, 1);
        self.write64(self.iman + 0x10, DMA_PHYS + OFF_ERST as u64);
        self.write64(self.erdp, DMA_PHYS + OFF_EV as u64);
        self.write32(self.iman, 0x2);
        true
    }

    fn program_msi(&self, pci: &PciFn) -> bool {
        let status = self.pci_read(pci.dev, pci.func, 0x04) >> 16;
        if status & (1 << 4) == 0 {
            return false;
        }
        let mut cap = (self.pci_read(pci.dev, pci.func, 0x34) & 0xff) as u8;
        let mut msix = None;
        for _ in 0..32 {
            if cap < 0x40 {
                break;
            }
            let hdr = self.pci_read(pci.dev, pci.func, cap);
            let id = (hdr & 0xff) as u8;
            let next = ((hdr >> 8) & 0xff) as u8;
            if id == 0x05 {
                // Metal xHCI (8086:8cb1) uses MSI. Leave MSI-X masked off.
                self.enable_msi(pci, cap, hdr);
                info!("console-driver: xhci msi");
                return true;
            }
            if id == 0x11 {
                msix = Some((cap, hdr));
            }
            if next == 0 {
                break;
            }
            cap = next;
        }
        // qemu-xhci on this host has MSI-X and no MSI capability.
        msix.is_some_and(|(cap, hdr)| {
            let ok = self.enable_msix(pci, cap, hdr);
            if ok {
                info!("console-driver: xhci msix");
            }
            ok
        })
    }

    fn enable_msix(&self, pci: &PciFn, cap: u8, hdr: u32) -> bool {
        let masked = ((hdr >> 16) | (1 << 14)) & !(1 << 15);
        self.pci_write(pci.dev, pci.func, cap, (hdr & 0xffff) | (masked << 16));
        let table = self.pci_read(pci.dev, pci.func, cap + 4);
        let bir = table & 0x7;
        let off = table & !0x7;
        if bir != 0 || off.checked_add(16).is_none_or(|end| end > self.mmio_len) {
            return false;
        }
        let data = usb::msi_data(SDF_VECTOR);
        self.write32(off, MSI_ADDR);
        self.write32(off + 4, 0);
        self.write32(off + 8, data);
        self.write32(off + 12, 0);
        let hdr = self.pci_read(pci.dev, pci.func, cap);
        let enabled = ((hdr >> 16) | (1 << 15)) & !(1 << 14);
        self.pci_write(pci.dev, pci.func, cap, (hdr & 0xffff) | (enabled << 16));
        true
    }

    fn enable_msi(&self, pci: &PciFn, cap: u8, hdr: u32) {
        let mut ctrl = (hdr >> 16) & !1 & !(0x7 << 4);
        self.pci_write(pci.dev, pci.func, cap, (hdr & 0xffff) | (ctrl << 16));
        let is64 = ctrl & (1 << 7) != 0;
        let data = usb::msi_data(SDF_VECTOR);
        self.pci_write(pci.dev, pci.func, cap + 4, MSI_ADDR);
        if is64 {
            self.pci_write(pci.dev, pci.func, cap + 8, 0);
            let prev = self.pci_read(pci.dev, pci.func, cap + 12);
            self.pci_write(pci.dev, pci.func, cap + 12, (prev & 0xffff_0000) | data);
        } else {
            let prev = self.pci_read(pci.dev, pci.func, cap + 8);
            self.pci_write(pci.dev, pci.func, cap + 8, (prev & 0xffff_0000) | data);
        }
        ctrl |= 1;
        let hdr = self.pci_read(pci.dev, pci.func, cap);
        self.pci_write(pci.dev, pci.func, cap, (hdr & 0xffff) | (ctrl << 16));
    }

    fn power_ports(&self) {
        for port in 1..=self.max_ports {
            let off = self.portsc(port);
            let raw = self.read32(off);
            self.write32(off, portsc_neutral(raw) | (1 << 9));
        }
    }

    fn wait_connect(&self) {
        for _ in 0..SPIN {
            for port in 1..=self.max_ports {
                if self.read32(self.portsc(port)) & 1 != 0 {
                    return;
                }
            }
            core::hint::spin_loop();
        }
    }

    fn reset_port(&self, port: u8) -> Option<u8> {
        let off = self.portsc(port);
        let raw = self.read32(off);
        self.write32(off, portsc_neutral(raw) | (1 << 9) | (1 << 4));
        for _ in 0..SPIN {
            let cur = self.read32(off);
            let resetting = cur & (1 << 4) != 0;
            let done = cur & (1 << 21) != 0;
            if resetting || !done {
                core::hint::spin_loop();
                continue;
            }
            self.write32(off, portsc_neutral(cur) | (cur & PORT_CHANGE));
            if cur & (1 << 1) == 0 {
                return None;
            }
            let speed = ((cur >> 10) & 0xf) as u8;
            return (speed != 0).then_some(speed);
        }
        None
    }

    fn ack_port_event(&self, ev: [u32; 4]) {
        let port = (ev[0] >> 24) as u8;
        if port == 0 || port > self.max_ports {
            return;
        }
        let off = self.portsc(port);
        let raw = self.read32(off);
        self.write32(off, portsc_neutral(raw) | (raw & PORT_CHANGE));
    }

    fn portsc(&self, port: u8) -> u32 {
        self.op + 0x400 + u32::from(port - 1) * 0x10
    }

    fn wait_status(&self, ready: impl Fn(u32) -> bool) -> bool {
        for _ in 0..SPIN {
            if ready(self.read32(self.op + 4)) {
                return true;
            }
            core::hint::spin_loop();
        }
        false
    }

    fn write_ep(&self, index: usize, kind: u8, mps: u16, interval: u8, ring_off: usize, avg: u16) {
        let phys = DMA_PHYS + ring_off as u64;
        self.write_ctx(true, index, 0, u32::from(interval) << 16);
        let dw1 = (3 << 1) | (u32::from(kind) << 3) | (u32::from(mps) << 16);
        self.write_ctx(true, index, 1, dw1);
        self.write_ctx(true, index, 2, (phys as u32 & !0xf) | 1);
        self.write_ctx(true, index, 3, (phys >> 32) as u32);
        self.write_ctx(true, index, 4, u32::from(avg) | (u32::from(mps) << 16));
    }

    fn ep_mps(&self) -> u16 {
        let dw1 = self.read_ctx(false, 1, 1);
        (dw1 >> 16) as u16
    }

    fn copy_dev_to_input(&self, dev_index: usize, in_index: usize) {
        let words = self.ctx_bytes / 4;
        for dw in 0..words {
            let value = self.read_ctx(false, dev_index, dw);
            self.write_ctx(true, in_index, dw, value);
        }
    }

    fn read_ctx(&self, input: bool, index: usize, dw: usize) -> u32 {
        self.read_dma_u32(self.ctx_off(input, index) + dw * 4)
    }

    fn write_ctx(&self, input: bool, index: usize, dw: usize, value: u32) {
        self.write_dma_u32(self.ctx_off(input, index) + dw * 4, value);
    }

    fn ctx_off(&self, input: bool, index: usize) -> usize {
        let base = if input { OFF_IN } else { OFF_OUT };
        base + index * self.ctx_bytes
    }

    fn ensure_room(&mut self, ring: Ring, count: u8) {
        let index = u16::from(self.ring_index(ring));
        if index + u16::from(count) > u16::from(RING_LAST) {
            self.write_link(ring);
        }
    }

    fn push_trb(&mut self, ring: Ring, d0: u32, d1: u32, d2: u32, d3: u32) -> u64 {
        if self.ring_index(ring) == RING_LAST {
            self.write_link(ring);
        }
        let index = self.ring_index(ring);
        let cycle = u32::from(self.ring_cycle(ring));
        let phys = self.ring_phys(ring) + u64::from(index) * 16;
        write_trb(self.trb_ptr(ring, index), d0, d1, d2, d3 | cycle);
        self.set_ring_index(ring, index + 1);
        phys
    }

    fn write_link(&mut self, ring: Ring) {
        let index = self.ring_index(ring);
        let cycle = u32::from(self.ring_cycle(ring));
        let base = self.ring_phys(ring);
        let d3 = cycle | TC | trb_type(TRB_LINK);
        write_trb(
            self.trb_ptr(ring, index),
            base as u32,
            (base >> 32) as u32,
            0,
            d3,
        );
        self.set_ring_index(ring, 0);
        self.set_ring_cycle(ring, self.ring_cycle(ring) ^ 1);
    }

    fn pop_event(&mut self) -> Option<[u32; 4]> {
        let ptr = self.dma.wrapping_add(OFF_EV + usize::from(self.ev_i) * 16);
        let d3 = unsafe { ptr.cast::<u32>().add(3).read_volatile() };
        if (d3 & 1) != u32::from(self.ev_c) {
            return None;
        }
        fence(Ordering::SeqCst);
        let ev = read_trb(ptr);
        self.ev_i = self.ev_i.wrapping_add(1);
        if self.ev_i == EV_LEN {
            self.ev_i = 0;
            self.ev_c ^= 1;
        }
        let next = DMA_PHYS + OFF_EV as u64 + u64::from(self.ev_i) * 16;
        self.write64(self.erdp, next | (1 << 3));
        Some(ev)
    }

    fn ring_index(&self, ring: Ring) -> u8 {
        match ring {
            Ring::Cmd => self.cmd_i,
            Ring::Ep0 => self.ep0_i,
            Ring::Intr => self.intr_i,
        }
    }

    fn set_ring_index(&mut self, ring: Ring, index: u8) {
        match ring {
            Ring::Cmd => self.cmd_i = index,
            Ring::Ep0 => self.ep0_i = index,
            Ring::Intr => self.intr_i = index,
        }
    }

    fn ring_cycle(&self, ring: Ring) -> u8 {
        match ring {
            Ring::Cmd => self.cmd_c,
            Ring::Ep0 => self.ep0_c,
            Ring::Intr => self.intr_c,
        }
    }

    fn set_ring_cycle(&mut self, ring: Ring, cycle: u8) {
        match ring {
            Ring::Cmd => self.cmd_c = cycle,
            Ring::Ep0 => self.ep0_c = cycle,
            Ring::Intr => self.intr_c = cycle,
        }
    }

    fn ring_phys(&self, ring: Ring) -> u64 {
        DMA_PHYS + self.ring_off(ring) as u64
    }

    fn ring_off(&self, ring: Ring) -> usize {
        match ring {
            Ring::Cmd => OFF_CMD,
            Ring::Ep0 => OFF_EP0,
            Ring::Intr => OFF_INTR,
        }
    }

    fn trb_ptr(&self, ring: Ring, index: u8) -> *mut u8 {
        self.dma
            .wrapping_add(self.ring_off(ring) + usize::from(index) * 16)
    }

    fn doorbell(&self, slot: u8, target: u8) {
        fence(Ordering::SeqCst);
        self.write32(self.db + u32::from(slot) * 4, u32::from(target));
    }

    fn pci_read(&self, dev: u8, func: u8, reg: u8) -> u32 {
        self.out32(self.pci_port, pci_addr(dev, func, reg));
        self.in32(self.pci_port + 4)
    }

    fn pci_write(&self, dev: u8, func: u8, reg: u8, data: u32) {
        self.out32(self.pci_port, pci_addr(dev, func, reg));
        self.out32(self.pci_port + 4, data);
    }

    fn out32(&self, port: u16, value: u32) {
        with_ipc_buffer_mut(|ipc| {
            ipc.inner_mut()
                .seL4_X86_IOPort_Out32(self.pci_cap, u64::from(port), u64::from(value));
        });
    }

    fn in32(&self, port: u16) -> u32 {
        with_ipc_buffer_mut(|ipc| {
            let ret = ipc.inner_mut().seL4_X86_IOPort_In32(self.pci_cap, port);
            ret.result
        })
    }

    fn read32(&self, off: u32) -> u32 {
        unsafe {
            self.regs
                .wrapping_add(off as usize)
                .cast::<u32>()
                .read_volatile()
        }
    }

    fn write32(&self, off: u32, value: u32) {
        unsafe {
            self.regs
                .wrapping_add(off as usize)
                .cast::<u32>()
                .write_volatile(value);
        }
    }

    fn write64(&self, off: u32, value: u64) {
        self.write32(off, value as u32);
        self.write32(off + 4, (value >> 32) as u32);
    }

    fn zero_dma(&self) {
        unsafe {
            core::ptr::write_bytes(self.dma, 0, DMA_LEN);
        }
        fence(Ordering::SeqCst);
    }

    fn zero_range(&self, off: usize, len: usize) {
        unsafe {
            core::ptr::write_bytes(self.dma.wrapping_add(off), 0, len);
        }
        fence(Ordering::SeqCst);
    }

    fn read_dma(&self, off: usize, dst: &mut [u8]) {
        for (i, byte) in dst.iter_mut().enumerate() {
            unsafe {
                *byte = self.dma.wrapping_add(off + i).read_volatile();
            }
        }
    }

    fn read_dma_u32(&self, off: usize) -> u32 {
        unsafe { self.dma.wrapping_add(off).cast::<u32>().read_volatile() }
    }

    fn write_dma_u32(&self, off: usize, value: u32) {
        unsafe {
            self.dma
                .wrapping_add(off)
                .cast::<u32>()
                .write_volatile(value);
        }
    }

    fn write_dma_u64(&self, off: usize, value: u64) {
        self.write_dma_u32(off, value as u32);
        self.write_dma_u32(off + 4, (value >> 32) as u32);
    }
}

fn trb_type(kind: u32) -> u32 {
    kind << 10
}

fn trb_kind(ev: [u32; 4]) -> u32 {
    (ev[3] >> 10) & 0x3f
}

fn completion(ev: [u32; 4]) -> u32 {
    ev[2] >> 24
}

fn endpoint_id(ev: [u32; 4]) -> u32 {
    (ev[3] >> 16) & 0x1f
}

fn event_ptr(ev: [u32; 4]) -> u64 {
    u64::from(ev[0]) | (u64::from(ev[1]) << 32)
}

/// PORTSC value that does not clear change bits or write PED.
fn portsc_neutral(raw: u32) -> u32 {
    const RO: u32 = (1 << 0) | (1 << 3) | (0xf << 10) | (1 << 30);
    const RWS: u32 = (0xf << 5) | (1 << 9) | (0x3 << 14) | (0x7 << 25);
    (raw & RO) | (raw & RWS)
}

fn pci_addr(dev: u8, func: u8, reg: u8) -> u32 {
    0x8000_0000 | (u32::from(dev) << 11) | (u32::from(func) << 8) | (u32::from(reg) & 0xfc)
}

fn write_trb(ptr: *mut u8, d0: u32, d1: u32, d2: u32, d3: u32) {
    unsafe {
        let words = ptr.cast::<u32>();
        words.write_volatile(d0);
        words.add(1).write_volatile(d1);
        words.add(2).write_volatile(d2);
        // The cycle bit lives in d3. The controller may read the TRB as soon as it sees it.
        fence(Ordering::SeqCst);
        words.add(3).write_volatile(d3);
    }
}

fn read_trb(ptr: *mut u8) -> [u32; 4] {
    unsafe {
        let words = ptr.cast::<u32>();
        [
            words.read_volatile(),
            words.add(1).read_volatile(),
            words.add(2).read_volatile(),
            words.add(3).read_volatile(),
        ]
    }
}
