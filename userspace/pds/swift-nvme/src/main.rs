//! Read-only NVMe at the recorded `02:00.0`. The Windows volume is not formatted.
//!
//! A read of [`NAMED_FILE`] is submitted on the sDDF block queue and, when the
//! controller answers, issued as NVMe IO read `0x02`. IO write and format are
//! not submitted.

#![no_std]
#![no_main]

use sel4_microkit::{
    memory_region_symbol, protection_domain, Channel, ChannelSet, Handler, Infallible,
};

use lerux_desk::FRAMEBUFFER_BYTES;
use lerux_sddf::{
    blk_dequeue_req, blk_enqueue_req, blk_queue_handle_t, blk_queue_init, blk_req_code_t,
};
use lerux_swift::{
    class_code, io_allowed, load_header, load_named, paint_row, report_bytes, vendor_device,
    write_result, NVME_BDF, NVME_CLASS,
};

const ROW: usize = 23;
const DMA_PHYS: u64 = 0xfb00_0000;
const ADMIN_SQ: usize = 0;
const ADMIN_CQ: usize = 0x1000;
const IO_SQ: usize = 0x2000;
const IO_CQ: usize = 0x3000;
const DATA: usize = 0x4000;
const BLK_REQ: usize = 0x5000;
const BLK_RESP: usize = 0x6000;
const SPIN: u32 = 200_000;
const SHELL: Channel = Channel::new(1);

struct HandlerImpl {
    irq: Channel,
}

#[protection_domain]
fn init() -> HandlerImpl {
    let headers = memory_region_symbol!(pci_headers_vaddr: *mut u8).as_ptr();
    let headers = unsafe { core::slice::from_raw_parts(headers, 0x1000) };
    let (bus, dev, func) = NVME_BDF;
    let header = load_header(headers, bus, dev, func);
    let present = vendor_device(&header).is_some() && class_code(&header) == Some(NVME_CLASS);
    let regs = memory_region_symbol!(nvme_regs_vaddr: *mut u8).as_ptr();
    let dma = memory_region_symbol!(nvme_dma_vaddr: *mut u8).as_ptr();
    let report = if present && queue_read(dma) {
        if let Some(mut ctrl) = Ctrl::enable(regs, dma) {
            load_named(|lba| ctrl.read_lba(lba))
        } else {
            lerux_swift::DiskReport::Absent
        }
    } else {
        lerux_swift::DiskReport::Absent
    };
    let payload = report_bytes(&report);
    let result = memory_region_symbol!(disk_result_vaddr: *mut u8).as_ptr();
    let page = unsafe { core::slice::from_raw_parts_mut(result, 0x1000) };
    write_result(page, payload);
    paint(payload);
    SHELL.notify();
    HandlerImpl {
        irq: Channel::new(0),
    }
}

fn queue_read(dma: *mut u8) -> bool {
    let mut handle = blk_queue_handle_t {
        req_queue: core::ptr::null_mut(),
        resp_queue: core::ptr::null_mut(),
        capacity: 0,
    };
    unsafe {
        blk_queue_init(
            &mut handle,
            dma.add(BLK_REQ).cast(),
            dma.add(BLK_RESP).cast(),
            2,
        );
        if blk_enqueue_req(&handle, blk_req_code_t::BLK_REQ_READ, DATA as u64, 0, 1, 1) != 0 {
            return false;
        }
        let mut code = blk_req_code_t::BLK_REQ_FLUSH;
        let mut offset = 0u64;
        let mut block = 0u64;
        let mut count = 0u16;
        let mut id = 0u32;
        if blk_dequeue_req(
            &handle,
            &mut code,
            &mut offset,
            &mut block,
            &mut count,
            &mut id,
        ) != 0
        {
            return false;
        }
        code == blk_req_code_t::BLK_REQ_READ && io_allowed(0x02)
    }
}

struct Sqe {
    opcode: u8,
    nsid: u32,
    prp: u64,
    cdw10: u32,
    cdw11: u32,
    cdw12: u32,
}

struct Ctrl {
    regs: *mut u8,
    dma: *mut u8,
    stride: u32,
    admin_tail: u16,
    admin_head: u16,
    admin_phase: u32,
    io_tail: u16,
    io_head: u16,
    io_phase: u32,
    cid: u16,
}

impl Ctrl {
    fn enable(regs: *mut u8, dma: *mut u8) -> Option<Self> {
        let vs = read32(regs, 0x08);
        if vs == 0 || vs == 0xffff_ffff {
            return None;
        }
        write32(regs, 0x14, 0);
        if !wait_rdy(regs, false) {
            return None;
        }
        let cap = read64(regs, 0);
        let dstrd = ((cap >> 32) & 0xf) as u32;
        let stride = 4u32 << dstrd;
        unsafe {
            dma.add(ADMIN_SQ).write_bytes(0, 0x1000);
            dma.add(ADMIN_CQ).write_bytes(0, 0x1000);
            dma.add(IO_SQ).write_bytes(0, 0x1000);
            dma.add(IO_CQ).write_bytes(0, 0x1000);
        }
        write32(regs, 0x24, 1 | (1 << 16));
        write64(regs, 0x28, DMA_PHYS + ADMIN_SQ as u64);
        write64(regs, 0x30, DMA_PHYS + ADMIN_CQ as u64);
        write32(regs, 0x14, 1 | (6 << 16) | (4 << 20));
        if !wait_rdy(regs, true) {
            return None;
        }
        let mut ctrl = Self {
            regs,
            dma,
            stride,
            admin_tail: 0,
            admin_head: 0,
            admin_phase: 1,
            io_tail: 0,
            io_head: 0,
            io_phase: 1,
            cid: 1,
        };
        if ctrl.admin(0x05, 0, DMA_PHYS + IO_CQ as u64, 1 | (1 << 16), 1) != Some(0) {
            return None;
        }
        if ctrl.admin(
            0x01,
            0,
            DMA_PHYS + IO_SQ as u64,
            1 | (1 << 16),
            1 | (1 << 16),
        ) != Some(0)
        {
            return None;
        }
        Some(ctrl)
    }

