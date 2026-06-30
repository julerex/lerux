#![no_std]
#![no_main]

use lerux_interface_types::{BlockRequest, BlockResponse, SECTOR_SIZE};
use lerux_ipc::call;
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

const SERIAL_DRIVER: Channel = Channel::new(0);
const BLK_SERVER: Channel = Channel::new(1);
#[cfg(feature = "composed-sync")]
const BOOT_INIT: Channel = Channel::new(2);

struct HandlerImpl {
    #[cfg(feature = "composed-sync")]
    blk_pending: bool,
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
        if self.blk_pending && channels.contains(BOOT_INIT) {
            probe_blk();
            self.blk_pending = false;
        }
        Ok(())
    }
}
