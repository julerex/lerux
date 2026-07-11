#![no_std]
#![no_main]

use lerux_interface_types::{NetRequest, NetResponse};
use lerux_ipc::call;
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

const SERIAL_DRIVER: Channel = Channel::new(0);
const NET_SERVER: Channel = Channel::new(1);
#[cfg(feature = "composed-sync")]
const SUPERVISOR: Channel = Channel::new(2);

struct HandlerImpl {
    #[cfg(feature = "composed-sync")]
    net_pending: bool,
}

fn poll_net() -> NetResponse {
    loop {
        match call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::Poll).expect("Poll IPC") {
            NetResponse::Pending => {}
            other => return other,
        }
    }
}

fn probe_net() {
    let pending = call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::udp_tx(b"lerux-net"))
        .expect("UdpTx IPC");
    assert!(matches!(pending, NetResponse::Pending));

    match poll_net() {
        NetResponse::Ok => log::info!("lerux-net: IPC ok"),
        NetResponse::Pending
        | NetResponse::Error
        | NetResponse::Ipv4 { .. }
        | NetResponse::TcpData { .. }
        | NetResponse::UdpData { .. } => panic!("net TX failed"),
    }
}

#[cfg(feature = "composed-sync")]
fn init_composed() -> HandlerImpl {
    HandlerImpl { net_pending: true }
}

#[cfg(not(feature = "composed-sync"))]
fn init_standalone() -> HandlerImpl {
    probe_net();
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
        if self.net_pending && channels.contains(SUPERVISOR) {
            probe_net();
            self.net_pending = false;
        }
        Ok(())
    }
}
