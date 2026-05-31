//! Process spawn helpers for lerux init (no relibc `std::process`).

#![no_std]
#![allow(unsafe_op_in_unsafe_fn)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use redox_rt::proc::{ExtraInfo, FdGuard, FexecResult, fexec_impl, fork_impl, ForkArgs};
use redox_rt::RtTcb;
use syscall::{Error, Result, EINTR};

/// Fork and exec a static ELF, optionally passing `INIT_NOTIFY` on the write end of a pipe.
pub fn spawn_executable(
    path: &str,
    argv: &[&str],
    extra_env: &[(&str, &str)],
    wait_ready: bool,
) -> Result<()> {
    let (read_fd, write_fd) = pipe()?;

    let mut envs: Vec<String> = extra_env
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    if wait_ready {
        envs.push(format!("INIT_NOTIFY={write_fd}"));
    }

    let env_refs: Vec<&[u8]> = envs.iter().map(|s| s.as_bytes()).collect();
    let arg_refs: Vec<&[u8]> = argv.iter().map(|s| s.as_bytes()).collect();

    let pid = fork_impl(&ForkArgs::Managed)?;
    if pid == 0 {
        if !wait_ready {
            let _ = syscall::close(write_fd);
        }
        let _ = syscall::close(read_fd);
        exec_current(path, &arg_refs, &env_refs);
    }

    let _ = syscall::close(write_fd);
    if wait_ready {
        let mut buf = [0u8];
        loop {
            match syscall::read(read_fd, &mut buf) {
                Ok(0) => return Err(Error::new(syscall::EIO)),
                Ok(_) => break,
                Err(Error { errno }) if errno == EINTR => {}
                Err(err) => return Err(err),
            }
        }
    }
    let _ = syscall::close(read_fd);
    Ok(())
}

fn pipe() -> Result<(usize, usize)> {
    let read = syscall::openat(
        syscall::UPPER_FDTBL_TAG + syscall::data::GlobalSchemes::Pipe as usize,
        "",
        syscall::O_CLOEXEC,
        0,
    )?;
    let write = syscall::dup(read, b"write")?;
    Ok((read, write))
}

fn exec_current(path: &str, args: &[&[u8]], envs: &[&[u8]]) -> ! {
    let proc_fd = redox_rt::current_proc_fd();
    let thr_fd = RtTcb::current().thread_fd();
    let ns_fd = redox_rt::current_namespace_fd().ok();
    let cwd_fd = None;

    let extrainfo = ExtraInfo {
        cwd: Some(
            path.rsplit_once('/')
                .map_or("/", |(dir, _)| dir)
                .as_bytes(),
        ),
        sigprocmask: 0,
        sigignmask: 0,
        umask: redox_rt::sys::get_umask(),
        thr_fd: thr_fd.as_raw_fd(),
        proc_fd: proc_fd.as_raw_fd(),
        ns_fd,
        cwd_fd,
    };

    let ns = ns_fd.or_else(|| redox_rt::sys::getns().ok()).expect("lerux-proc: no namespace fd");
    let image = FdGuard::new(
        syscall::openat(ns, path, syscall::O_RDONLY | syscall::O_CLOEXEC, 0)
            .expect("lerux-proc: open executable"),
    )
    .to_upper()
    .expect("lerux-proc: to_upper image");

    match fexec_impl(
        image,
        thr_fd,
        proc_fd,
        path.as_bytes(),
        args,
        envs,
        &extrainfo,
        None,
    ) {
        Ok(FexecResult::Interp { .. }) => {
            syscall::write(2, b"lerux-proc: PT_INTERP not supported for initfs ELFs\n").ok();
            unsafe { core::arch::asm!("ud2", options(noreturn)) };
        }
        Ok(_) => unsafe { core::arch::asm!("ud2", options(noreturn)) },
        Err(err) => {
            let msg = format!("lerux-proc: fexec {path}: {err}\n");
            syscall::write(2, msg.as_bytes()).ok();
            unsafe { core::arch::asm!("ud2", options(noreturn)) };
        }
    }
}
