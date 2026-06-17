//! A unified view over the RSDT and XSDT root tables.
//!
//! Firmware provides either an [`rsdt`](super) (32-bit pointers) or an
//! [`xsdt`](super) (64-bit pointers). This module wraps whichever one exists so
//! the rest of the ACPI code can iterate the listed tables without caring which
//! root format is in use.
//!
//! See also: [`docs/kernel/architecture.md`] section 8.
//!
//! [`docs/kernel/architecture.md`]: ../../../../docs/kernel/architecture.md

use crate::rmm::PhysicalAddress;
use alloc::boxed::Box;

pub trait Rxsdt {
    fn iter(&self) -> Box<dyn Iterator<Item = PhysicalAddress>>;
}
