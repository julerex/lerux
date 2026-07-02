#![no_std]
#![no_main]

use lerux_logging::{log, serial};
use rtcc::{DateTimeAccess, Datelike};
use sel4_driver_interfaces::timer::Clock;
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};
use sel4_microkit_driver_adapters::{
    rtc::client::Client as RtcClient, timer::client::Client as TimerClient,
};

#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
use lerux_interface_types::{FsRequest, FsResponse, NetRequest, NetResponse};
#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
use lerux_ipc::call;

const SERIAL_DRIVER: Channel = Channel::new(0);
const RTC_DRIVER: Channel = Channel::new(1);
const TIMER_DRIVER: Channel = Channel::new(2);
#[cfg(any(
    feature = "board-qemu_virt_aarch64_composed",
    feature = "board-qemu_virt_aarch64_http_composed",
    feature = "board-qemu_virt_aarch64_blk_composed",
    feature = "board-qemu_virt_aarch64_net_composed",
    feature = "board-qemu_virt_aarch64_ipc_composed"
))]
const APP: Channel = Channel::new(3);

#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
const FS_SERVER: Channel = Channel::new(3);
#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
const NET_SERVER: Channel = Channel::new(4);

struct HandlerImpl;

fn log_rtc(rtc: &mut RtcClient) {
    let dt = rtc.datetime().unwrap();
    log::info!(
        "lerux-supervisor: RTC {:04}-{:02}-{:02}",
        dt.year(),
        dt.month(),
        dt.day()
    );
}

fn log_timer(timer: &mut TimerClient) {
    let elapsed = timer.get_time().unwrap();
    log::info!("lerux-supervisor: timer {}ms", elapsed.as_millis());
    log::info!("lerux-supervisor: timer ok");
}

#[cfg(any(
    feature = "board-qemu_virt_aarch64_composed",
    feature = "board-qemu_virt_aarch64_http_composed",
    feature = "board-qemu_virt_aarch64_blk_composed",
    feature = "board-qemu_virt_aarch64_net_composed",
    feature = "board-qemu_virt_aarch64_ipc_composed"
))]
fn notify_app() {
    APP.notify();
}

#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
fn poll_fs() -> FsResponse {
    loop {
        match call::<FsRequest, FsResponse>(FS_SERVER, FsRequest::Poll).expect("FS Poll IPC") {
            FsResponse::Pending => {}
            other => return other,
        }
    }
}

#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
fn probe_fs() {
    // Exercise FS server to ensure FS is "mounted" (triggers format if needed)
    match call::<FsRequest, FsResponse>(FS_SERVER, FsRequest::ListDir) {
        Ok(FsResponse::Pending) => {
            let _ = poll_fs();
        }
        Ok(FsResponse::DirList { .. }) | Ok(FsResponse::Ok) | Err(_) => {}
        _ => {}
    }
    log::info!("lerux-supervisor: fs up");
}

#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
fn poll_net() -> NetResponse {
    loop {
        match call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::Poll).expect("Net Poll IPC") {
            NetResponse::Pending => {}
            other => return other,
        }
    }
}

#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
fn probe_net() {
    // Exercise net server to ensure "net up"
    let pending =
        call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::udp_tx(b"lerux-workstation"))
            .expect("Net UdpTx IPC");
    if matches!(pending, NetResponse::Pending) {
        let _ = poll_net();
    }
    log::info!("lerux-supervisor: net up");
}

#[protection_domain]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();
    let mut rtc = RtcClient::new(RTC_DRIVER);
    log_rtc(&mut rtc);
    let mut timer = TimerClient::new(TIMER_DRIVER);
    log_timer(&mut timer);
    log::info!("lerux-supervisor: init ok");
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_composed",
        feature = "board-qemu_virt_aarch64_http_composed",
        feature = "board-qemu_virt_aarch64_blk_composed",
        feature = "board-qemu_virt_aarch64_net_composed",
        feature = "board-qemu_virt_aarch64_ipc_composed"
    ))]
    notify_app();
    #[cfg(feature = "board-qemu_virt_aarch64_workstation")]
    {
        probe_fs();
        probe_net();
        log::info!("lerux-supervisor: ready");
    }
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;
}
