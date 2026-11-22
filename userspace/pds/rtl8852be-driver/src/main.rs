//! Map the RTL8852BE BAR 2, stop the PCI HCI, and publish one status page.
//!
//! This domain has no channel and writes no serial text. `console_driver`
//! owns `0xcf8` and fills the config page before this init runs.

#![no_std]
#![no_main]

use sel4_microkit::{memory_region_symbol, protection_domain, ChannelSet, Handler, Infallible};

use lerux_rtw8852be::{
    card_state, idle, CardState, Programmed, Regs, Status, MAPPED_MMIO_PHYS, MMIO_LEN,
    STATUS_BAR_MISMATCH, STATUS_MISSING, STATUS_PROGRAMMED,
};

struct HandlerImpl;

struct VolatileRegs {
    base: *mut u32,
    len: usize,
}

impl Regs for VolatileRegs {
    fn read32(&self, offset: usize) -> u32 {
        let Some(ptr) = self.word(offset) else {
            return 0xffff_ffff;
        };
        // SAFETY: `word` returns a 4-byte-aligned address inside the mapped BAR.
        unsafe { ptr.read_volatile() }
    }

    fn write32(&mut self, offset: usize, value: u32) {
        let Some(ptr) = self.word(offset) else {
            return;
        };
        // SAFETY: `word` returns a 4-byte-aligned address inside the mapped BAR.
        unsafe { ptr.write_volatile(value) }
    }
}

impl VolatileRegs {
    fn word(&self, offset: usize) -> Option<*mut u32> {
        if !offset.is_multiple_of(4) || offset.saturating_add(4) > self.len {
            return None;
        }
        Some(self.base.wrapping_byte_add(offset))
    }
}

#[protection_domain]
fn init() -> HandlerImpl {
    let (bridge, endpoint) = read_headers();
    let state = card_state(&bridge, &endpoint, MAPPED_MMIO_PHYS);
    let mut regs = mmio_regs();
    write_status(status_of(state, &mut regs));
    HandlerImpl
}

fn mmio_regs() -> VolatileRegs {
    let base = memory_region_symbol!(rtl8852be_mmio_vaddr: *mut u32).as_ptr();
    VolatileRegs {
        base,
        len: usize::try_from(MMIO_LEN).expect("mmio len fits"),
    }
}

fn status_of(state: CardState, regs: &mut VolatileRegs) -> Status {
    match state {
        CardState::Missing => blank(STATUS_MISSING, 0),
        CardState::BarMismatch(mismatch) => blank(STATUS_BAR_MISMATCH, mismatch.device.0),
        CardState::Programmed(programmed) => programmed_status(&programmed, regs),
    }
}

fn programmed_status(programmed: &Programmed, regs: &mut VolatileRegs) -> Status {
    match idle(regs) {
        Ok(chip_ver) => Status {
            kind: STATUS_PROGRAMMED,
            chip_ver,
            device: programmed.card.device.0,
            mmio_phys: MAPPED_MMIO_PHYS,
        },
        Err(()) => blank(STATUS_MISSING, programmed.card.device.0),
    }
}

fn blank(kind: u8, device: u16) -> Status {
    Status {
        kind,
        chip_ver: 0,
        device,
        mmio_phys: 0,
    }
}

fn read_headers() -> ([u8; 64], [u8; 64]) {
    let page = memory_region_symbol!(rtl8852be_cfg_vaddr: *mut u8).as_ptr();
    let mut bridge = [0u8; 64];
    let mut endpoint = [0u8; 64];
    // SAFETY: the template maps a 4 KiB config page at this symbol. The two
    // headers occupy the first 128 bytes.
    unsafe {
        copy_volatile(page, &mut bridge);
        copy_volatile(page.add(64), &mut endpoint);
    }
    (bridge, endpoint)
}

fn write_status(status: Status) {
    let ptr = memory_region_symbol!(rtl8852be_status_vaddr: *mut Status).as_ptr();
    // SAFETY: the template maps a 4 KiB status page at this symbol. Status is
    // 16 bytes at offset 0, and the page address is 4 KiB aligned.
    unsafe { ptr.write_volatile(status) }
}

/// # Safety
///
/// `src` must be readable for `dst.len()` bytes.
unsafe fn copy_volatile(src: *const u8, dst: &mut [u8]) {
    for (i, byte) in dst.iter_mut().enumerate() {
        *byte = unsafe { src.add(i).read_volatile() };
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, _channels: ChannelSet) -> Result<(), Self::Error> {
        Ok(())
    }
}
