#![no_std]
#![no_main]

use lerux_interface_types::{NetRequest, NetResponse};
use lerux_ipc::{recv, send, send_unspecified_error};
use lerux_logging::{debug, log};
use sel4_driver_interfaces::net::GetNetDeviceMeta;
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible, MessageInfo};
use sel4_microkit_driver_adapters::net::client::Client as NetClient;

mod config;
mod net;

const NET_DRIVER: Channel = Channel::new(1);
const CLIENT: Channel = Channel::new(2);

struct HandlerImpl {
    net: net::NetStack,
    completed_ok: bool,
}

#[protection_domain(heap_size = 512 * 1024)]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    let mut net_client = NetClient::new(NET_DRIVER);
    let mac = net_client.get_mac_address().unwrap();
    log::info!(
        "virtio-net: MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac.0[0],
        mac.0[1],
        mac.0[2],
        mac.0[3],
        mac.0[4],
        mac.0[5],
    );
    let mut net_stack = net::NetStack::new(mac);
    for _ in 0..2000 {
        net_stack.poll();
    }
    log::info!("lerux-net: ready");
    HandlerImpl {
        net: net_stack,
        completed_ok: false,
    }
}

impl HandlerImpl {
    fn handle_udp_tx(
        &mut self,
        payload_len: u8,
        payload: [u8; lerux_interface_types::MAX_NET_UDP_PAYLOAD],
    ) -> NetResponse {
        self.net.queue_udp_tx(payload_len, payload);
        NetResponse::Pending
    }

    fn handle_poll(&mut self) -> NetResponse {
        if self.completed_ok {
            self.completed_ok = false;
            return NetResponse::Ok;
        }
        NET_DRIVER.notify();
        self.net.poll();
        if self.net.is_tx_done() {
            self.completed_ok = true;
        }
        NetResponse::Pending
    }

    fn handle_net_driver(&mut self) {
        self.net.poll();
        if self.net.is_tx_done() {
            self.completed_ok = true;
        }
    }
}

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

        Ok(match recv::<NetRequest>(msg_info) {
            Ok(req) => match req {
                NetRequest::UdpTx {
                    payload_len,
                    payload,
                } => send(self.handle_udp_tx(payload_len, payload)),
                NetRequest::Poll => send(self.handle_poll()),
            },
            Err(_) => send_unspecified_error(),
        })
    }

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(NET_DRIVER) {
            self.handle_net_driver();
        }
        Ok(())
    }
}
