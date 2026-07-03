#![no_std]
#![no_main]

use lerux_interface_types::{NetRequest, NetResponse};
use lerux_ipc::{recv, send, send_unspecified_error};
#[cfg(not(feature = "workstation"))]
use lerux_logging::debug;
use lerux_logging::log;
#[cfg(feature = "workstation")]
use lerux_logging::server;
use sel4_driver_interfaces::net::GetNetDeviceMeta;
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible, MessageInfo};
use sel4_microkit_driver_adapters::net::client::Client as NetClient;

mod config;
mod net;

const NET_DRIVER: Channel = Channel::new(1);
const CLIENT: Channel = Channel::new(2);
#[cfg(feature = "workstation")]
const LOG_SERVER: Channel = Channel::new(4);

struct HandlerImpl {
    net: net::NetStack,
    completed: Option<NetResponse>,
}

#[protection_domain(heap_size = 512 * 1024)]
fn init() -> HandlerImpl {
    #[cfg(feature = "workstation")]
    server::init(LOG_SERVER).unwrap();
    #[cfg(not(feature = "workstation"))]
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
        completed: None,
    }
}

impl HandlerImpl {
    fn handle_poll(&mut self) -> NetResponse {
        if let Some(resp) = self.completed.take() {
            return resp;
        }
        if let Some(resp) = self.net.take_completed() {
            return resp;
        }
        NET_DRIVER.notify();
        self.net.poll();
        self.net.take_completed().unwrap_or(NetResponse::Pending)
    }

    fn handle_net_driver(&mut self) {
        self.net.poll();
        if let Some(resp) = self.net.take_completed() {
            self.completed = Some(resp);
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
        if channel != CLIENT && channel != Channel::new(3) {
            // Channel 3 used by shell on workstation
            unreachable!("unexpected net client");
        }

        Ok(match recv::<NetRequest>(msg_info) {
            Ok(req) => match req {
                NetRequest::UdpTx {
                    payload_len,
                    payload,
                } => {
                    self.net.queue_udp_tx(payload_len, payload);
                    send(NetResponse::Pending)
                }
                NetRequest::DnsResolve { name_len, name } => {
                    self.net.queue_dns_resolve(name_len, name);
                    send(self.net.take_completed().unwrap_or(NetResponse::Pending))
                }
                NetRequest::TcpConnect { addr, port } => {
                    self.net.queue_tcp_connect(addr, port);
                    send(NetResponse::Pending)
                }
                NetRequest::TcpSend {
                    payload_len,
                    payload,
                } => {
                    self.net.queue_tcp_send(payload_len, payload);
                    send(NetResponse::Pending)
                }
                NetRequest::TcpRecv => {
                    self.net.queue_tcp_recv();
                    send(NetResponse::Pending)
                }
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
