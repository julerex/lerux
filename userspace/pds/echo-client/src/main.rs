#![no_std]
#![no_main]

use core::str;

use lerux_interface_types::{EchoRequest, EchoResponse};
use lerux_ipc::call;
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

// Channel 0: serial driver (<end pd="echo_client" id="0" pp="true" />).
const SERIAL_DRIVER: Channel = Channel::new(0);
// Channel 1: echo server (<end pd="echo_client" id="1" pp="true" />).
const ECHO_SERVER: Channel = Channel::new(1);

#[protection_domain]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();
    probe_echo();
    #[cfg(feature = "bench")]
    bench_echo();
    HandlerImpl
}

fn probe_echo() {
    let pong = call::<EchoRequest, EchoResponse>(ECHO_SERVER, EchoRequest::Ping).unwrap();
    assert!(matches!(pong, EchoResponse::Pong));
    log::info!("lerux-echo: pong");

    let echoed =
        call::<EchoRequest, EchoResponse>(ECHO_SERVER, EchoRequest::echo(b"lerux")).unwrap();
    let text = echoed.as_echo_slice().unwrap();
    log::info!(
        "lerux-echo: {}",
        str::from_utf8(text).unwrap_or("<invalid utf-8>")
    );
}

/// Phase 49: N Ping PPCs; host times wall-clock between start/done lines.
#[cfg(feature = "bench")]
fn bench_echo() {
    const WARMUP: u32 = 32;
    const N: u32 = 1_000;
    for _ in 0..WARMUP {
        let _ = call::<EchoRequest, EchoResponse>(ECHO_SERVER, EchoRequest::Ping).unwrap();
    }
    log::info!("lerux-bench: echo start n={N}");
    for _ in 0..N {
        let pong = call::<EchoRequest, EchoResponse>(ECHO_SERVER, EchoRequest::Ping).unwrap();
        assert!(matches!(pong, EchoResponse::Pong));
    }
    log::info!("lerux-bench: echo done n={N}");
}

struct HandlerImpl;

impl Handler for HandlerImpl {
    type Error = Infallible;
}
