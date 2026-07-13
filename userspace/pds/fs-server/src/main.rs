#![no_std]
#![no_main]

extern crate alloc;

#[cfg(not(feature = "workstation"))]
use lerux_logging::debug;
use lerux_logging::log;
#[cfg(feature = "workstation")]
use lerux_logging::server;
use sel4_driver_interfaces::block::GetBlockDeviceLayout;
use sel4_microkit::protection_domain;
use sel4_microkit_driver_adapters::block::client::Client as BlockClient;

mod block_io;
mod config;
mod shell;

#[cfg(feature = "backend-fat")]
mod fat_format;
#[cfg(not(feature = "backend-fat"))]
mod leruxfs_format;

#[cfg(feature = "backend-fat")]
use fat_format::FatFormat as Format;
#[cfg(not(feature = "backend-fat"))]
use leruxfs_format::LeruxFsFormat as Format;
use shell::FsServer;

use block_io::BLK_DRIVER;
#[cfg(feature = "workstation")]
use block_io::LOG_SERVER;

#[protection_domain(heap_size = 512 * 1024)]
fn init() -> FsServer<Format> {
    #[cfg(feature = "workstation")]
    server::init_with_tag(LOG_SERVER, b"fs").unwrap();
    #[cfg(not(feature = "workstation"))]
    debug::init().unwrap();
    let mut blk = BlockClient::new(BLK_DRIVER);
    let block_size = blk.get_block_size().unwrap();
    let num_blocks = blk.get_num_blocks().unwrap();
    log::info!("virtio-blk: {num_blocks} blocks x {block_size} bytes");
    FsServer::new(Format::new(block_size))
}
