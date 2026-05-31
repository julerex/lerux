#![no_std]
#![no_main]
#![feature(never_type)]

extern crate alloc;

#[allow(unused_imports)]
use lerux_shim as _;

use redox_scheme::Socket;
use scheme_utils::Blocking;

use crate::scheme::{Ty, ZeroScheme};

mod scheme;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;
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

#[unsafe(no_mangle)]
fn lerux_rt_main() -> ! {
    fn daemon(daemon: daemon::SchemeDaemon) -> ! {
        let arg = lerux_entry::stack()
            .arg(1)
            .expect("zerod: needs null or zero as argument");
        let ty = match arg {
            b"null" => Ty::Null,
            b"zero" => Ty::Zero,
            _ => panic!("zerod: needs to be called with either null or zero as argument"),
        };

        let socket = Socket::create().expect("zerod: failed to create zero scheme");
        let mut zero_scheme = ZeroScheme(ty);
        let zero_handler = Blocking::new(&socket, 16);

        let _ = daemon.ready_sync_scheme(&socket, &mut zero_scheme);

        libredox::call::setrens(0, 0).expect("zerod: failed to enter null namespace");

        zero_handler
            .process_requests_blocking(zero_scheme)
            .expect("zerod: failed to process events from zero scheme");
    }

    daemon::SchemeDaemon::new(daemon);
}
