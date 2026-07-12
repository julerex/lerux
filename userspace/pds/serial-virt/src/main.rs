//! Serial virtualiser PD (Phase 42).
//!
//! Presents multi-client postcard serial RPC and forwards each request to a
//! device-only `serial-driver` over a single protected channel.

#![no_std]
#![no_main]

mod handler;

use handler::HandlerImpl;
use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Channel, Handler};

/// Channel 0: notify + protected RPC to `serial-driver`.
const DRIVER: Channel = Channel::new(0);

#[cfg(not(any(feature = "multi-client-2", feature = "multi-client-3")))]
const CLIENTS: [Channel; 1] = [Channel::new(1)];

#[cfg(feature = "multi-client-2")]
const CLIENTS: [Channel; 2] = [Channel::new(1), Channel::new(2)];

#[cfg(feature = "multi-client-3")]
const CLIENTS: [Channel; 3] = [Channel::new(1), Channel::new(2), Channel::new(3)];

#[cfg(not(any(feature = "multi-client-2", feature = "multi-client-3")))]
type VirtHandler = HandlerImpl<1>;

#[cfg(feature = "multi-client-2")]
type VirtHandler = HandlerImpl<2>;

#[cfg(feature = "multi-client-3")]
type VirtHandler = HandlerImpl<3>;

#[protection_domain]
fn init() -> impl Handler {
    debug::init().unwrap();
    log::info!("serial-virt: multi-client mux → serial-driver");
    VirtHandler::new(DRIVER, CLIENTS)
}
