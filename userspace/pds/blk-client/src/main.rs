#![no_std]
#![no_main]

use lerux_interface_types::{BlockRequest, BlockResponse, SECTOR_SIZE};
use lerux_ipc::BlkClient;
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

const SERIAL_DRIVER: Channel = Channel::new(0);
const BLK_SERVER: BlkClient = BlkClient::new(Channel::new(1));
#[cfg(feature = "composed-sync")]
const SUPERVISOR: Channel = Channel::new(2);
#[cfg(feature = "composed-chain")]
const NET_CLIENT: Channel = Channel::new(3);

const TEST_LBA: u32 = 1;
const WRITE_MAGIC: &[u8] = b"lerux-blk-write-test";

struct HandlerImpl {
    #[cfg(feature = "composed-sync")]
    blk_pending: bool,
}

fn read_sector(lba: u32) -> [u8; SECTOR_SIZE] {
    match BLK_SERVER.call(BlockRequest::ReadSector { lba }) {
        BlockResponse::Sector { data } => data,
        _ => panic!("blk read failed"),
    }
}

fn write_sector(lba: u32, data: [u8; SECTOR_SIZE]) {
    match BLK_SERVER.call(BlockRequest::WriteSector { lba, data }) {
        BlockResponse::Ok => {}
        _ => panic!("blk write failed"),
    }
}

fn probe_blk() {
    let sector = read_sector(0);
    log::info!(
        "lerux-blk: MBR sig 0x{:02x} 0x{:02x}",
        sector[SECTOR_SIZE - 2],
        sector[SECTOR_SIZE - 1]
    );

    let mut write_data = [0u8; SECTOR_SIZE];
    let magic_len = WRITE_MAGIC.len().min(SECTOR_SIZE);
    write_data[..magic_len].copy_from_slice(&WRITE_MAGIC[..magic_len]);
    write_sector(TEST_LBA, write_data);

    let read_back = read_sector(TEST_LBA);
    assert!(
        read_back[..magic_len] == WRITE_MAGIC[..magic_len],
        "write round-trip mismatch"
    );
    log::info!("lerux-blk: write round-trip ok");

    #[cfg(feature = "bench")]
    bench_blk_read();
}

/// Phase 49: N sector reads; host times wall-clock between start/done lines.
#[cfg(feature = "bench")]
fn bench_blk_read() {
    const WARMUP: u32 = 16;
    const N: u32 = 500;
    for _ in 0..WARMUP {
        let _ = read_sector(0);
    }
    log::info!("lerux-bench: blk_read start n={N}");
    for _ in 0..N {
        let _ = read_sector(0);
    }
    log::info!("lerux-bench: blk_read done n={N}");
}

#[cfg(feature = "composed-sync")]
fn init_composed() -> HandlerImpl {
    HandlerImpl { blk_pending: true }
}

#[cfg(not(feature = "composed-sync"))]
fn init_standalone() -> HandlerImpl {
    probe_blk();
    HandlerImpl {}
}

#[protection_domain]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();
    #[cfg(feature = "composed-sync")]
    return init_composed();
    #[cfg(not(feature = "composed-sync"))]
    init_standalone()
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(
        &mut self,
        #[cfg_attr(
            not(feature = "composed-sync"),
            expect(
                unused_variables,
                reason = "no sync notifications without composed-sync"
            )
        )]
        channels: ChannelSet,
    ) -> Result<(), Self::Error> {
        #[cfg(feature = "composed-sync")]
        if self.blk_pending && channels.contains(SUPERVISOR) {
            probe_blk();
            self.blk_pending = false;
            #[cfg(feature = "composed-chain")]
            NET_CLIENT.notify();
        }
        Ok(())
    }
}
