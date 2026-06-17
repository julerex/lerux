//! The RSDP: the entry point to all ACPI tables.
//!
//! The **RSDP** (Root System Description Pointer) is the very first ACPI
//! structure the kernel finds (the bootloader passes it, or it is searched for
//! in BIOS memory). It points at the root table (RSDT/XSDT), from which every
//! other ACPI table is reached. This module validates and parses it.
//!
//! See also: [`docs/kernel/architecture.md`] section 8.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

use core::ptr::NonNull;

use crate::rmm::PhysicalAddress;

/// RSDP
#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct Rsdp {
    signature: [u8; 8],
    _checksum: u8,
    _oemid: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    length: u32,
    xsdt_address: u64,
    _extended_checksum: u8,
    _reserved: [u8; 3],
}

impl Rsdp {
    pub unsafe fn get_rsdp(already_supplied_rsdp: Option<NonNull<u8>>) -> Option<Rsdp> {
        already_supplied_rsdp.and_then(|rsdp_ptr: NonNull<u8>| {
            let rsdp: Rsdp = unsafe { rsdp_ptr.cast().read() };

            if rsdp.signature != *b"RSD PTR " {
                error!("RSDP signature check failed");
                return None;
            }

            let mut sum: u8 = 0;
            for i in 0..20 {
                sum = sum.wrapping_add(unsafe { rsdp_ptr.add(i).read() });
            }
            if sum != 0 {
                error!("RSDP checksum failed");
                return None;
            }

            if rsdp.revision >= 2 {
                let mut sum: u8 = 0;
                for i in 0..rsdp.length as usize {
                    sum = sum.wrapping_add(unsafe { rsdp_ptr.add(i).read() });
                }
                if sum != 0 {
                    error!("XSDP checksum failed");
                    return None;
                }
            }

            Some(rsdp)
        })
    }

    /// Get the RSDT or XSDT address
    pub fn sdt_address(&self) -> PhysicalAddress {
        PhysicalAddress::new(if self.revision >= 2 {
            self.xsdt_address as usize
        } else {
            self.rsdt_address as usize
        })
    }
}
