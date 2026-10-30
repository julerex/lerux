//! AMD xHCI bring-up for one Bluetooth HCI reset.
//!
//! The ring layout matches the console driver's host. This domain has no PCI
//! configuration port: firmware left the BARs, and the probe already enabled
//! memory space and bus master. Events are polled. The keyboard on this image
//! is the i8042, so resetting these controllers does not take the console.

use core::sync::atomic::{fence, Ordering};

use lerux_swift::{
    acknowledge_reset, find_hci_interrupt, frame_usb_event, usb_hci_reset, HciIntr, XferFail,
};

const DMA_PHYS: u64 = 0xf800_0000;
const DMA_LEN: usize = 0x4_0000;
const MMIO_LEN: u32 = 0x1_0000;

const OFF_DCBAA: usize = 0x0000;
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

enum Intr {
    Wait,
    Bytes(usize),
    Failed,
}

struct Host {
    regs: *mut u8,
    dma: *mut u8,
    op: u32,
    db: u32,
    iman: u32,
    erdp: u32,
    ctx_bytes: usize,
    max_ports: u8,
    max_slots: u8,
    hcc: u32,
    hcs2: u32,
    slot: u8,
    dci: u8,
    speed: u8,
    report_len: usize,
    intr_trb: u64,
    intr: Intr,
    cmd_i: u8,
    cmd_c: u8,
    ep0_i: u8,
    ep0_c: u8,
    intr_i: u8,
    intr_c: u8,
    ev_i: u8,
    ev_c: u8,
}

pub fn poll_reset(regs: *mut u8, dma: *mut u8, event: &mut [u8]) -> Result<usize, XferFail> {
    let mut host = Host::new(regs, dma);
    let result = host.attempt(event);
    host.halt();
    result
}

