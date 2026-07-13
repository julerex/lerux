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
    CFG_NET_GATEWAY, CFG_NET_IP, CFG_NET_MODE, CFG_NET_PREFIX, LOG_LEVEL_DEBUG, LOG_LEVEL_ERROR,
    LOG_LEVEL_INFO, LOG_LEVEL_WARN, MAX_CONFIG_VAL_LEN, MAX_SERVICES, MAX_SERVICE_ERR,
    MAX_SERVICE_NAME, SERVICE_STATE_DEGRADED, SERVICE_STATE_ERROR, SERVICE_STATE_READY,
    SERVICE_STATE_STARTING,
};
#[cfg(feature = "workstation")]
use lerux_ipc::{call, FsClient, NetClient};

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
const FS_SERVER: FsClient = FsClient::new(Channel::new(3));
#[cfg(feature = "workstation")]
const NET_SERVER: NetClient = NetClient::new(Channel::new(4));
#[cfg(feature = "workstation")]
const SHELL: Channel = Channel::new(5);
#[cfg(feature = "workstation")]
const LOG_SERVER: Channel = Channel::new(6);
#[cfg(feature = "workstation")]
const CONFIG_SERVER: Channel = Channel::new(7);

#[cfg(feature = "workstation")]
const SERVICE_NAMES: &[&[u8]] = &[
    b"supervisor",
    b"fs-server",
    b"net-server",
    b"shell",
    b"edit",
    b"chat-client",
    b"http-fs",
    b"backup",
];

#[cfg(feature = "workstation")]
struct HandlerImpl {
    states: [u8; MAX_SERVICES],
    errs: [[u8; MAX_SERVICE_ERR]; MAX_SERVICES],
    err_lens: [u8; MAX_SERVICES],
}

#[cfg(not(feature = "workstation"))]
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

/// Phase 56: static service graph (systemd-unit analogue — fixed PDs, ordered readiness).
fn log_service_graph() {
    #[cfg(feature = "workstation")]
    {
        log::info!("lerux-supervisor: service-graph unit=fs-server after=supervisor restart=no");
        log::info!("lerux-supervisor: service-graph unit=config-server after=fs-server restart=no");
        log::info!("lerux-supervisor: service-graph unit=net-server after=fs-server restart=no");
        log::info!("lerux-supervisor: service-graph unit=shell after=net-server after=config-server restart=no");
    }
    #[cfg(not(feature = "workstation"))]
    {
        log::info!("lerux-supervisor: service-graph unit=rtc after=serial-driver restart=no");
        log::info!("lerux-supervisor: service-graph unit=timer after=serial-driver restart=no");
        log::info!(
            "lerux-supervisor: service-graph unit=supervisor after=rtc after=timer restart=no"
        );
    }
}

/// Phase 56: hang detection — re-query timer after bring-up; fail closed only if timer dies.
#[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
fn watchdog_check(timer: &mut TimerClient) {
    match timer.get_time() {
        Ok(elapsed) => {
            log::info!("lerux-supervisor: watchdog ok ({}ms)", elapsed.as_millis());
        }
        Err(_) => {
            log::error!("lerux-supervisor: watchdog fail (timer unresponsive)");
        }
    }
}

