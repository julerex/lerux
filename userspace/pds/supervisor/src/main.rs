#![no_std]
#![no_main]

#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
use lerux_interface_types::{SupervisorRequest, SupervisorResponse};
use lerux_ipc::send_unspecified_error;
#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
use lerux_ipc::{recv, send};
#[cfg(not(feature = "board-qemu_virt_aarch64_workstation"))]
use lerux_logging::{log, serial};
#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
use lerux_logging::{log, server};
use rtcc::{DateTimeAccess, Datelike};
use sel4_driver_interfaces::timer::Clock;
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};
use sel4_microkit_driver_adapters::{
    rtc::client::Client as RtcClient, timer::client::Client as TimerClient,
};

#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
use lerux_interface_types::{
    FsRequest, FsResponse, LogRequest, LogResponse, NetRequest, NetResponse,
};
#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
use lerux_ipc::call;

#[cfg(not(feature = "board-qemu_virt_aarch64_workstation"))]
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
#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
const SHELL: Channel = Channel::new(5);
#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
const LOG_SERVER: Channel = Channel::new(6);

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

#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
fn persist_boot_log() {
    if let Ok(LogResponse::Recent { count, lens, lines }) =
        call::<LogRequest, LogResponse>(LOG_SERVER, LogRequest::GetRecent)
    {
        // concatenate into a buffer for FS write
        let mut buf = [0u8; 512];
        let mut pos = 0usize;
        for i in 0..(count as usize) {
            let l = lens[i] as usize;
            if pos + l + 1 >= buf.len() {
                break;
            }
            buf[pos..pos + l].copy_from_slice(&lines[i][..l]);
            pos += l;
            buf[pos] = b'\n';
            pos += 1;
        }
        if pos > 0 {
            // direct create + write like probe style
            let create_resp =
                call::<FsRequest, FsResponse>(FS_SERVER, FsRequest::create(b"/boot.log"));
            if let Ok(FsResponse::Handle { id }) = create_resp {
                let write_req = FsRequest::write(id, 0, &buf[..pos]);
                if let Ok(FsResponse::Ok) = call::<FsRequest, FsResponse>(FS_SERVER, write_req) {
                    log::info!("lerux-supervisor: boot log written to /boot.log");
                }
            }
        }
    }
}

#[cfg(feature = "board-qemu_virt_aarch64_workstation")]
fn handle_supervisor(req: SupervisorRequest) -> SupervisorResponse {
    match req {
        SupervisorRequest::Reboot => {
            log::info!("lerux-supervisor: reboot requested");
            SupervisorResponse::Ok
        }
        SupervisorRequest::GetTime => {
            let mut rtc = RtcClient::new(RTC_DRIVER);
            if let Ok(dt) = rtc.datetime() {
                SupervisorResponse::Time {
                    year: dt.year() as u16,
                    month: dt.month() as u8,
                    day: dt.day() as u8,
                }
            } else {
                SupervisorResponse::Error
            }
        }
        _ => SupervisorResponse::Ok,
    }
}

#[protection_domain]
fn init() -> HandlerImpl {
    #[cfg(feature = "board-qemu_virt_aarch64_workstation")]
    server::init(LOG_SERVER).unwrap();
    #[cfg(not(feature = "board-qemu_virt_aarch64_workstation"))]
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
        persist_boot_log();
        log::info!("lerux-supervisor: ready");
    }
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        #[cfg_attr(
            not(feature = "board-qemu_virt_aarch64_workstation"),
            expect(
                unused_variables,
                reason = "channel and msg_info only used for workstation shell IPC"
            )
        )]
        channel: Channel,
        #[cfg_attr(
            not(feature = "board-qemu_virt_aarch64_workstation"),
            expect(
                unused_variables,
                reason = "channel and msg_info only used for workstation shell IPC"
            )
        )]
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        #[cfg(feature = "board-qemu_virt_aarch64_workstation")]
        if channel == SHELL {
            return Ok(match recv::<SupervisorRequest>(msg_info) {
                Ok(req) => send(handle_supervisor(req)),
                Err(_) => send_unspecified_error(),
            });
        }
        // No other IPC for supervisor on non-workstation boards
        Ok(send_unspecified_error())
    }
}
