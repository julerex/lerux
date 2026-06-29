#![no_std]
#![no_main]

use lerux_interface_types::{EchoRequest, EchoResponse};
use lerux_ipc::{recv, send, send_unspecified_error};
use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};

// Channel 1: IPC from echo_client (<end pd="echo_server" id="1" />).
const CLIENT: Channel = Channel::new(1);

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().unwrap();
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
