//! QEMU fw_cfg MMIO + DMA on virt (needed to configure ramfb).
//!
//! DMA descriptors and payloads must live in a RAM MR with a known guest
//! physical address — a stack `FWCfgDmaAccess` is a virtual address and
//! QEMU will DMA the wrong page.

use core::sync::atomic::{compiler_fence, Ordering};

const SELECTOR: usize = 8;
const DMA_ADDR: usize = 16;

const CTL_ERROR: u32 = 1 << 0;
const CTL_READ: u32 = 1 << 1;
const CTL_SELECT: u32 = 1 << 3;
const CTL_WRITE: u32 = 1 << 4;

const FW_CFG_FILE_DIR: u16 = 0x19;
const FW_CFG_SIGNATURE: u16 = 0;

const DMA_ACCESS_BYTES: usize = 16;
const DMA_DATA_OFF: usize = 64;
const DMA_PAGE: usize = 0x1000;

const FILE_ENTRY: usize = 64;
const FILE_NAME_OFF: usize = 8;
const FILE_NAME_LEN: usize = 56;

const RAMFB_CFG_BYTES: usize = 28;
const _: () = assert!(DMA_ACCESS_BYTES == 16);

/// Mode written into QEMU's `etc/ramfb` config.
pub struct RamfbCfg {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub fourcc: u32,
}

struct FwCfg {
    mmio: *mut u8,
    dma: *mut u8,
    dma_paddr: u64,
}

/// Configure ramfb via `etc/ramfb`. `mmio` is the fw_cfg register block;
/// `dma` / `dma_paddr` is the dedicated DMA page; `fb_paddr` is the device
/// framebuffer (not the app bitmap).
pub fn configure_ramfb(
    mmio: *mut u8,
    dma: *mut u8,
    dma_paddr: u64,
    fb_paddr: u64,
    mode: RamfbCfg,
) -> Result<(), &'static str> {
    if dma_paddr == 0 || fb_paddr == 0 {
        return Err("fw-cfg/fb paddr is zero");
    }
    let fw = FwCfg {
        mmio,
        dma,
        dma_paddr,
    };
    if !fw.signature_ok() {
        return Err("fw-cfg signature");
    }
    let select = fw.find_file(b"etc/ramfb\0")?;
    fw.write_ramfb(select, fb_paddr, &mode)
}

impl FwCfg {
    fn signature_ok(&self) -> bool {
        // Selector is 16-bit BE on MMIO fw_cfg. Data is string-preserving.
        write_be16(self.mmio, SELECTOR, FW_CFG_SIGNATURE);
        // SAFETY: `mmio` is the mapped fw_cfg page; offset 0 is the data register.
        let word = unsafe { self.mmio.cast::<u32>().read_volatile() };
        word.to_le_bytes() == *b"QEMU"
    }

    fn find_file(&self, name: &[u8]) -> Result<u16, &'static str> {
        let max = DMA_PAGE - DMA_DATA_OFF;
        self.dma_op(dir_control(FW_CFG_FILE_DIR, true), max as u32)?;
        // SAFETY: DMA completed into the data half of the dedicated page.
        let data = unsafe { self.dma.add(DMA_DATA_OFF) };
        let count = u32::from_be(unsafe { data.cast::<u32>().read_volatile() }) as usize;
        let cap = (max - 4) / FILE_ENTRY;
        if count == 0 || count > cap {
            return Err("fw-cfg file dir");
        }
        for i in 0..count {
            let entry = unsafe { data.add(4 + i * FILE_ENTRY) };
            if name_eq(entry, name) {
                let select = u16::from_be(unsafe { entry.add(4).cast::<u16>().read_volatile() });
                return Ok(select);
            }
        }
        Err("etc/ramfb missing (is -device ramfb on the QEMU command?)")
    }

    fn write_ramfb(&self, select: u16, fb_paddr: u64, mode: &RamfbCfg) -> Result<(), &'static str> {
        // SAFETY: data half of the DMA page is exclusive to this PD.
        let cfg = unsafe { self.dma.add(DMA_DATA_OFF) };
        write_be64_at(cfg, 0, fb_paddr);
        write_be32(cfg, 8, mode.fourcc);
        write_be32(cfg, 12, 0);
        write_be32(cfg, 16, mode.width);
        write_be32(cfg, 20, mode.height);
        write_be32(cfg, 24, mode.stride);
        self.dma_op(dir_control(select, false), RAMFB_CFG_BYTES as u32)
    }

    fn dma_op(&self, control: u32, length: u32) -> Result<(), &'static str> {
        let data_paddr = self.dma_paddr + DMA_DATA_OFF as u64;
        write_be32(self.dma, 0, control);
        write_be32(self.dma, 4, length);
        write_be64_at(self.dma, 8, data_paddr);
        compiler_fence(Ordering::SeqCst);
        trigger(self.mmio, self.dma_paddr);
        for _ in 0..1_000_000 {
            let c = u32::from_be(unsafe { self.dma.cast::<u32>().read_volatile() });
            if c & CTL_ERROR != 0 {
                return Err("fw-cfg DMA error");
            }
            if c == 0 {
                return Ok(());
            }
            compiler_fence(Ordering::SeqCst);
        }
        Err("fw-cfg DMA timeout")
    }
}

fn name_eq(entry: *mut u8, want: &[u8]) -> bool {
    for (i, &b) in want.iter().enumerate().take(FILE_NAME_LEN) {
        // SAFETY: `entry` is a 64-byte fw_cfg file record in the DMA page.
        if unsafe { entry.add(FILE_NAME_OFF + i).read_volatile() } != b {
            return false;
        }
    }
    true
}

fn dir_control(select: u16, read: bool) -> u32 {
    let op = if read { CTL_READ } else { CTL_WRITE };
    (u32::from(select) << 16) | CTL_SELECT | op
}

fn trigger(mmio: *mut u8, access_paddr: u64) {
    // Two 32-bit BE writes; the low half (offset 4 of the DMA register) starts
    // the transfer (QEMU fw_cfg.rst).
    write_be32(mmio, DMA_ADDR, (access_paddr >> 32) as u32);
    compiler_fence(Ordering::SeqCst);
    write_be32(mmio, DMA_ADDR + 4, access_paddr as u32);
}

fn write_be16(base: *mut u8, off: usize, val: u16) {
    // SAFETY: `base+off` is inside the fw_cfg MMIO window.
    unsafe {
        base.add(off).cast::<u16>().write_volatile(val.to_be());
    }
}

fn write_be32(base: *mut u8, off: usize, val: u32) {
    // SAFETY: `base+off` is inside the fw_cfg MMIO window or the DMA page.
    unsafe {
        base.add(off).cast::<u32>().write_volatile(val.to_be());
    }
}

fn write_be64_at(base: *mut u8, off: usize, val: u64) {
    // SAFETY: `base+off` is 8-byte aligned inside the DMA page.
    unsafe {
        base.add(off).cast::<u64>().write_volatile(val.to_be());
    }
}
