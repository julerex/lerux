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
mod queue;

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
    clients: queue::ClientQueue,
}

#[protection_domain(heap_size = 512 * 1024)]
fn init() -> HandlerImpl {
    #[cfg(feature = "workstation")]
    server::init_with_tag(LOG_SERVER, b"net").unwrap();
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
    // DHCP / static fallback: each poll advances fake time (~20 ms).
    for _ in 0..400 {
        net_stack.poll();
        NET_DRIVER.notify();
    }
    log::info!("lerux-net: ready");
    HandlerImpl {
        net: net_stack,
        clients: queue::ClientQueue::new(),
    }
}

impl HandlerImpl {
    fn client_idx(channel: Channel) -> usize {
        channel.index()
    }

    fn begin_async(&mut self, channel: Channel) -> bool {
        self.clients.begin(Self::client_idx(channel))
    }

    fn start_or_queue(&mut self, channel: Channel, req: NetRequest) -> NetResponse {
        if !self.begin_async(channel) {
            return NetResponse::Pending;
        }
        let idx = Self::client_idx(channel);
        if self.net.is_busy() && self.clients.current != Some(idx) {
            self.clients.queue_request(idx, req);
            log::info!("lerux-net: queued");
            return NetResponse::Pending;
        }
        self.dispatch(channel, req);
        if let Some(resp) = self.clients.take_completed(idx) {
            self.clients.finish(idx);
            self.pump_queue();
            return resp;
        }
        NetResponse::Pending
    }

    fn dispatch(&mut self, channel: Channel, req: NetRequest) {
        let idx = Self::client_idx(channel);
        if self.clients.current.is_some() && self.clients.current != Some(idx) && self.net.is_busy()
        {
            self.clients.queue_request(idx, req);
            return;
        }
        if self.clients.current.is_some() && self.clients.current != Some(idx) {
            log::info!("lerux-net: multi-client ok");
        }
        self.clients.current = Some(idx);
        match req {
            NetRequest::UdpTx {
                payload_len,
                payload,
            } => self.net.queue_udp_tx(payload_len, payload),
            NetRequest::UdpRecv => self.net.queue_udp_recv(),
            NetRequest::DnsResolve { name_len, name } => self.net.queue_dns_resolve(name_len, name),
            NetRequest::TcpConnect { addr, port } => self.net.queue_tcp_connect(addr, port),
            NetRequest::TcpListen { port } => self.net.queue_tcp_listen(port),
            NetRequest::TcpSend {
                payload_len,
                payload,
            } => self.net.queue_tcp_send(payload_len, payload),
            NetRequest::TcpRecv => self.net.queue_tcp_recv(),
            NetRequest::TcpClose => self.net.queue_tcp_close(),
            NetRequest::Abort
            | NetRequest::Poll
            | NetRequest::GetIface
            | NetRequest::ApplyIface { .. } => {}
        }
        if let Some(resp) = self.net.take_completed() {
            self.clients.stash(idx, resp);
            self.clients.current = None;
            self.pump_queue();
        }
    }

    fn pump_queue(&mut self) {
        while !self.net.is_busy() {
            let Some(idx) = self.clients.next_queued() else {
                break;
            };
            let Some(req) = self.clients.take_queued(idx) else {
                break;
            };
            log::info!("lerux-net: multi-client ok");
            let ch = Channel::new(idx);
            self.dispatch(ch, req);
        }
    }

    fn abort_async(&mut self, channel: Channel) {
        let idx = Self::client_idx(channel);
        if self.clients.current == Some(idx) {
            self.net.cancel_async();
        }
        self.clients.abort(idx);
        self.pump_queue();
    }

    fn handle_poll(&mut self, channel: Channel) -> NetResponse {
        let idx = Self::client_idx(channel);
        if let Some(resp) = self.clients.take_completed(idx) {
            self.clients.finish(idx);
            self.pump_queue();
            return resp;
        }
        if self.clients.current == Some(idx) {
            NET_DRIVER.notify();
            self.net.poll();
            if let Some(resp) = self.net.take_completed() {
                self.clients.finish(idx);
                self.pump_queue();
                return resp;
            }
        }
        self.pump_queue();
        NetResponse::Pending
    }

    fn handle_net_driver(&mut self) {
        self.net.poll();
        if let Some(idx) = self.clients.current
            && let Some(resp) = self.net.take_completed()
        {
            self.clients.stash(idx, resp);
            self.clients.current = None;
            self.pump_queue();
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
                NetRequest::GetIface => send(self.net.iface_response()),
                NetRequest::ApplyIface {
                    dhcp,
                    addr,
                    prefix,
                    gateway,
                    dns,
                } => send(self.net.apply_iface(dhcp, addr, prefix, gateway, dns)),
                NetRequest::Abort => {
                    self.abort_async(channel);
                    send(NetResponse::Ok)
                }
                NetRequest::Poll => send(self.handle_poll(channel)),
                other => send(self.start_or_queue(channel, other)),
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
