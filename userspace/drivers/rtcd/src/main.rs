#![no_std]
#![no_main]
#![feature(never_type)]

extern crate alloc;

#[allow(unused_imports)]
use lerux_entry as _;
#[allow(unused_imports)]
use lerux_shim as _;

mod pio;
mod x86;

use libredox::Fd;
use libredox::flag::O_WRONLY;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    rt_bin::panic_handler(info)
}

fn acquire_iopl() {
    use libredox::call::call_wo;
    let thr = unsafe { lerux_shim::redox_cur_thrfd_v0() };
    let kernel_fd = syscall::dup(thr, b"open_via_dup").expect("rtcd: open_via_dup");
    call_wo(
        kernel_fd,
        &[],
        syscall::CallFlags::empty(),
        &[syscall::ProcSchemeVerb::Iopl as u64],
    )
    .expect("rtcd: iopl");
    syscall::close(kernel_fd).ok();
}

#[unsafe(no_mangle)]
fn lerux_rt_main() -> ! {
    acquire_iopl();
    let time_s = x86::get_time();
    let time_ns = (time_s as u128) * 1_000_000_000;
    let fd = Fd::open("/scheme/sys/update_time_offset", O_WRONLY, 0)
        .expect("rtcd: failed to open sys time offset");
    fd.write(&time_ns.to_ne_bytes())
        .expect("rtcd: failed to write time offset");
    libredox::call::setrens(0, 0).expect("rtcd: failed to enter null namespace");
    loop {
        core::hint::spin_loop();
    }
}