    fn dma_at(&self, off: usize) -> *mut u8 {
        unsafe { self.dma.add(off) }
    }

    fn read_lba(&mut self, lba: u64) -> Option<[u8; 512]> {
        unsafe { self.dma_at(DATA).write_bytes(0, 0x1000) };
        if self.io_read(lba) != Some(0) {
            return None;
        }
        let mut sector = [0u8; 512];
        unsafe {
            core::ptr::copy_nonoverlapping(self.dma_at(DATA), sector.as_mut_ptr(), 512);
        }
        Some(sector)
    }

    fn io_read(&mut self, lba: u64) -> Option<u16> {
        if !io_allowed(0x02) {
            return None;
        }
        self.submit(
            self.dma_at(IO_SQ),
            self.io_tail,
            Sqe {
                opcode: 0x02,
                nsid: 1,
                prp: DMA_PHYS + DATA as u64,
                cdw10: lba as u32,
                cdw11: (lba >> 32) as u32,
                cdw12: 0,
            },
        );
        self.io_tail = self.io_tail.wrapping_add(1) & 1;
        self.ring(2);
        let status = poll(self.dma_at(IO_CQ), self.io_head, self.io_phase)?;
        self.io_head = self.io_head.wrapping_add(1);
        if self.io_head.is_multiple_of(2) {
            self.io_phase ^= 1;
            self.io_head = 0;
        }
        self.ring(3);
        if status == 0 {
            Some(0)
        } else {
            None
        }
    }

    fn admin(&mut self, opcode: u8, nsid: u32, prp: u64, cdw10: u32, cdw11: u32) -> Option<u16> {
        self.submit(
            self.dma_at(ADMIN_SQ),
            self.admin_tail,
            Sqe {
                opcode,
                nsid,
                prp,
                cdw10,
                cdw11,
                cdw12: 0,
            },
        );
        self.admin_tail = self.admin_tail.wrapping_add(1) & 1;
        self.ring(0);
        let status = poll(self.dma_at(ADMIN_CQ), self.admin_head, self.admin_phase)?;
        self.admin_head = self.admin_head.wrapping_add(1);
        if self.admin_head.is_multiple_of(2) {
            self.admin_phase ^= 1;
            self.admin_head = 0;
        }
        self.ring(1);
        if status == 0 {
            Some(0)
        } else {
            None
        }
    }

    fn submit(&mut self, sq: *mut u8, slot: u16, cmd: Sqe) {
        let cid = self.cid;
        self.cid = self.cid.wrapping_add(1);
        unsafe {
            let entry = sq.add(usize::from(slot) * 64);
            entry.write_bytes(0, 64);
            entry.write(cmd.opcode);
            entry.add(2).cast::<u16>().write(cid);
            entry.add(4).cast::<u32>().write(cmd.nsid);
            entry.add(24).cast::<u64>().write(cmd.prp);
            entry.add(40).cast::<u32>().write(cmd.cdw10);
            entry.add(44).cast::<u32>().write(cmd.cdw11);
            entry.add(48).cast::<u32>().write(cmd.cdw12);
        }
    }

    fn ring(&self, qid: u32) {
        let tail = if qid == 0 {
            self.admin_tail
        } else if qid == 2 {
            self.io_tail
        } else {
            0
        };
        let head = if qid == 1 {
            self.admin_head
        } else if qid == 3 {
            self.io_head
        } else {
            tail
        };
        let value = if qid.is_multiple_of(2) {
            u32::from(tail)
        } else {
            u32::from(head)
        };
        let off = 0x1000 + (qid * self.stride) as usize;
        write32(self.regs, off, value);
    }
}

fn poll(cq: *mut u8, head: u16, phase: u32) -> Option<u16> {
    let slot = usize::from(head & 1) * 16;
    for _ in 0..SPIN {
        let dw3 = unsafe { cq.add(slot + 12).cast::<u32>().read_volatile() };
        if (dw3 >> 16) & 1 == phase {
            return Some(((dw3 >> 17) & 0x7fff) as u16);
        }
        core::hint::spin_loop();
    }
    None
}

fn wait_rdy(regs: *mut u8, ready: bool) -> bool {
    for _ in 0..SPIN {
        let csts = read32(regs, 0x1c);
        if csts == 0xffff_ffff {
            return false;
        }
        if (csts & 1 == 1) == ready {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

fn paint(text: &[u8]) {
    let mode = memory_region_symbol!(mode_record_vaddr: *mut u8).as_ptr();
    let frame = memory_region_symbol!(fb_vaddr: *mut u8).as_ptr();
    let record = unsafe { core::slice::from_raw_parts(mode, 40) };
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, FRAMEBUFFER_BYTES as usize) };
    paint_row(record, frame, ROW, text);
}

fn read32(base: *mut u8, off: usize) -> u32 {
    unsafe { base.add(off).cast::<u32>().read_volatile() }
}

fn read64(base: *mut u8, off: usize) -> u64 {
    unsafe { base.add(off).cast::<u64>().read_volatile() }
}

fn write32(base: *mut u8, off: usize, value: u32) {
    unsafe { base.add(off).cast::<u32>().write_volatile(value) }
}

fn write64(base: *mut u8, off: usize, value: u64) {
    unsafe { base.add(off).cast::<u64>().write_volatile(value) }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(self.irq) {
            self.irq.irq_ack().expect("ack nvme");
        }
        Ok(())
    }
}
