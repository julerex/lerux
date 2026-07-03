#![no_std]
#![no_main]

use lerux_logging::{debug, log};
use sel4_microkit::{
    memory_region_symbol, protection_domain, Channel, ChannelSet, Handler, Infallible,
};

mod config;

const CLIENT: Channel = Channel::new(1);

struct Emmc2Driver;

impl Emmc2Driver {
    fn new() -> Self {
        Self
    }

    fn read_mbr(&mut self) {
        log::info!("emmc2: stub read of block 0 (MBR)");
    }
}

struct HandlerImpl {
    dev: Emmc2Driver,
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(CLIENT) {
            // blk-server posted work. In full impl drain free ring, perform emmc cmd, enqueue to used.
            self.dev.read_mbr();
        }
        Ok(())
    }
}

#[protection_domain(heap_size = 128 * 1024)]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    log::info!("emmc2-driver: RPi4 bcm2711-emmc2 native driver (stub for Phase 37)");

    let _mmio = memory_region_symbol!(emmc2_mmio_vaddr: *mut ());
    let _ = _mmio;
    log::info!("emmc2: registers mapped (full SDHCI/ADMA + DMA setup TODO)");

    let dev = Emmc2Driver::new();
    HandlerImpl { dev }
}
