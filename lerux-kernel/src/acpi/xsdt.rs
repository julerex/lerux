//! The XSDT: the 64-bit root table listing all other ACPI tables.
//!
//! The **XSDT** (Extended System Description Table) is the modern, 64-bit-pointer
//! version of the [`rsdt`](super). Firmware provides one or the other;
//! [`rxsdt`](super) lets the kernel treat both uniformly.
//!
//! See also: [`docs/kernel/architecture.md`] section 8.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

use crate::rmm::PhysicalAddress;
use alloc::boxed::Box;
use core::convert::TryFrom;

use super::{rxsdt::Rxsdt, sdt::Sdt};

#[derive(Debug)]
pub struct Xsdt(&'static Sdt);

impl Xsdt {
    pub fn new(sdt: &'static Sdt) -> Option<Xsdt> {
        if &sdt.signature == b"XSDT" {
            Some(Xsdt(sdt))
        } else {
            None
        }
    }
    pub fn as_slice(&self) -> &[u8] {
        let length =
            usize::try_from(self.0.length).expect("expected 32-bit length to fit within usize");

        unsafe { core::slice::from_raw_parts(self.0 as *const _ as *const u8, length) }
    }
}

impl Rxsdt for Xsdt {
    fn iter(&self) -> Box<dyn Iterator<Item = PhysicalAddress>> {
        Box::new(XsdtIter { sdt: self.0, i: 0 })
    }
}

pub struct XsdtIter {
    sdt: &'static Sdt,
    i: usize,
}

impl Iterator for XsdtIter {
    type Item = PhysicalAddress;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.sdt.data_len() / size_of::<u64>() {
            let item = unsafe {
                core::ptr::read_unaligned((self.sdt.data_address() as *const u64).add(self.i))
            };
            self.i += 1;
            Some(PhysicalAddress::new(item as usize))
        } else {
            None
        }
    }
}
