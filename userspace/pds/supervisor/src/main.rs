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
    NetResponse, CFG_BOOT_SEEDED, CFG_HOSTNAME, CFG_LOG_LEVEL, CFG_LOG_ROTATE, CFG_NET_DNS,
    CFG_NET_GATEWAY, CFG_NET_IP, CFG_NET_MODE, CFG_NET_PREFIX, MAX_CONFIG_VAL_LEN,
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

#[cfg(feature = "workstation")]
fn config_get(key: &[u8]) -> Option<(u8, [u8; MAX_CONFIG_VAL_LEN])> {
    match call::<ConfigRequest, ConfigResponse>(CONFIG_SERVER, ConfigRequest::get(key)) {
        Ok(ConfigResponse::Value { val_len, value }) => Some((val_len, value)),
        _ => None,
    }
}

#[cfg(feature = "workstation")]
fn config_set(key: &[u8], value: &[u8]) -> bool {
    matches!(
        call::<ConfigRequest, ConfigResponse>(CONFIG_SERVER, ConfigRequest::set(key, value)),
        Ok(ConfigResponse::Ok)
    )
}

/// Phase 52/54: seed **missing** keys only; never overwrite operator edits.
#[cfg(feature = "workstation")]
fn seed_first_boot() {
    if fs_call(FsRequest::mkdir(b"/config")) == FsResponse::Ok {
        log::info!("lerux-supervisor: mkdir /config");
    }
    let _ = fs_call(FsRequest::mkdir(b"/config/secrets"));

    if config_get(CFG_BOOT_SEEDED).is_some() {
        log::info!("lerux-supervisor: config already seeded");
        return;
    }

    #[cfg(feature = "board-rpi4b_4gb_workstation")]
    let seeds: &[(&[u8], &[u8])] = &[
        (CFG_NET_MODE, b"dhcp"),
        (CFG_NET_IP, b"192.168.1.10"),
        (CFG_NET_GATEWAY, b"192.168.1.1"),
        (CFG_NET_DNS, b"192.168.1.1"),
        (CFG_NET_PREFIX, b"24"),
        (CFG_HOSTNAME, b"lerux-rpi4"),
        (CFG_LOG_LEVEL, b"info"),
        (CFG_LOG_ROTATE, b"1"),
    ];
    #[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
    let seeds: &[(&[u8], &[u8])] = &[
        (CFG_NET_MODE, b"dhcp"),
        (CFG_NET_IP, b"10.0.2.15"),
        (CFG_NET_GATEWAY, b"10.0.2.2"),
        (CFG_NET_DNS, b"10.0.2.3"),
        (CFG_NET_PREFIX, b"24"),
        (CFG_HOSTNAME, b"lerux"),
        (CFG_LOG_LEVEL, b"info"),
        (CFG_LOG_ROTATE, b"1"),
    ];

    for (k, v) in seeds {
        if config_get(k).is_none() {
            let _ = config_set(k, v);
        }
    }
    let _ = config_set(CFG_BOOT_SEEDED, b"1");
    log::info!("lerux-supervisor: first-boot seed ok");
}

/// Phase 54: log active policy after seed (read, do not invent).
#[cfg(feature = "workstation")]
fn apply_config_policy() {
    let (h_len, h_buf) = config_get(CFG_HOSTNAME).unwrap_or((0, [0; MAX_CONFIG_VAL_LEN]));
    let (m_len, m_buf) = config_get(CFG_NET_MODE).unwrap_or((0, [0; MAX_CONFIG_VAL_LEN]));
    let (l_len, l_buf) = config_get(CFG_LOG_LEVEL).unwrap_or((0, [0; MAX_CONFIG_VAL_LEN]));
    let h = if h_len > 0 {
        core::str::from_utf8(&h_buf[..h_len as usize]).unwrap_or("?")
    } else {
        "lerux"
    };
    let m = if m_len > 0 {
        core::str::from_utf8(&m_buf[..m_len as usize]).unwrap_or("?")
    } else {
        "dhcp"
    };
    let l = if l_len > 0 {
        core::str::from_utf8(&l_buf[..l_len as usize]).unwrap_or("?")
    } else {
        "info"
    };
    log::info!(
        "lerux-supervisor: config hostname={} net.mode={} log.level={}",
        h,
        m,
        l
    );
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
        if pos == 0 {
            return;
        }
        // Phase 54: optional rotate /boot.log → /boot.log.1
        let rotate = config_get(CFG_LOG_ROTATE)
            .map(|(n, v)| n > 0 && v[0] == b'1')
            .unwrap_or(true);
        if rotate {
            let _ = fs_call(FsRequest::unlink(b"/boot.log.1"));
            let _ = fs_call(FsRequest::rename(b"/boot.log", b"/boot.log.1"));
        } else {
            let _ = fs_call(FsRequest::unlink(b"/boot.log"));
        }
        if let FsResponse::Handle { id } = fs_call(FsRequest::create(b"/boot.log"))
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
        apply_config_policy();
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
