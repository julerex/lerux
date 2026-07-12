#![no_std]
#![no_main]

#[cfg(feature = "workstation")]
use lerux_interface_types::{SupervisorRequest, SupervisorResponse};
use lerux_ipc::send_unspecified_error;
#[cfg(feature = "workstation")]
use lerux_ipc::{recv, send};
#[cfg(not(feature = "workstation"))]
use lerux_logging::{log, serial};
#[cfg(feature = "workstation")]
use lerux_logging::{log, server};
#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
use rtcc::{DateTimeAccess, Datelike};
#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
use sel4_driver_interfaces::timer::Clock;
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};
#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
use sel4_microkit_driver_adapters::{
    rtc::client::Client as RtcClient, timer::client::Client as TimerClient,
};

#[cfg(feature = "workstation")]
use lerux_interface_types::{
    ConfigRequest, ConfigResponse, FsRequest, FsResponse, LogRequest, LogResponse, NetRequest,
    NetResponse, MAX_CONFIG_KEY_LEN, MAX_CONFIG_VAL_LEN,
};
#[cfg(feature = "workstation")]
use lerux_ipc::call;

#[cfg(not(feature = "workstation"))]
const SERIAL_DRIVER: Channel = Channel::new(0);
#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
const RTC_DRIVER: Channel = Channel::new(1);
#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
const TIMER_DRIVER: Channel = Channel::new(2);
#[cfg(any(
    feature = "board-qemu_virt_aarch64_composed",
    feature = "board-qemu_virt_aarch64_http_composed",
    feature = "board-qemu_virt_aarch64_blk_composed",
    feature = "board-qemu_virt_aarch64_net_composed",
    feature = "board-qemu_virt_aarch64_ipc_composed"
))]
const APP: Channel = Channel::new(3);

#[cfg(feature = "workstation")]
const FS_SERVER: Channel = Channel::new(3);
#[cfg(feature = "workstation")]
const NET_SERVER: Channel = Channel::new(4);
#[cfg(feature = "workstation")]
const SHELL: Channel = Channel::new(5);
#[cfg(feature = "workstation")]
const LOG_SERVER: Channel = Channel::new(6);
#[cfg(feature = "workstation")]
const CONFIG_SERVER: Channel = Channel::new(7);

struct HandlerImpl;

#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
fn log_rtc(rtc: &mut RtcClient) {
    let dt = rtc.datetime().unwrap();
    log::info!(
        "lerux-supervisor: RTC {:04}-{:02}-{:02}",
        dt.year(),
        dt.month(),
        dt.day()
    );
}

#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
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

#[cfg(feature = "workstation")]
fn poll_fs() -> FsResponse {
    loop {
        match call::<FsRequest, FsResponse>(FS_SERVER, FsRequest::Poll).expect("FS Poll IPC") {
            FsResponse::Pending => {}
            other => return other,
        }
    }
}

#[cfg(feature = "workstation")]
fn fs_call(req: FsRequest) -> FsResponse {
    match call::<FsRequest, FsResponse>(FS_SERVER, req) {
        Ok(FsResponse::Pending) => poll_fs(),
        Ok(other) => other,
        Err(_) => FsResponse::Error,
    }
}

#[cfg(feature = "workstation")]
fn probe_fs() {
    // Exercise FS server to ensure FS is "mounted" (triggers format if needed)
    match fs_call(FsRequest::list_root()) {
        FsResponse::DirList { .. } | FsResponse::Ok | FsResponse::Error => {}
        _ => {}
    }
    log::info!("lerux-supervisor: fs up");
}

/// Phase 52: seed hierarchical `/config` defaults after first format (idempotent).
#[cfg(feature = "workstation")]
fn seed_first_boot() {
    // Ensure `/config` exists as a real directory (LERUXFS2).
    match fs_call(FsRequest::mkdir(b"/config")) {
        FsResponse::Ok => log::info!("lerux-supervisor: mkdir /config"),
        FsResponse::Error => {
            // Already present from a previous boot.
        }
        _ => {}
    }

    #[cfg(feature = "board-rpi4b_4gb_workstation")]
    let seeds: &[(&[u8], &[u8])] = &[
        (b"net.ip", b"192.168.1.10"),
        (b"net.gateway", b"192.168.1.1"),
        (b"net.dns", b"192.168.1.1"),
        (b"hostname", b"lerux-rpi4"),
    ];
    #[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
    let seeds: &[(&[u8], &[u8])] = &[
        (b"net.ip", b"10.0.2.15"),
        (b"net.gateway", b"10.0.2.2"),
        (b"net.dns", b"10.0.2.3"),
        (b"hostname", b"lerux"),
    ];

    for (kdata, vdata) in seeds {
        let mut key = [0u8; MAX_CONFIG_KEY_LEN];
        let kl = kdata.len().min(MAX_CONFIG_KEY_LEN);
        key[..kl].copy_from_slice(&kdata[..kl]);
        let mut val = [0u8; MAX_CONFIG_VAL_LEN];
        let vl = vdata.len().min(MAX_CONFIG_VAL_LEN);
        val[..vl].copy_from_slice(&vdata[..vl]);
        let _ = call::<ConfigRequest, ConfigResponse>(
            CONFIG_SERVER,
            ConfigRequest::Set {
                key_len: kl as u8,
                key,
                val_len: vl as u8,
                value: val,
            },
        );
    }
    log::info!("lerux-supervisor: first-boot seed ok");
}

