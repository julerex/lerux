#![no_std]
#![no_main]

use lerux_logging::{log, serial};
use rtcc::{DateTimeAccess, Datelike};
use sel4_driver_interfaces::timer::Clock;
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};
use sel4_microkit_driver_adapters::rtc::client::Client as RtcClient;
use sel4_microkit_driver_adapters::timer::client::Client as TimerClient;

const SERIAL_DRIVER: Channel = Channel::new(0);
const RTC_DRIVER: Channel = Channel::new(1);
const TIMER_DRIVER: Channel = Channel::new(2);
#[cfg(feature = "board-qemu_virt_aarch64_composed")]
const HELLO: Channel = Channel::new(3);

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

    let mut timer = TimerClient::new(TIMER_DRIVER);
    let elapsed = timer.get_time().unwrap();
    log::info!("lerux-init: timer {}ms", elapsed.as_millis());
    log::info!("lerux-init: timer ok");

    log::info!("lerux-init: init ok");

    #[cfg(feature = "board-qemu_virt_aarch64_composed")]
    HELLO.notify();

    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;
}