//! riscv64 shutdown/reset, via SBI (the x86 `stop.rs` analog).
use crate::sync::CleanLockToken;

pub unsafe fn kreset() -> ! {
    println!("kreset");
    unimplemented!()
}

pub unsafe fn emergency_reset() -> ! {
    unimplemented!()
}

pub unsafe fn kstop(_token: &mut CleanLockToken) -> ! {
    println!("kstop");
    unimplemented!()
}
