//! Phase 46: deliberate VM fault so the parent `debug-handler` receives it.
//!
//! Null-pointer store produces a data abort (VmFault) under aarch64 seL4.

#![no_std]
#![no_main]

use core::ptr;

use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Handler, Infallible};

struct HandlerImpl;

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    log::info!("crash-demo: starting");
    log::info!("crash-demo: about to fault");
    // Deliberate null write — delivered to parent PD as VmFault.
    unsafe {
        ptr::write_volatile(ptr::null_mut::<u32>(), 0xdead_beef);
    }
    // Unreachable if the fault is delivered as expected.
    log::error!("crash-demo: survived fault (unexpected)");
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;
}
