#![no_std]
#![no_main]

use lerux_logging::log;
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

#[cfg(feature = "serial-ipc")]
use lerux_logging::serial;
#[cfg(not(feature = "serial-ipc"))]
use lerux_logging::debug;

#[cfg(feature = "virtio")]
use sel4_driver_interfaces::block::GetBlockDeviceLayout;
#[cfg(feature = "virtio")]
use sel4_driver_interfaces::net::GetNetDeviceMeta;
#[cfg(feature = "virtio")]
use sel4_microkit_driver_adapters::block::client::Client as BlockClient;
#[cfg(feature = "virtio")]
use sel4_microkit_driver_adapters::net::client::Client as NetClient;

#[cfg(feature = "serial-ipc")]
const SERIAL_DRIVER: Channel = Channel::new(0);
#[cfg(feature = "virtio")]
const NET_DRIVER: Channel = Channel::new(1);
#[cfg(feature = "virtio")]
const BLK_DRIVER: Channel = Channel::new(2);

#[protection_domain]
fn init() -> HandlerImpl {
    init_logging();
    log::info!("lerux: Hello from Rust on seL4 Microkit!");
    probe_virtio();
    HandlerImpl
}

fn init_logging() {
    #[cfg(feature = "serial-ipc")]
    serial::init(SERIAL_DRIVER).unwrap();

    #[cfg(not(feature = "serial-ipc"))]
    debug::init().unwrap();
}

#[cfg(feature = "virtio")]
fn probe_virtio() {
    let mut blk = BlockClient::new(BLK_DRIVER);
    let block_size = blk.get_block_size().unwrap();
    let num_blocks = blk.get_num_blocks().unwrap();
    log::info!("virtio-blk: {num_blocks} blocks x {block_size} bytes");

    let mut net = NetClient::new(NET_DRIVER);
    let mac = net.get_mac_address().unwrap();
    log::info!(
        "virtio-net: MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac.0[0],
        mac.0[1],
        mac.0[2],
        mac.0[3],
        mac.0[4],
        mac.0[5],
    );
}

#[cfg(not(feature = "virtio"))]
fn probe_virtio() {}

struct HandlerImpl;

impl Handler for HandlerImpl {
    type Error = Infallible;
}