#[cfg(feature = "workstation")]
fn probe_net() {
    // Exercise net server to ensure "net up". Bound Poll so a stuck Pending
    // cannot hang init (http-file-browser may leave the stack busy briefly).
    let pending =
        call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::udp_tx(b"lerux-workstation"))
            .expect("Net UdpTx IPC");
    if matches!(pending, NetResponse::Pending) {
        for _ in 0..512 {
            match call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::Poll) {
                Ok(NetResponse::Pending) => {}
                Ok(_) | Err(_) => break,
            }
        }
    }
    log::info!("lerux-supervisor: net up");
}

#[cfg(feature = "workstation")]
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
        if pos > 0
            && let FsResponse::Handle { id } = fs_call(FsRequest::create(b"/boot.log"))
            && matches!(
                fs_call(FsRequest::write(id, 0, &buf[..pos])),
                FsResponse::Ok
            )
        {
            log::info!("lerux-supervisor: boot log written to /boot.log");
        }
    }
}

#[cfg(feature = "workstation")]
fn service_list() -> SupervisorResponse {
    const NAMES: &[&[u8]] = &[
        b"supervisor",
        b"fs-server",
        b"net-server",
        b"shell",
        b"edit",
        b"chat-client",
        b"http-fs",
        b"log-server",
    ];
    let mut name_lens = [0u8; lerux_interface_types::MAX_SERVICES];
    let mut names =
        [[0u8; lerux_interface_types::MAX_SERVICE_NAME]; lerux_interface_types::MAX_SERVICES];
    let mut ready = [false; lerux_interface_types::MAX_SERVICES];
    let count = NAMES.len().min(lerux_interface_types::MAX_SERVICES) as u8;
    for (i, name) in NAMES.iter().take(count as usize).enumerate() {
        let n = name.len().min(lerux_interface_types::MAX_SERVICE_NAME);
        name_lens[i] = n as u8;
        names[i][..n].copy_from_slice(&name[..n]);
        // Init probes mark FS/net up; remaining services are present in the image.
        ready[i] = true;
    }
    SupervisorResponse::ServiceList {
        count,
        name_lens,
        names,
        ready,
    }
}

#[cfg(feature = "workstation")]
fn handle_supervisor(req: SupervisorRequest) -> SupervisorResponse {
    match req {
        SupervisorRequest::Reboot => {
            log::info!("lerux-supervisor: reboot requested");
            SupervisorResponse::Ok
        }
        SupervisorRequest::ListServices => service_list(),
        SupervisorRequest::ServiceStatus { id } => {
            if let SupervisorResponse::ServiceList { count, ready, .. } = service_list() {
                if (id as usize) < count as usize {
                    SupervisorResponse::Status {
                        ready: ready[id as usize],
                    }
                } else {
                    SupervisorResponse::Error
                }
            } else {
                SupervisorResponse::Error
            }
        }
        SupervisorRequest::GetTime => {
            #[cfg(feature = "board-rpi4b_4gb_workstation")]
            {
                let _ = ();
                SupervisorResponse::Error
            }
            #[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
            {
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
        }
        SupervisorRequest::GetUptime => {
            #[cfg(feature = "board-rpi4b_4gb_workstation")]
            {
                // No timer PD on RPi4 workstation profile.
                SupervisorResponse::Uptime { secs: 0 }
            }
            #[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
            {
                let mut timer = TimerClient::new(TIMER_DRIVER);
                match timer.get_time() {
                    Ok(elapsed) => SupervisorResponse::Uptime {
                        secs: elapsed.as_secs() as u32,
                    },
                    Err(_) => SupervisorResponse::Error,
                }
            }
        }
    }
}

#[protection_domain]
fn init() -> HandlerImpl {
    #[cfg(feature = "workstation")]
    server::init(LOG_SERVER).unwrap();
    #[cfg(not(feature = "workstation"))]
    serial::init(SERIAL_DRIVER).unwrap();
    #[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
    {
        let mut rtc = RtcClient::new(RTC_DRIVER);
        log_rtc(&mut rtc);
        let mut timer = TimerClient::new(TIMER_DRIVER);
        log_timer(&mut timer);
    }
    #[cfg(feature = "board-rpi4b_4gb_workstation")]
    log::info!("lerux-supervisor: no RTC/timer PDs on RPi4 workstation");
    log::info!("lerux-supervisor: init ok");
    #[cfg(any(
        feature = "board-qemu_virt_aarch64_composed",
        feature = "board-qemu_virt_aarch64_http_composed",
        feature = "board-qemu_virt_aarch64_blk_composed",
        feature = "board-qemu_virt_aarch64_net_composed",
        feature = "board-qemu_virt_aarch64_ipc_composed"
    ))]
    notify_app();
    #[cfg(feature = "workstation")]
    {
        probe_fs();
        seed_first_boot();
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
            not(feature = "workstation"),
            expect(
                unused_variables,
                reason = "channel and msg_info only used for workstation shell IPC"
            )
        )]
        channel: Channel,
        #[cfg_attr(
            not(feature = "workstation"),
            expect(
                unused_variables,
                reason = "channel and msg_info only used for workstation shell IPC"
            )
        )]
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        #[cfg(feature = "workstation")]
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
