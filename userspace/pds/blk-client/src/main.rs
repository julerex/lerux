#![no_std]
#![no_main]

use lerux_interface_types::{BlockRequest, BlockResponse, SECTOR_SIZE};
use lerux_ipc::call;
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

const SERIAL_DRIVER: Channel = Channel::new(0);
const BLK_SERVER: Channel = Channel::new(1);

#[protection_domain]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();
    probe_blk();
    HandlerImpl
}

fn probe_blk() {
    let pending =
        call::<BlockRequest, BlockResponse>(BLK_SERVER, BlockRequest::ReadSector { lba: 0 })
            .expect("ReadSector IPC");
    assert!(matches!(pending, BlockResponse::Pending));

    let sector = loop {
        match call::<BlockRequest, BlockResponse>(BLK_SERVER, BlockRequest::Poll).expect("Poll IPC")
        {
            BlockResponse::Sector { data } => break data,
            BlockResponse::Pending => {}
            BlockResponse::Error => panic!("blk read failed"),
        }
    };

    log::info!(
        "lerux-blk: MBR sig 0x{:02x} 0x{:02x}",
        sector[SECTOR_SIZE - 2],
        sector[SECTOR_SIZE - 1]
    );
}

struct HandlerImpl;

impl Handler for HandlerImpl {
    type Error = Infallible;
}
