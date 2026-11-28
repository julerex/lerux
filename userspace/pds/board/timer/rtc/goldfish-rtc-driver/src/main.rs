//! Goldfish RTC driver (QEMU `virt` RISC-V at `0x101000`).
//!
//! Wall-clock nanoseconds via TIME_LOW/TIME_HIGH; served through the rust-sel4
//! RTC Microkit adapter so `supervisor` can use the stock `RtcClient`.

#![no_std]
#![no_main]

use core::ptr::read_volatile;

use rtcc::{DateTime, DateTimeAccess, NaiveDateTime};
use sel4_microkit::{memory_region_symbol, protection_domain, Channel, Handler};
use sel4_microkit_driver_adapters::rtc::driver::HandlerImpl;

const CLIENT: Channel = Channel::new(1);

const REG_TIME_LOW: usize = 0x00;
const REG_TIME_HIGH: usize = 0x04;

/// Google Goldfish RTC MMIO (see QEMU `hw/rtc/goldfish_rtc.c`).
struct Driver {
    base: *mut u8,
}

unsafe impl Send for Driver {}
unsafe impl Sync for Driver {}

impl Driver {
    unsafe fn new(base: *mut u8) -> Self {
        Self { base }
    }

    /// Read the 64-bit nanosecond counter (TIME_LOW latches TIME_HIGH).
    fn time_ns(&self) -> u64 {
        // SAFETY: MMIO region is mapped RW uncached for this PD only.
        let low = unsafe { read_volatile(self.base.add(REG_TIME_LOW) as *const u32) };
        let high = unsafe { read_volatile(self.base.add(REG_TIME_HIGH) as *const u32) };
        (u64::from(high) << 32) | u64::from(low)
    }
}

impl DateTimeAccess for Driver {
    type Error = Error;

    fn datetime(&mut self) -> Result<NaiveDateTime, Self::Error> {
        let ns = self.time_ns();
        let secs = (ns / 1_000_000_000) as i64;
        DateTime::from_timestamp(secs, 0)
            .map(|dt| dt.naive_utc())
            .ok_or(Error::InvalidTimestamp)
    }

    fn set_datetime(&mut self, _datetime: &NaiveDateTime) -> Result<(), Self::Error> {
        Err(Error::UnsupportedOperation)
    }
}

#[derive(Debug, Clone, Copy)]
enum Error {
    UnsupportedOperation,
    InvalidTimestamp,
}

#[protection_domain]
fn init() -> impl Handler {
    let base = memory_region_symbol!(goldfish_rtc_mmio_vaddr: *mut u8).as_ptr();
    let driver = unsafe { Driver::new(base) };
    HandlerImpl::new(driver, CLIENT)
}
