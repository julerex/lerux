#![no_std]
#![no_main]

use lerux_logging::{log, serial};
use rtcc::{DateTimeAccess, Datelike};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};
use sel4_microkit_driver_adapters::rtc::client::Client as RtcClient;

const SERIAL_DRIVER: Channel = Channel::new(0);
const RTC_DRIVER: Channel = Channel::new(1);

struct HandlerImpl;

#[protection_domain]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();

    let mut rtc = RtcClient::new(RTC_DRIVER);
    let dt = rtc.datetime().unwrap();
    log::info!(
        "lerux-init: RTC {:04}-{:02}-{:02}",
        dt.year(),
        dt.month(),
        dt.day()
    );
    log::info!("lerux-init: init ok");

    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;
}