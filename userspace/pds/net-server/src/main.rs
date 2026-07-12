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

// Channel IDs must match support/profiles + system templates (Phase 41/43).
// Sole L2 client of virtio-net / genet / virtio-pci driver PDs.
const NET_DRIVER: Channel = Channel::new(1);
/// Default smoke client (net-client / supervisor on some boards).
const CLIENT: Channel = Channel::new(2);
#[cfg(feature = "workstation")]
const LOG_SERVER: Channel = Channel::new(4);
/// http-file-browser on workstation (net_server id 7).
#[cfg(feature = "workstation")]
const HTTP_FS_CLIENT: Channel = Channel::new(7);

struct HandlerImpl {
    net: net::NetStack,
    /// Response waiting for the owning client to Poll.
    completed: Option<NetResponse>,
    /// Client that owns the in-flight async operation (or pending completion).
    active_client: Option<Channel>,
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
        active_client: None,
    }
}

impl HandlerImpl {
    /// Reserve this client for an async op. Returns false when another client owns the stack
    /// or this client still has an undelivered completion.
    fn begin_async(&mut self, channel: Channel) -> bool {
        if self.completed.is_some() {
            return false;
        }
        if self.net.is_busy() && self.active_client != Some(channel) {
            return false;
        }
        self.active_client = Some(channel);
        true
    }

    fn finish_async(&mut self) {
        self.active_client = None;
    }

    fn abort_async(&mut self, channel: Channel) {
        if self.active_client == Some(channel) {
            self.net.cancel_recv();
            self.completed = None;
            self.finish_async();
        }
    }

    fn handle_poll(&mut self, channel: Channel) -> NetResponse {
        if self.active_client != Some(channel) {
            return NetResponse::Pending;
        }
        if let Some(resp) = self.completed.take() {
            self.finish_async();
            return resp;
        }
        if let Some(resp) = self.net.take_completed() {
            self.finish_async();
            return resp;
        }
        NET_DRIVER.notify();
        self.net.poll();
        if let Some(resp) = self.net.take_completed() {
            self.finish_async();
            return resp;
        }
        NetResponse::Pending
    }

    fn handle_net_driver(&mut self) {
        self.net.poll();
        if let Some(resp) = self.net.take_completed() {
            self.completed = Some(resp);
        }
        #[cfg(feature = "workstation")]
        if self.net.listen_activity {
            HTTP_FS_CLIENT.notify();
        }
    }

    fn is_client(channel: Channel) -> bool {
        channel == CLIENT
            || channel == Channel::new(3)
            || channel == Channel::new(5)
            || channel == Channel::new(6)
            || channel == Channel::new(7)
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if !Self::is_client(channel) {
            // 2=sup, 3=shell, 5=config, 6=chat, 7=http-file-browser (workstation)
            unreachable!("unexpected net client");
        }

        Ok(match recv::<NetRequest>(msg_info) {
            Ok(req) => match req {
                NetRequest::UdpTx {
                    payload_len,
                    payload,
                } => {
                    if !self.begin_async(channel) {
                        return Ok(send(NetResponse::Pending));
                    }
                    self.net.queue_udp_tx(payload_len, payload);
                    send(NetResponse::Pending)
                }
                NetRequest::UdpRecv => {
                    if !self.begin_async(channel) {
                        return Ok(send(NetResponse::Pending));
                    }
                    self.net.queue_udp_recv();
                    send(NetResponse::Pending)
                }
                NetRequest::DnsResolve { name_len, name } => {
                    self.net.queue_dns_resolve(name_len, name);
                    send(self.net.take_completed().unwrap_or(NetResponse::Pending))
                }
                NetRequest::TcpConnect { addr, port } => {
                    if !self.begin_async(channel) {
                        return Ok(send(NetResponse::Pending));
                    }
                    self.net.queue_tcp_connect(addr, port);
                    send(NetResponse::Pending)
                }
                NetRequest::TcpListen { port } => {
                    if !self.begin_async(channel) {
                        return Ok(send(NetResponse::Pending));
                    }
                    self.net.queue_tcp_listen(port);
                    send(NetResponse::Pending)
                }
                NetRequest::TcpSend {
                    payload_len,
                    payload,
                } => {
                    if !self.begin_async(channel) {
                        return Ok(send(NetResponse::Pending));
                    }
                    self.net.queue_tcp_send(payload_len, payload);
                    send(NetResponse::Pending)
                }
                NetRequest::TcpRecv => {
                    if !self.begin_async(channel) {
                        return Ok(send(NetResponse::Pending));
                    }
                    self.net.queue_tcp_recv();
                    send(NetResponse::Pending)
                }
                NetRequest::TcpClose => {
                    if !self.begin_async(channel) {
                        return Ok(send(NetResponse::Pending));
                    }
                    self.net.queue_tcp_close();
                    send(NetResponse::Pending)
                }
                NetRequest::Abort => {
                    self.abort_async(channel);
                    send(NetResponse::Ok)
                }
                NetRequest::Poll => send(self.handle_poll(channel)),
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
