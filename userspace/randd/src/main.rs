#![no_std]
#![no_main]
#![feature(never_type)]

extern crate alloc;

#[allow(unused_imports)]
use lerux_shim as _;

mod scheme;

use redox_scheme::Socket;
use scheme_utils::Blocking;

use crate::scheme::RandScheme;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    rt_bin::panic_handler(info)
}

#[unsafe(no_mangle)]
fn lerux_rt_main() -> ! {
    fn daemon(daemon: daemon::SchemeDaemon) -> ! {
        let socket = Socket::create().expect("randd: failed to create rand scheme");
        let mut scheme = RandScheme::new();
        let handler = Blocking::new(&socket, 16);
        let _ = daemon.ready_sync_scheme(&socket, &mut scheme);
        libredox::call::setrens(0, 0).expect("randd: failed to enter null namespace");
        handler
            .process_requests_blocking(scheme)
            .expect("randd: failed to process events from rand scheme");
    }
    daemon::SchemeDaemon::new(daemon);
}
