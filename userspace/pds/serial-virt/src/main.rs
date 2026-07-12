//! Serial virtualiser PD (Phase 42).
//!
//! Presents the same postcard serial RPC as the legacy multi-client
//! `serial-driver`, and forwards bytes over sDDF-shaped queues to a
//! device-only `serial-driver`.

#![no_std]
#![no_main]

mod handler;

use handler::HandlerImpl;
use lerux_logging::{debug, log};
use lerux_serial_queue::{SerialQueue, SerialQueueHandle, DEFAULT_CAPACITY};
use sel4_microkit::{memory_region_symbol, protection_domain, Channel, Handler};

/// Channel 0: notify from / to `serial-driver`.
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
    log::info!("serial-virt: multi-client mux over queues");

    // SAFETY: regions mapped shared with serial-driver in the system description.
    let (tx, rx) = unsafe {
        let tx_q = memory_region_symbol!(serial_tx_queue: *mut SerialQueue).as_ptr();
        let tx_d = memory_region_symbol!(serial_tx_data: *mut [u8], n = DEFAULT_CAPACITY)
            .as_ptr()
            .cast::<u8>();
        let rx_q = memory_region_symbol!(serial_rx_queue: *mut SerialQueue).as_ptr();
        let rx_d = memory_region_symbol!(serial_rx_data: *mut [u8], n = DEFAULT_CAPACITY)
            .as_ptr()
            .cast::<u8>();
        (
            SerialQueueHandle::new(tx_q, tx_d, DEFAULT_CAPACITY),
            SerialQueueHandle::new(rx_q, rx_d, DEFAULT_CAPACITY),
        ) // SAFETY: handled by enclosing unsafe block
    };
    // Queue headers are initialized by serial-driver (device-only) at boot.

    VirtHandler::new(DRIVER, CLIENTS, tx, rx)
}