impl Host {
    fn new(regs: *mut u8, dma: *mut u8) -> Self {
        Self {
            regs,
            dma,
            op: 0,
            db: 0,
            iman: 0,
            erdp: 0,
            ctx_bytes: 32,
            max_ports: 0,
            max_slots: 0,
            hcc: 0,
            hcs2: 0,
            slot: 0,
            dci: 0,
            speed: 0,
            report_len: 0,
            intr_trb: 0,
            intr: Intr::Wait,
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

    fn attempt(&mut self, event: &mut [u8]) -> Result<usize, XferFail> {
        if !self.read_caps() {
            return Err(XferFail::Halt);
        }
        self.legacy_handoff();
        if !self.halt_now() || !self.reset_controller() || self.read32(self.op + 8) != 1 {
            return Err(XferFail::Halt);
        }
        if !self.program_rings() || !self.run() {
            return Err(XferFail::Halt);
        }
        let Some(radio) = self.find_radio() else {
            return Err(XferFail::Port);
        };
        self.arm_interrupt(&radio);
        self.send_reset(event)
    }

    fn find_radio(&mut self) -> Option<HciIntr> {
        self.power_ports();
        self.wait_connect();
        for port in 1..=self.max_ports {
            if self.read32(self.portsc(port)) & 1 == 0 {
                continue;
            }
            let Some(speed) = self.reset_port(port) else {
                continue;
            };
            if !self.enable_slot() {
                continue;
            }
            if self.address(port, speed)
                && let Some(radio) = self.bluetooth()
            {
                return Some(radio);
            }
            self.disable_slot();
        }
        None
    }

    fn bluetooth(&mut self) -> Option<HciIntr> {
        self.device_descriptor()?;
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
        let radio = find_hci_interrupt(&cfg[..take])?;
        if !matches!(
            self.control(0x00, 9, u16::from(radio.config_value), 0, 0),
            Ctl::Done(_)
        ) {
            return None;
        }
        Some(radio)
    }

    fn arm_interrupt(&mut self, radio: &HciIntr) {
        let dci = endpoint_dci(radio.ep_addr);
        if dci == 0 || radio.max_packet == 0 {
            return;
        }
        self.zero_range(OFF_IN, 33 * self.ctx_bytes);
        self.copy_dev_to_input(0, 1);
        let dw0 = self.read_ctx(true, 1, 0);
        self.write_ctx(true, 1, 0, (dw0 & !(0x1f << 27)) | (u32::from(dci) << 27));
        self.intr_i = 0;
        self.intr_c = 1;
        self.zero_range(OFF_INTR, 32 * 16);
        let interval = xhci_interval(self.speed, radio.interval);
        let packet = radio.max_packet.min(64);
        self.report_len = usize::from(packet);
        self.write_ep(usize::from(dci) + 1, 7, packet, interval, OFF_INTR, packet);
        self.write_ctx(true, 0, 1, (1 << 0) | (1 << dci));
        if !self.command_ok(TRB_CONFIGURE) {
            return;
        }
        self.dci = dci;
        self.intr_trb = self.queue_intr(packet);
    }

    fn send_reset(&mut self, event: &mut [u8]) -> Result<usize, XferFail> {
        if self.dci == 0 || self.intr_trb == 0 {
            return Err(XferFail::Event);
        }
        let payload = usb_hci_reset();
        self.write_dma(OFF_DATA, &payload);
        if !matches!(
            self.control(0x20, 0, 0, 0, payload.len() as u16),
            Ctl::Done(_)
        ) {
            return Err(XferFail::Event);
        }
        let Some(n) = self.wait_intr() else {
            return Err(XferFail::Event);
        };
        let mut usb = [0u8; 16];
        let take = n.min(usb.len());
        self.read_dma(OFF_REPORT, &mut usb[..take]);
        let framed = frame_usb_event(&usb[..take], event);
        if framed > 0 && acknowledge_reset(&event[..framed]) {
            Ok(framed)
        } else {
            Err(XferFail::Event)
        }
    }

    fn queue_intr(&mut self, packet: u16) -> u64 {
        self.ensure_room(Ring::Intr, 1);
        self.zero_range(OFF_REPORT, usize::from(packet));
        let buf = DMA_PHYS + OFF_REPORT as u64;
        let d3 = ISP | IOC | trb_type(TRB_NORMAL);
        let phys = self.push_trb(
            Ring::Intr,
            buf as u32,
            (buf >> 32) as u32,
            u32::from(packet),
            d3,
        );
        self.doorbell(self.slot, self.dci);
        phys
    }

    fn wait_intr(&mut self) -> Option<usize> {
        if let Some(n) = self.intr_bytes() {
            return n;
        }
        for _ in 0..SPIN {
            let Some(ev) = self.pop_event() else {
                core::hint::spin_loop();
                continue;
            };
            if trb_kind(ev) == TRB_PORT_STATUS {
                self.ack_port_event(ev);
                continue;
            }
            if trb_kind(ev) == TRB_TRANSFER && event_ptr(ev) & !0xf == self.intr_trb & !0xf {
                self.note_intr(ev);
                return self.intr_bytes().unwrap_or(None);
            }
        }
        None
    }

    fn note_intr(&mut self, ev: [u32; 4]) {
        let code = completion(ev);
        self.intr = if code == CC_SHORT {
            let residual = usize::try_from(ev[2] & 0x00ff_ffff).unwrap_or(0);
            Intr::Bytes(self.report_len.saturating_sub(residual))
        } else if code == CC_SUCCESS {
            Intr::Bytes(self.report_len)
        } else {
            Intr::Failed
        };
    }

    fn intr_bytes(&self) -> Option<Option<usize>> {
        match self.intr {
            Intr::Wait => None,
            Intr::Bytes(n) => Some(Some(n)),
            Intr::Failed => Some(None),
        }
    }

    fn read_caps(&mut self) -> bool {
        let caplen = self.read32(0) & 0xff;
        if !(0x20..=0x80).contains(&caplen) || !caplen.is_multiple_of(4) || caplen >= MMIO_LEN {
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
        if db == 0 || rt == 0 || rt + 0x40 >= MMIO_LEN || db + 4 >= MMIO_LEN {
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
            if off + 8 >= MMIO_LEN {
                return;
            }
            let dw = self.read32(off);
            let id = dw & 0xff;
            let next = (dw >> 8) & 0xff;
            if id == 1 {
                self.write32(off, dw | (1 << 24));
                for _ in 0..SPIN {
                    if self.read32(off) & (1 << 16) == 0 {
                        break;
                    }
                    core::hint::spin_loop();
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

    fn halt(&self) {
        self.halt_now();
    }

    fn halt_now(&self) -> bool {
        if self.op == 0 {
            return false;
        }
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
        let scratch = scratchpad_count(self.hcs2);
        if scratch > MAX_SCRATCH {
            return false;
        }
        self.zero_dma();
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

    fn run(&self) -> bool {
        self.write32(self.op, (1 << 0) | (1 << 2));
        self.wait_status(|status| status & 1 == 0)
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
            if trb_kind(ev) == TRB_PORT_STATUS {
                self.ack_port_event(ev);
                continue;
            }
            if trb_kind(ev) != TRB_TRANSFER {
                continue;
            }
            let ptr = event_ptr(ev) & !0xf;
            let code = completion(ev);
            if self.intr_trb != 0 && ptr == self.intr_trb & !0xf {
                self.note_intr(ev);
                continue;
            }
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
                    self.recover_endpoint(1);
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
            self.recover_endpoint(1);
            Ctl::Stall
        } else {
            Ctl::Fail
        }
    }

    fn recover_endpoint(&mut self, dci: u8) {
        let slot = u32::from(self.slot) << 24;
        let ep = u32::from(dci) << 16;
        let reset = trb_type(TRB_RESET_EP) | ep | slot;
        if self.submit_cmd(0, 0, 0, reset).is_none() {
            return;
        }
        let index = self.ring_index(Ring::Ep0);
        let cycle = u32::from(self.ring_cycle(Ring::Ep0));
        let ptr = self.ring_phys(Ring::Ep0) + u64::from(index) * 16;
        let d0 = (ptr as u32 & !0xf) | cycle;
        let d3 = trb_type(TRB_SET_DEQ) | ep | slot;
        let _ = self.submit_cmd(d0, (ptr >> 32) as u32, 0, d3);
    }

    fn submit_cmd(&mut self, d0: u32, d1: u32, d2: u32, d3: u32) -> Option<[u32; 4]> {
        self.ensure_room(Ring::Cmd, 1);
        let phys = self.push_trb(Ring::Cmd, d0, d1, d2, d3);
        self.doorbell(0, 0);
        self.wait_completion(phys)
    }

    fn wait_completion(&mut self, trb: u64) -> Option<[u32; 4]> {
        for _ in 0..SPIN {
            let Some(ev) = self.pop_event() else {
                core::hint::spin_loop();
                continue;
            };
            if trb_kind(ev) == TRB_PORT_STATUS {
                self.ack_port_event(ev);
                continue;
            }
            if trb_kind(ev) == TRB_COMMAND && event_ptr(ev) & !0xf == trb & !0xf {
                return Some(ev);
            }
        }
        None
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
        self.speed = speed;
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

    fn write_dma(&self, off: usize, src: &[u8]) {
        for (i, byte) in src.iter().enumerate() {
            unsafe {
                self.dma.wrapping_add(off + i).write_volatile(*byte);
            }
        }
        fence(Ordering::SeqCst);
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

fn endpoint_dci(ep_addr: u8) -> u8 {
    let number = ep_addr & 0x0f;
    if ep_addr & 0x80 != 0 {
        number * 2 + 1
    } else {
        number * 2
    }
}

fn xhci_interval(speed: u8, b_interval: u8) -> u8 {
    if speed <= 2 {
        let frames = u32::from(b_interval.max(1));
        let exponent = 32 - frames.saturating_mul(8).leading_zeros() - 1;
        return u8::try_from(exponent.clamp(3, 10)).unwrap_or(5);
    }
    b_interval.clamp(1, 16) - 1
}

fn scratchpad_count(hcsparams2: u32) -> u32 {
    ((hcsparams2 >> 16) & 0x3e0) | ((hcsparams2 >> 27) & 0x1f)
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

fn event_ptr(ev: [u32; 4]) -> u64 {
    u64::from(ev[0]) | (u64::from(ev[1]) << 32)
}

fn portsc_neutral(raw: u32) -> u32 {
    const RO: u32 = (1 << 0) | (1 << 3) | (0xf << 10) | (1 << 30);
    const RWS: u32 = (0xf << 5) | (1 << 9) | (0x3 << 14) | (0x7 << 25);
    (raw & RO) | (raw & RWS)
}

fn write_trb(ptr: *mut u8, d0: u32, d1: u32, d2: u32, d3: u32) {
    unsafe {
        let words = ptr.cast::<u32>();
        words.write_volatile(d0);
        words.add(1).write_volatile(d1);
        words.add(2).write_volatile(d2);
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
