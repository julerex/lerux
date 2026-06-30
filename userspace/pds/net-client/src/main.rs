#![no_std]
#![no_main]

use lerux_interface_types::{NetRequest, NetResponse};
use lerux_ipc::call;
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

const SERIAL_DRIVER: Channel = Channel::new(0);
const NET_SERVER: Channel = Channel::new(1);

struct HandlerImpl;

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
        NetResponse::Pending | NetResponse::Error => panic!("net TX failed"),
    }
}

#[protection_domain]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();
    probe_net();
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;
}
