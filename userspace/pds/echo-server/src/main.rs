#![no_std]
#![no_main]

use lerux_interface_types::{EchoRequest, EchoResponse};
use lerux_ipc::{recv, send, send_unspecified_error};
use lerux_logging::log;
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};

#[cfg(not(feature = "serial-ipc"))]
use lerux_logging::debug;
#[cfg(feature = "serial-ipc")]
use lerux_logging::serial;

// Channel 1: IPC from echo_client (<end pd="echo_server" id="1" />).
const CLIENT: Channel = Channel::new(1);
#[cfg(feature = "serial-ipc")]
const SERIAL_DRIVER: Channel = Channel::new(0);

fn init_logging() {
    #[cfg(feature = "serial-ipc")]
    serial::init(SERIAL_DRIVER).unwrap();

    #[cfg(not(feature = "serial-ipc"))]
    debug::init().unwrap();
}

#[protection_domain]
fn init() -> HandlerImpl {
    init_logging();
    log::info!("echo-server ready");
    HandlerImpl
}

struct HandlerImpl;

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != CLIENT {
            unreachable!();
        }

        Ok(match recv::<EchoRequest>(msg_info) {
            Ok(req) => match req {
                EchoRequest::Ping => send(EchoResponse::Pong),
                EchoRequest::Echo { len, text } => send(EchoResponse::Echo { len, text }),
            },
            Err(_) => send_unspecified_error(),
        })
    }
}
