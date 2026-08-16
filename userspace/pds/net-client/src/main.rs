#![no_std]
#![no_main]

use lerux_interface_types::{NetRequest, NetResponse};
use lerux_ipc::NetClient;
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

const SERIAL_DRIVER: Channel = Channel::new(0);
const NET_SERVER: NetClient = NetClient::new(Channel::new(1));
#[cfg(feature = "composed-sync")]
const SUPERVISOR: Channel = Channel::new(2);

struct HandlerImpl {
    #[cfg(feature = "composed-sync")]
    net_pending: bool,
}

fn probe_net() {
    match NET_SERVER.call(NetRequest::udp_tx(b"lerux-net")) {
        NetResponse::Ok => log::info!("lerux-net: IPC ok"),
        _ => panic!("net TX failed"),
    }

    #[cfg(feature = "bench")]
    bench_udp_tx();

    apply_static_smoke();
}

/// Phase 54: ApplyIface static must be visible on GetIface without reboot.
fn apply_static_smoke() {
    let NetResponse::Iface {
        addr,
        prefix,
        gateway,
        dns,
        ..
    } = NET_SERVER.call(NetRequest::GetIface)
    else {
        panic!("net iface unavailable");
    };
    match NET_SERVER.call(NetRequest::apply_iface(false, addr, prefix, gateway, dns)) {
        NetResponse::Ok => {}
        _ => panic!("net apply failed"),
    }
    match NET_SERVER.call(NetRequest::GetIface) {
        NetResponse::Iface {
            dhcp: false,
            addr: got,
            ..
        } if got == addr => log::info!("lerux-net: apply static"),
        _ => panic!("net apply not visible"),
    }
}

/// Phase 49: N UdpTx+Poll completions; host times wall-clock between start/done.
#[cfg(feature = "bench")]
fn bench_udp_tx() {
    const WARMUP: u32 = 16;
    const N: u32 = 200;
    for _ in 0..WARMUP {
        let _ = NET_SERVER.call(NetRequest::udp_tx(b"b"));
    }
    log::info!("lerux-bench: udp_tx start n={N}");
    for _ in 0..N {
        assert!(matches!(
            NET_SERVER.call(NetRequest::udp_tx(b"b")),
            NetResponse::Ok
        ));
    }
    log::info!("lerux-bench: udp_tx done n={N}");
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
