//! Smoke program for `program-runtime` (ADR-010).
//!
//! `black_box` keeps the `1 + 1 == 2` test in the Wasm body. `opt-level=z`
//! would otherwise delete the branch, and the interpreter would never see
//! `i32.add` / `i32.ne` / `br_if`.
//!
//! Pack with `lerux prog pack` (`-C link-arg=-zstack-size=4096`), which keeps
//! the module's linear memory at one 64 KiB page.

#![no_std]
#![no_main]

use core::hint::black_box;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[link(wasm_import_module = "lerux")]
unsafe extern "C" {
    safe fn log(ptr: *const u8, len: i32);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let sum = black_box(1_i32) + black_box(1_i32);
    if sum == 2 {
        let msg: &[u8] = b"lerux-prog: ran";
        log(msg.as_ptr(), msg.len() as i32);
    }
}
