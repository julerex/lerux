#![no_std]
#![no_main]

use lerux_logging::log;
use sel4_driver_interfaces::net::GetNetDeviceMeta;
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};
use sel4_microkit_driver_adapters::net::client::Client as NetClient;

#[cfg(not(feature = "serial-ipc"))]
use lerux_logging::debug;
#[cfg(feature = "serial-ipc")]
use lerux_logging::serial;

mod config;
mod net;

#[cfg(all(feature = "serial-ipc", feature = "composed-sync"))]
const SERIAL_DRIVER: Channel = Channel::new(2);
#[cfg(all(feature = "serial-ipc", not(feature = "composed-sync")))]
const SERIAL_DRIVER: Channel = Channel::new(0);
#[cfg(feature = "composed-sync")]
const BOOT_INIT: Channel = Channel::new(0);
const NET_DRIVER: Channel = Channel::new(1);

struct HandlerImpl {
    #[cfg(feature = "composed-sync")]
    net_pending: bool,
    net: Option<net::HttpNet>,
}

fn init_logging() {
    #[cfg(feature = "serial-ipc")]
    serial::init(SERIAL_DRIVER).unwrap();

    #[cfg(not(feature = "serial-ipc"))]
    debug::init().unwrap();
}

#[cfg(feature = "composed-sync")]
fn init_composed_sync() -> HandlerImpl {
    HandlerImpl {
        net_pending: true,
        net: None,
    }
}

fn prime_net_stack(http_net: &mut net::HttpNet) {
    for _ in 0..2000 {
        http_net.poll();
    }
}

#[cfg(feature = "board-x86_64_generic_http")]
fn kick_virtio_driver() {
    NET_DRIVER.notify();
}

#[cfg(feature = "board-x86_64_generic_http")]
fn wait_for_inbound(http_net: &mut net::HttpNet) {
    log::info!("lerux-http: waiting for GET / (host: curl http://127.0.0.1:18080/)");
    while !http_net.is_served() {
        kick_virtio_driver();
        http_net.poll();
    }
    for _ in 0..500 {
        kick_virtio_driver();
        http_net.poll();
    }
}

#[cfg(feature = "board-x86_64_generic_http")]
fn drive_net(http_net: &mut net::HttpNet) {
    let mut flush_after_serve = 0;
    for _ in 0..4000 {
        kick_virtio_driver();
        http_net.poll();
        if http_net.is_served() {
            flush_after_serve = 500;
        }
        if flush_after_serve > 0 {
            flush_after_serve -= 1;
            if flush_after_serve == 0 {
                break;
            }
        }
    }
}

#[cfg(not(feature = "board-x86_64_generic_http"))]
fn drive_net(http_net: &mut net::HttpNet) {
    http_net.poll();
    if http_net.is_served() {
        for _ in 0..20 {
            http_net.poll();
        }
    }
}

#[cfg(not(feature = "composed-sync"))]
fn init_with_net() -> HandlerImpl {
    log::info!("lerux-http: starting");
    let mut net_client = NetClient::new(NET_DRIVER);
    let mac = log_net_mac(&mut net_client);
    let mut http_net = net::HttpNet::new(mac);
    prime_net_stack(&mut http_net);
    // x86 PCI virtio-net: poll shared rings during init until GET / is served;
    // post-init driver notifications alone are unreliable for passive listen.
    #[cfg(feature = "board-x86_64_generic_http")]
    wait_for_inbound(&mut http_net);
    HandlerImpl {
        net: Some(http_net),
    }
}

#[protection_domain(heap_size = 512 * 1024)]
fn init() -> HandlerImpl {
    init_logging();
    #[cfg(feature = "composed-sync")]
    return init_composed_sync();
    #[cfg(not(feature = "composed-sync"))]
    init_with_net()
}

fn log_net_mac(net_client: &mut NetClient) -> sel4_driver_interfaces::net::MacAddress {
    let mac = net_client.get_mac_address().unwrap();
    log::info!(
        "lerux-http: MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac.0[0],
        mac.0[1],
        mac.0[2],
        mac.0[3],
        mac.0[4],
        mac.0[5],
    );
    mac
}

#[cfg(feature = "composed-sync")]
fn start_net() -> net::HttpNet {
    let mut net_client = NetClient::new(NET_DRIVER);
    let mac = log_net_mac(&mut net_client);
    let mut http_net = net::HttpNet::new(mac);
    prime_net_stack(&mut http_net);
    http_net
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        #[cfg(feature = "composed-sync")]
        if self.net_pending && channels.contains(BOOT_INIT) {
            self.net = Some(start_net());
            self.net_pending = false;
        }

        if channels.contains(NET_DRIVER)
            && let Some(http_net) = &mut self.net
        {
            drive_net(http_net);
        }
        Ok(())
    }
}
