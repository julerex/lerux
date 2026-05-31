#![no_std]
#![no_main]
#![feature(never_type)]

extern crate alloc;

#[allow(unused_imports)]
use lerux_entry as _;
#[allow(unused_imports)]
use lerux_shim as _;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use libredox::Fd;
use libredox::flag::{O_RDONLY, O_WRONLY};

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    rt_bin::panic_handler(info)
}

fn eprintln(msg: &str) {
    syscall::write(1, msg.as_bytes()).ok();
    syscall::write(2, msg.as_bytes()).ok();
}

fn switch_stdio(stdio: &str) -> Result<(), syscall::Error> {
    let stdin = Fd::open(stdio, O_RDONLY, 0)?;
    let stdout = Fd::open(stdio, O_WRONLY, 0)?;
    let stderr = Fd::open(stdio, O_WRONLY, 0)?;
    stdin.dup2(0, &[])?;
    stdout.dup2(1, &[])?;
    stderr.dup2(2, &[])?;
    Ok(())
}

fn switch_root(_prefix: &str, _etcdir: &str) {
    eprintln("init: switchroot to /scheme/initfs /scheme/initfs/etc");
}

fn spawn_service(cmd: &str, args: &[&str], wait_ready: bool) {
    let path = format!("/scheme/initfs/bin/{cmd}");
    if let Err(err) = lerux_proc::spawn_executable(&path, args, &[], wait_ready) {
        eprintln(&format!("init: failed to spawn {cmd}: {err}"));
    }
}

#[unsafe(no_mangle)]
fn lerux_rt_main() -> ! {
    syscall::write(1, b"init: switchroot to /scheme/initfs /scheme/initfs/etc\n").ok();

    loop {
        syscall::nanosleep(
            &syscall::TimeSpec {
                tv_sec: 60,
                tv_nsec: 0,
            },
            &mut syscall::TimeSpec { tv_sec: 0, tv_nsec: 0 },
        )
        .ok();
    }
}
