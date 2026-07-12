//! MC146818 CMOS RTC via x86 I/O ports `0x70`/`0x71` (QEMU PC / Microkit).
//!
//! Served through the rust-sel4 RTC adapter for stock `RtcClient` use.

#![no_std]
#![no_main]

use rtcc::{DateTimeAccess, NaiveDate, NaiveDateTime};
use sel4::with_ipc_buffer_mut;
use sel4_microkit::{protection_domain, var, Channel, Handler};
use sel4_microkit_driver_adapters::rtc::driver::HandlerImpl;

const CLIENT: Channel = Channel::new(1);

/// Microkit assigns IOPort caps starting at this CNode slot (see serial ns16550).
const BASE_IOPORT_CAP: u64 = 394;

const REG_SECONDS: u8 = 0x00;
const REG_MINUTES: u8 = 0x02;
const REG_HOURS: u8 = 0x04;
const REG_DAY: u8 = 0x07;
const REG_MONTH: u8 = 0x08;
const REG_YEAR: u8 = 0x09;
const REG_STATUS_A: u8 = 0x0a;
const REG_STATUS_B: u8 = 0x0b;

const STATUS_A_UIP: u8 = 0x80;
const STATUS_B_BINARY: u8 = 0x04;

struct Driver {
    ioport_id: u32,
    index_port: u16,
}

unsafe impl Send for Driver {}
unsafe impl Sync for Driver {}

impl Driver {
    fn from_system_vars() -> Self {
        let ioport_id = *var!(cmos_ioport_id: usize = usize::MAX) as u32;
        let index_port = *var!(cmos_ioport_addr: usize = usize::MAX) as u16;
        Self {
            ioport_id,
            index_port,
        }
    }

    fn ioport_cap(&self) -> u64 {
        BASE_IOPORT_CAP + u64::from(self.ioport_id)
    }

    fn out8(&self, port: u16, value: u8) {
        with_ipc_buffer_mut(|ipc| {
            ipc.inner_mut()
                .seL4_X86_IOPort_Out8(self.ioport_cap(), port as u64, value as u64);
        });
    }

    fn in8(&self, port: u16) -> u8 {
        with_ipc_buffer_mut(|ipc| {
            let ret = ipc.inner_mut().seL4_X86_IOPort_In8(self.ioport_cap(), port);
            ret.result
        })
    }

    fn cmos_read(&self, reg: u8) -> u8 {
        // NMI disable bit (0x80) kept clear; Microkit smoke does not need NMI.
        self.out8(self.index_port, reg);
        self.in8(self.index_port + 1)
    }

    fn wait_update_done(&self) {
        // Spin while update-in-progress; CMOS updates are short.
        for _ in 0..100_000 {
            if self.cmos_read(REG_STATUS_A) & STATUS_A_UIP == 0 {
                break;
            }
        }
    }

    fn bcd_to_bin(v: u8) -> u8 {
        (v & 0x0f) + ((v >> 4) * 10)
    }
}

impl DateTimeAccess for Driver {
    type Error = Error;

    fn datetime(&mut self) -> Result<NaiveDateTime, Self::Error> {
        self.wait_update_done();
        let status_b = self.cmos_read(REG_STATUS_B);
        let binary = status_b & STATUS_B_BINARY != 0;

        let mut sec = self.cmos_read(REG_SECONDS);
        let mut min = self.cmos_read(REG_MINUTES);
        let mut hour = self.cmos_read(REG_HOURS);
        let mut day = self.cmos_read(REG_DAY);
        let mut month = self.cmos_read(REG_MONTH);
        let mut year = self.cmos_read(REG_YEAR);

        if !binary {
            sec = Self::bcd_to_bin(sec);
            min = Self::bcd_to_bin(min);
            hour = Self::bcd_to_bin(hour & 0x7f);
            day = Self::bcd_to_bin(day);
            month = Self::bcd_to_bin(month);
            year = Self::bcd_to_bin(year);
        } else {
            hour &= 0x7f;
        }

        // QEMU CMOS year is years since 2000 when < 100 (classic PC RTC).
        let full_year = 2000i32 + i32::from(year);

        NaiveDate::from_ymd_opt(full_year, u32::from(month), u32::from(day))
            .and_then(|d| d.and_hms_opt(u32::from(hour), u32::from(min), u32::from(sec)))
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
    let driver = Driver::from_system_vars();
    HandlerImpl::new(driver, CLIENT)
}
