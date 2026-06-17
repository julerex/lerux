//! riscv64 CPU identification reporting (the x86 `device/cpu.rs` analog).
use core::fmt::{Result, Write};

pub fn cpu_info<W: Write>(_w: &mut W) -> Result {
    unimplemented!()
}
