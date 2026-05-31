#![no_std]
#![no_main]
#![feature(never_type)]

extern crate alloc;

#[allow(unused_imports)]
use lerux_shim as _;

use redox_scheme::Socket;
use scheme_utils::Blocking;

mod scheme;

use crate::scheme::RamfsScheme;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    rt_bin::panic_handler(info)
}

#[unsafe(no_mangle)]
fn lerux_rt_main() -> ! {
    fn daemon(daemon: daemon::SchemeDaemon) -> ! {
        let name = core::str::from_utf8(
            lerux_entry::stack()
                .arg(1)
                .expect("ramfs: missing scheme name"),
        )
        .expect("ramfs: scheme name not utf-8");
        let socket = Socket::create().expect("ramfs: failed to create socket");
        let mut scheme = RamfsScheme::new(name);
        let handler = Blocking::new(&socket, 16);
        let _ = daemon.ready_sync_scheme(&socket, &mut scheme);
        libredox::call::setrens(0, 0).expect("ramfs: failed to enter null namespace");
        handler
            .process_requests_blocking(scheme)
            .expect("ramfs: failed to process events");
    }
    daemon::SchemeDaemon::new(daemon);
}
