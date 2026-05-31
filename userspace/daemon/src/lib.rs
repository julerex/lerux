//! A library for creating and managing daemons for RedoxOS.
#![no_std]
#![feature(never_type)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::str;

use libredox::Fd;
use redox_scheme::Socket;
use redox_scheme::scheme::{SchemeAsync, SchemeSync};

fn parse_init_notify_fd() -> usize {
    let val = lerux_entry::env_var(lerux_entry::stack(), "INIT_NOTIFY")
        .expect("daemon: INIT_NOTIFY not set");
    str::from_utf8(val)
        .expect("daemon: INIT_NOTIFY not UTF-8")
        .parse()
        .expect("daemon: INIT_NOTIFY not a number")
}

fn set_fd_cloexec(fd: usize, cloexec: bool) -> syscall::Result<()> {
    let flags = if cloexec {
        syscall::CallFlags::FD_CLOEXEC.bits()
    } else {
        0
    };
    syscall::fcntl(fd, syscall::F_SETFD, flags).map(|_| ())
}

/// A long running background process that handles requests using schemes.
pub struct SchemeDaemon {
    notify_fd: usize,
}

impl SchemeDaemon {
    /// Create a new daemon for use with schemes.
    pub fn new(f: impl FnOnce(SchemeDaemon) -> !) -> ! {
        let notify_fd = parse_init_notify_fd();
        set_fd_cloexec(notify_fd, true).expect("daemon: failed to set CLOEXEC on INIT_NOTIFY");
        f(SchemeDaemon { notify_fd })
    }

    /// Notify the process that the scheme daemon is ready to accept requests.
    pub fn ready_with_fd(self, cap_fd: Fd) -> syscall::Result<()> {
        syscall::call_wo(
            self.notify_fd,
            &cap_fd.into_raw().to_ne_bytes(),
            syscall::CallFlags::FD,
            &[],
        )?;
        Ok(())
    }

    /// Notify the process that the synchronous scheme daemon is ready to accept requests.
    pub fn ready_sync_scheme<S: SchemeSync>(
        self,
        socket: &Socket,
        scheme: &mut S,
    ) -> syscall::Result<()> {
        let cap_id = scheme.scheme_root()?;
        let cap_fd = socket.create_this_scheme_fd(0, cap_id, 0, 0)?;
        self.ready_with_fd(Fd::new(cap_fd))
    }

    /// Notify the process that the asynchronous scheme daemon is ready to accept requests.
    pub fn ready_async_scheme<S: SchemeAsync>(
        self,
        socket: &Socket,
        scheme: &mut S,
    ) -> syscall::Result<()> {
        let cap_id = scheme.scheme_root()?;
        let cap_fd = socket.create_this_scheme_fd(0, cap_id, 0, 0)?;
        self.ready_with_fd(Fd::new(cap_fd))
    }
}