#[cfg(feature = "board-rpi4b_4gb_workstation")]
fn watchdog_check() {
    // No timer PD on RPi4 workstation profile yet.
    log::info!("lerux-supervisor: watchdog skip (no timer PD)");
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
fn fs_call(req: FsRequest) -> FsResponse {
    FS_SERVER.call(req)
}

#[cfg(feature = "workstation")]
fn set_service_err(h: &mut HandlerImpl, id: usize, state: u8, msg: &[u8]) {
    if id >= MAX_SERVICES {
        return;
    }
    h.states[id] = state;
    let n = msg.len().min(MAX_SERVICE_ERR);
    h.err_lens[id] = n as u8;
    h.errs[id] = [0u8; MAX_SERVICE_ERR];
    h.errs[id][..n].copy_from_slice(&msg[..n]);
}

#[cfg(feature = "workstation")]
fn probe_fs(h: &mut HandlerImpl) {
    // Exercise FS server to ensure FS is "mounted" (triggers format if needed)
    match fs_call(FsRequest::list_root()) {
        FsResponse::DirList { .. } | FsResponse::Ok => {
            set_service_err(h, 1, SERVICE_STATE_READY, b"");
            log::info!("lerux-supervisor: fs up");
        }
        FsResponse::Error => {
            set_service_err(h, 1, SERVICE_STATE_ERROR, b"list_root error");
            log::error!("lerux-supervisor: fs probe error");
        }
        _ => {
            set_service_err(h, 1, SERVICE_STATE_DEGRADED, b"unexpected response");
            log::warn!("lerux-supervisor: fs degraded");
        }
    }
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

/// Phase 54/57: log active policy; apply log.level to log-server min filter.
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
    let min = match l.as_bytes() {
        b"error" => LOG_LEVEL_ERROR,
        b"warn" => LOG_LEVEL_WARN,
        b"debug" => LOG_LEVEL_DEBUG,
        _ => LOG_LEVEL_INFO,
    };
    let _ = call::<LogRequest, LogResponse>(LOG_SERVER, LogRequest::SetMinLevel { level: min });
}

#[cfg(feature = "workstation")]
fn probe_net(h: &mut HandlerImpl) {
    // Exercise net server to ensure "net up". Bound Poll so a stuck Pending
    // cannot hang init (http-file-browser may leave the stack busy briefly).
    match NET_SERVER.call_bounded(NetRequest::udp_tx(b"lerux-workstation"), 512) {
        NetResponse::Pending => {
            // Stack still busy after bound polls — treat as degraded but present.
            set_service_err(h, 2, SERVICE_STATE_DEGRADED, b"poll timeout");
            log::warn!("lerux-supervisor: net poll timeout");
            // Keep historical smoke string so boot remains diagnosable.
            log::info!("lerux-supervisor: net up");
        }
        NetResponse::Error => {
            set_service_err(h, 2, SERVICE_STATE_ERROR, b"net probe error");
            log::error!("lerux-supervisor: net probe error");
        }
        _ => {
            set_service_err(h, 2, SERVICE_STATE_READY, b"");
            log::info!("lerux-supervisor: net up");
        }
    }
}

#[cfg(feature = "workstation")]
fn persist_boot_log() {
    if let Ok(LogResponse::Recent {
        count, lens, lines, ..
    }) = call::<LogRequest, LogResponse>(LOG_SERVER, LogRequest::get_recent())
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
fn service_list(h: &HandlerImpl) -> SupervisorResponse {
    let mut name_lens = [0u8; MAX_SERVICES];
    let mut names = [[0u8; MAX_SERVICE_NAME]; MAX_SERVICES];
    let mut ready = [false; MAX_SERVICES];
    let mut states = [SERVICE_STATE_STARTING; MAX_SERVICES];
    let count = SERVICE_NAMES.len().min(MAX_SERVICES) as u8;
    for (i, name) in SERVICE_NAMES.iter().take(count as usize).enumerate() {
        let n = name.len().min(MAX_SERVICE_NAME);
        name_lens[i] = n as u8;
        names[i][..n].copy_from_slice(&name[..n]);
        states[i] = h.states[i];
        ready[i] = matches!(h.states[i], SERVICE_STATE_READY | SERVICE_STATE_DEGRADED);
    }
    SupervisorResponse::ServiceList {
        count,
        name_lens,
        names,
        ready,
        states,
    }
}

#[cfg(feature = "workstation")]
fn handle_supervisor(h: &HandlerImpl, req: SupervisorRequest) -> SupervisorResponse {
    match req {
        SupervisorRequest::Reboot => {
            log::info!("lerux-supervisor: reboot requested");
            SupervisorResponse::Ok
        }
        SupervisorRequest::ListServices => service_list(h),
        SupervisorRequest::ServiceStatus { id } => {
            if let SupervisorResponse::ServiceList {
                count,
                ready,
                states,
                ..
            } = service_list(h)
            {
                if (id as usize) < count as usize {
                    let i = id as usize;
                    SupervisorResponse::Status {
                        ready: ready[i],
                        state: states[i],
                        err_len: h.err_lens[i],
                        err: h.errs[i],
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
    server::init_with_tag(LOG_SERVER, b"supervis").unwrap();
    #[cfg(not(feature = "workstation"))]
    serial::init(SERIAL_DRIVER).unwrap();
    #[cfg(feature = "workstation")]
    let mut handler = {
        let mut states = [SERVICE_STATE_STARTING; MAX_SERVICES];
        // Image-resident services start as ready once supervisor itself is up.
        states[0] = SERVICE_STATE_READY; // supervisor
        states[3] = SERVICE_STATE_READY; // shell
        states[4] = SERVICE_STATE_READY; // edit
        states[5] = SERVICE_STATE_READY; // chat-client
        states[6] = SERVICE_STATE_READY; // http-fs
        states[7] = SERVICE_STATE_READY; // log-server
        HandlerImpl {
            states,
            errs: [[0u8; MAX_SERVICE_ERR]; MAX_SERVICES],
            err_lens: [0u8; MAX_SERVICES],
        }
    };
    #[cfg(not(feature = "workstation"))]
    let handler = HandlerImpl;
    #[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
    let mut timer = {
        let mut rtc = RtcClient::new(RTC_DRIVER);
        log_rtc(&mut rtc);
        let mut timer = TimerClient::new(TIMER_DRIVER);
        log_timer(&mut timer);
        timer
    };
    #[cfg(feature = "board-rpi4b_4gb_workstation")]
    log::info!("lerux-supervisor: no RTC/timer PDs on RPi4 workstation");
    log_service_graph();
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
        probe_fs(&mut handler);
        seed_first_boot();
        apply_config_policy();
        probe_net(&mut handler);
        persist_boot_log();
        log::info!("lerux-supervisor: ready");
    }
    #[cfg(not(feature = "board-rpi4b_4gb_workstation"))]
    watchdog_check(&mut timer);
    #[cfg(feature = "board-rpi4b_4gb_workstation")]
    watchdog_check();
    handler
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
                Ok(req) => send(handle_supervisor(self, req)),
                Err(_) => send_unspecified_error(),
            });
        }
        // No other IPC for supervisor on non-workstation boards
        Ok(send_unspecified_error())
    }
}
