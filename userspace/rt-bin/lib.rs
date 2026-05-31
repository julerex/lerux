//! Shared helpers for lerux `no_std` initfs binaries.

#![no_std]

extern crate alloc;

use core::fmt::Write;

pub fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    struct W;
    impl Write for W {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let _ = syscall::write(1, s.as_bytes());
            Ok(())
        }
    }
    let _ = writeln!(&mut W, "{info}");
    unsafe { core::arch::asm!("ud2", options(noreturn)) };
}
