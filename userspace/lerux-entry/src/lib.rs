#![no_std]

#[cfg(target_arch = "x86_64")]
#[path = "arch/x86_64.rs"]
pub mod arch;
pub mod stack;
mod start;

pub use stack::{auxv_lookup, auxv_lookup_at_sp, env_var, Stack};
pub use start::stack;

#[macro_export]
macro_rules! rt_main {
    ($($body:tt)*) => {
        #![no_std]
        #![no_main]
        #![feature(never_type)]

        extern crate alloc;

        #[panic_handler]
        fn __lerux_panic(info: &core::panic::PanicInfo) -> ! {
            use core::fmt::Write;
            struct W;
            impl Write for W {
                fn write_str(&mut self, s: &str) -> core::fmt::Result {
                    let _ = syscall::write(1, s.as_bytes());
                    Ok(())
                }
            }
            let _ = writeln!(&mut W, "{info}");
            core::arch::asm!("ud2", options(noreturn));
        }

        #[unsafe(no_mangle)]
        fn lerux_rt_main() -> ! {
            $($body)*
        }
    };
}
