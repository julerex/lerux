#![no_std]
#![no_main]

extern crate alloc;

use lerux_html::{parse, SMOKE_FIXTURE};
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

/// Channel 0: serial-driver (`<end pd="html_demo" id="0" pp="true" />`).
const SERIAL_DRIVER: Channel = Channel::new(0);

struct HandlerImpl;

#[protection_domain(heap_size = 64 * 1024)]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();
    let doc = parse(SMOKE_FIXTURE);
    log::info!("lerux-html: nodes={}", doc.node_count());
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;
}
