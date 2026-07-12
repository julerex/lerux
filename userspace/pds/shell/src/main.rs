#![no_std]
#![no_main]

use core::fmt::Write;

use embedded_hal_nb::{
    nb,
    serial::{Read as _, Write as _},
};
use lerux_interface_types::{
    ChatRequest, ChatResponse, ConfigRequest, ConfigResponse, EditRequest, EditResponse, FsRequest,
    FsResponse, LogRequest, LogResponse, NetRequest, NetResponse, SupervisorRequest,
    SupervisorResponse, CFG_HOSTNAME, CFG_SECRET_PREFIX, MAX_CHAT_MSG, MAX_SERVICE_NAME,
};
use lerux_ipc::call;
use lerux_logging::{log, server};
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};
use sel4_microkit_driver_adapters::serial::client::Client as SerialClient;

const SERIAL_DRIVER: Channel = Channel::new(0);
const FS_SERVER: Channel = Channel::new(1);
const NET_SERVER: Channel = Channel::new(2);
const SUPERVISOR: Channel = Channel::new(3);
const LOG_SERVER: Channel = Channel::new(4);
const CONFIG_SERVER: Channel = Channel::new(5);
const EDIT: Channel = Channel::new(6);
const CHAT: Channel = Channel::new(7);

const INPUT_BUF_CAP: usize = 128;
const CWD_CAP: usize = lerux_interface_types::MAX_FS_PATH;
/// Phase 53: lines per pager page for `cat` / `dmesg`.
const PAGE_LINES: usize = 16;
/// History ring capacity (Phase 53).
const HISTORY_CAP: usize = 8;
const HISTORY_LINE: usize = 64;

/// Machine-readable command list for smokes and `help -l` (Phase 53).
const COMMANDS: &[&str] = &[
    "ls", "cat", "write", "mkdir", "rm", "mv", "cd", "pwd", "stat", "df", "ip", "ifconfig", "ping",
    "time", "date", "uptime", "clear", "history", "ps", "top", "status", "qos", "reboot", "fetch",
    "dmesg", "edit", "chat", "echo", "config", "get", "set", "list", "hostname", "help",
];

struct HandlerImpl {
    console: SerialClient,
    input_buf: [u8; INPUT_BUF_CAP],
    input_len: usize,
    in_edit: bool,
    in_chat: bool,
    /// Shell-local cwd (Phase 50); server paths are absolute after resolve.
    cwd: [u8; CWD_CAP],
    cwd_len: u8,
    /// Circular command history (Phase 53).
    history: [[u8; HISTORY_LINE]; HISTORY_CAP],
    history_lens: [u8; HISTORY_CAP],
    history_len: u8,
    history_next: u8,
}

fn write_bytes(console: &mut SerialClient, bytes: &[u8]) {
    for &b in bytes {
        let _ = console.write(b);
    }
    let _ = console.flush();
}

fn print(console: &mut SerialClient, s: &str) {
    write_bytes(console, s.as_bytes());
}

fn println(console: &mut SerialClient, s: &str) {
    print(console, s);
    write_bytes(console, b"\r\n");
}

fn print_prompt(console: &mut SerialClient) {
    print(console, "lerux> ");
}

fn poll_fs() -> FsResponse {
    loop {
        match call::<FsRequest, FsResponse>(FS_SERVER, FsRequest::Poll) {
            Ok(FsResponse::Pending) => {}
            Ok(other) => return other,
            Err(_) => return FsResponse::Error,
        }
    }
}

fn fs_call(req: FsRequest) -> FsResponse {
    match call::<FsRequest, FsResponse>(FS_SERVER, req) {
        Ok(FsResponse::Pending) => poll_fs(),
        Ok(other) => other,
        Err(_) => FsResponse::Error,
    }
}

fn edit_call(req: EditRequest) -> EditResponse {
    match call::<EditRequest, EditResponse>(EDIT, req) {
        Ok(other) => other,
        Err(_) => EditResponse::Error,
    }
}

fn chat_call(req: ChatRequest) -> ChatResponse {
    match call::<ChatRequest, ChatResponse>(CHAT, req) {
        Ok(other) => other,
        Err(_) => ChatResponse::Error,
    }
}

fn edit_view_to_display(console: &mut SerialClient, v: &EditResponse) {
    if let EditResponse::View {
        path_len,
        path,
        line_count,
        line_lens,
        lines,
        cursor_row: _,
        cursor_col: _,
        modified,
    } = v
    {
        write_bytes(console, b"\r\n--- edit: ");
        let p = &path[..*path_len as usize];
        write_bytes(console, p);
        if *modified {
            write_bytes(console, b" *");
        }
        write_bytes(console, b" (Ctrl-S save  Ctrl-Q quit)\r\n");
        for i in 0..(*line_count as usize) {
            let l = line_lens[i] as usize;
            write_bytes(console, &lines[i][..l]);
            write_bytes(console, b"\r\n");
        }
    } else {
        println(console, "edit: view error");
    }
}

fn chat_view_to_display(console: &mut SerialClient, v: &ChatResponse) {
    if let ChatResponse::View {
        count,
        line_lens,
        lines,
    } = v
    {
        write_bytes(console, b"\r\n--- chat (Enter send  Ctrl-Q quit)\r\n");
        for i in 0..(*count as usize) {
            let l = line_lens[i] as usize;
            write_bytes(console, &lines[i][..l]);
            write_bytes(console, b"\r\n");
        }
        write_bytes(console, b"> ");
    } else {
        println(console, "chat: view error");
    }
}

/// Resolve `path` against shell cwd into `out`; returns length.
fn resolve_path(cwd: &[u8], path: &[u8], out: &mut [u8; CWD_CAP]) -> Option<usize> {
    if path.is_empty() {
        let n = cwd.len().min(out.len());
        out[..n].copy_from_slice(&cwd[..n]);
        return Some(n);
    }
    if path[0] == b'/' {
        let n = path.len().min(out.len());
        out[..n].copy_from_slice(&path[..n]);
        return Some(n);
    }
    // cwd + "/" + path
    let pos = if cwd == b"/" {
        out[0] = b'/';
        1usize
    } else {
        let n = cwd.len().min(out.len());
        out[..n].copy_from_slice(&cwd[..n]);
        let mut pos = n;
        if pos < out.len() {
            out[pos] = b'/';
            pos += 1;
        }
        pos
    };
    if pos + path.len() > out.len() {
        return None;
    }
    out[pos..pos + path.len()].copy_from_slice(path);
    Some(pos + path.len())
}

fn ls(console: &mut SerialClient, cwd: &[u8], arg: Option<&[u8]>) {
    let mut path_buf = [0u8; CWD_CAP];
    let path = match arg {
        Some(p) => match resolve_path(cwd, p, &mut path_buf) {
            Some(n) => &path_buf[..n],
            None => {
                println(console, "ls: path too long");
                return;
            }
        },
        None => cwd,
    };
    match fs_call(FsRequest::list_dir(path)) {
        FsResponse::DirList { count, entries } => {
            for e in entries.iter().take(count as usize) {
                let name = core::str::from_utf8(e.name_slice()).unwrap_or("?");
                let kind = if e.is_dir { "dir" } else { "file" };
                let _ = writeln!(
                    ConsoleWriter(console),
                    "{:<20} {:>6} {}",
                    name,
                    e.size,
                    kind
                );
            }
        }
        _ => println(console, "ls: error"),
    }
}

struct ConsoleWriter<'a>(&'a mut SerialClient);

impl Write for ConsoleWriter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write_bytes(self.0, s.as_bytes());
        Ok(())
    }
}

/// Wait for space/enter to continue or `q` to quit (Phase 53 pager).
fn wait_more(console: &mut SerialClient) -> bool {
    print(console, " -- more -- ");
    let _ = console.flush();
    loop {
        match console.read() {
            Ok(b' ') | Ok(b'\r') | Ok(b'\n') => {
                write_bytes(console, b"\r\n");
                return true;
            }
            Ok(b'q') | Ok(b'Q') => {
                write_bytes(console, b"\r\n");
                return false;
            }
            Ok(_) | Err(nb::Error::WouldBlock) => {}
            Err(nb::Error::Other(_)) => return false,
        }
    }
}

fn cat(console: &mut SerialClient, cwd: &[u8], path: &[u8]) {
    let mut path_buf = [0u8; CWD_CAP];
    let Some(n) = resolve_path(cwd, path, &mut path_buf) else {
        println(console, "cat: path too long");
        return;
    };
    let handle = match fs_call(FsRequest::open(&path_buf[..n])) {
        FsResponse::Handle { id } => id,
        _ => {
            println(console, "cat: open failed");
            return;
        }
    };
    let mut offset = 0u32;
    let mut lines_on_page = 0usize;
    loop {
        match fs_call(FsRequest::Read {
            handle,
            offset,
            len: 128,
        }) {
            FsResponse::Data { data_len, data } if data_len > 0 => {
                let chunk = &data[..data_len as usize];
                for &b in chunk {
                    let _ = console.write(b);
                    if b == b'\n' {
                        lines_on_page += 1;
                        if lines_on_page >= PAGE_LINES {
                            let _ = console.flush();
                            if !wait_more(console) {
                                return;
                            }
                            lines_on_page = 0;
                        }
                    }
                }
                offset += data_len as u32;
            }
            _ => break,
        }
    }
    write_bytes(console, b"\r\n");
}

fn write_file(console: &mut SerialClient, cwd: &[u8], path: &[u8], data: &[u8]) {
    let mut path_buf = [0u8; CWD_CAP];
    let Some(n) = resolve_path(cwd, path, &mut path_buf) else {
        println(console, "write: path too long");
        return;
    };
    let handle = match fs_call(FsRequest::create(&path_buf[..n])) {
        FsResponse::Handle { id } => id,
        _ => {
            println(console, "write: create failed");
            return;
        }
    };
    let mut offset = 0u32;
    let max_chunk = lerux_interface_types::MAX_FS_DATA;
    while (offset as usize) < data.len() {
        let end = (offset as usize + max_chunk).min(data.len());
        match fs_call(FsRequest::write(
            handle,
            offset,
            &data[offset as usize..end],
        )) {
            FsResponse::Ok => offset = end as u32,
            _ => {
                println(console, "write: failed");
                return;
            }
        }
    }
    println(console, "write: ok");
}

fn mkdir_cmd(console: &mut SerialClient, cwd: &[u8], path: &[u8]) {
    let mut path_buf = [0u8; CWD_CAP];
    let Some(n) = resolve_path(cwd, path, &mut path_buf) else {
        println(console, "mkdir: path too long");
        return;
    };
    match fs_call(FsRequest::mkdir(&path_buf[..n])) {
        FsResponse::Ok => println(console, "mkdir: ok"),
        _ => println(console, "mkdir: failed"),
    }
}

fn rm_cmd(console: &mut SerialClient, cwd: &[u8], path: &[u8]) {
    let mut path_buf = [0u8; CWD_CAP];
    let Some(n) = resolve_path(cwd, path, &mut path_buf) else {
        println(console, "rm: path too long");
        return;
    };
    match fs_call(FsRequest::unlink(&path_buf[..n])) {
        FsResponse::Ok => println(console, "rm: ok"),
        _ => println(console, "rm: failed"),
    }
}

fn mv_cmd(console: &mut SerialClient, cwd: &[u8], from: &[u8], to: &[u8]) {
    let mut from_buf = [0u8; CWD_CAP];
    let mut to_buf = [0u8; CWD_CAP];
    let Some(fn_) = resolve_path(cwd, from, &mut from_buf) else {
        println(console, "mv: path too long");
        return;
    };
    let Some(tn) = resolve_path(cwd, to, &mut to_buf) else {
        println(console, "mv: path too long");
        return;
    };
    match fs_call(FsRequest::rename(&from_buf[..fn_], &to_buf[..tn])) {
        FsResponse::Ok => println(console, "mv: ok"),
        _ => println(console, "mv: failed"),
    }
}

fn pwd_cmd(console: &mut SerialClient, cwd: &[u8]) {
    write_bytes(console, cwd);
    write_bytes(console, b"\r\n");
}

fn cd_cmd(h: &mut HandlerImpl, path: &[u8]) {
    let mut path_buf = [0u8; CWD_CAP];
    let cwd = &h.cwd[..h.cwd_len as usize];
    let Some(n) = resolve_path(cwd, path, &mut path_buf) else {
        println(&mut h.console, "cd: path too long");
        return;
    };
    // Allow "/" always; otherwise require Stat is_dir.
    if n == 1 && path_buf[0] == b'/' {
        h.cwd[0] = b'/';
        h.cwd_len = 1;
        return;
    }
    match fs_call(FsRequest::stat(&path_buf[..n])) {
        FsResponse::Stat { is_dir: true, .. } => {
            h.cwd[..n].copy_from_slice(&path_buf[..n]);
            h.cwd_len = n as u8;
        }
        _ => println(&mut h.console, "cd: failed"),
    }
}

fn time(console: &mut SerialClient) {
    match call::<SupervisorRequest, SupervisorResponse>(SUPERVISOR, SupervisorRequest::GetTime) {
        Ok(SupervisorResponse::Time { year, month, day }) => {
            let _ = writeln!(
                ConsoleWriter(console),
                "{:04}-{:02}-{:02}",
                year,
                month,
                day
            );
        }
        _ => println(console, "time: error"),
    }
}

fn stat_cmd(console: &mut SerialClient, cwd: &[u8], path: &[u8]) {
    let mut path_buf = [0u8; CWD_CAP];
    let Some(n) = resolve_path(cwd, path, &mut path_buf) else {
        println(console, "stat: path too long");
        return;
    };
    match fs_call(FsRequest::stat(&path_buf[..n])) {
        FsResponse::Stat { size, is_dir } => {
            let kind = if is_dir { "dir" } else { "file" };
            let name = core::str::from_utf8(&path_buf[..n]).unwrap_or("?");
            let _ = writeln!(ConsoleWriter(console), "{}: {} size={}", name, kind, size);
        }
        _ => println(console, "stat: failed"),
    }
}

fn df_cmd(console: &mut SerialClient) {
    // DiskInfo may return Ok after first-boot format; retry once.
    let mut resp = fs_call(FsRequest::DiskInfo);
    if matches!(resp, FsResponse::Ok | FsResponse::Pending) {
        resp = fs_call(FsRequest::DiskInfo);
    }
    match resp {
        FsResponse::DiskInfo {
            block_size,
            total_blocks,
            free_blocks,
        } => {
            let used = total_blocks.saturating_sub(free_blocks);
            let _ = writeln!(
                ConsoleWriter(console),
                "Filesystem  1K-blocks  Used  Available",
            );
            let total_k = (total_blocks as u64 * block_size as u64) / 1024;
            let used_k = (used as u64 * block_size as u64) / 1024;
            let free_k = (free_blocks as u64 * block_size as u64) / 1024;
            let _ = writeln!(
                ConsoleWriter(console),
                "leruxfs     {:>9}  {:>4}  {:>9}",
                total_k,
                used_k,
                free_k
            );
        }
        _ => println(console, "df: unavailable"),
    }
}

fn ping_cmd(console: &mut SerialClient) {
    // UDP reachability probe to default gateway (same path as fetch demo).
    for _ in 0..16 {
        match call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::udp_tx(b"ping")) {
            Ok(NetResponse::Pending) => {
                if poll_net() == NetResponse::Ok {
                    println(console, "ping: udp ok");
                    return;
                }
            }
            Ok(NetResponse::Ok) => {
                println(console, "ping: udp ok");
                return;
            }
            _ => break,
        }
    }
    println(console, "ping: failed");
}

fn uptime_cmd(console: &mut SerialClient) {
    match call::<SupervisorRequest, SupervisorResponse>(SUPERVISOR, SupervisorRequest::GetUptime) {
        Ok(SupervisorResponse::Uptime { secs }) => {
            let h = secs / 3600;
            let m = (secs % 3600) / 60;
            let s = secs % 60;
            let _ = writeln!(ConsoleWriter(console), "up {:02}:{:02}:{:02}", h, m, s);
        }
        _ => println(console, "uptime: unavailable"),
    }
}

fn clear_cmd(console: &mut SerialClient) {
    // ANSI clear screen + home (works on most serial terminals / screen).
    write_bytes(console, b"\x1b[2J\x1b[H");
}

fn history_cmd(h: &mut HandlerImpl) {
    let n = h.history_len as usize;
    if n == 0 {
        println(&mut h.console, "history: empty");
        return;
    }
    let start = if h.history_len < HISTORY_CAP as u8 {
        0
    } else {
        h.history_next as usize
    };
    for i in 0..n {
        let idx = (start + i) % HISTORY_CAP;
        let len = h.history_lens[idx] as usize;
        let _ = write!(ConsoleWriter(&mut h.console), "{:>3}  ", i + 1);
        write_bytes(&mut h.console, &h.history[idx][..len]);
        write_bytes(&mut h.console, b"\r\n");
    }
}

fn push_history(h: &mut HandlerImpl, line: &[u8]) {
    if line.is_empty() {
        return;
    }
    let i = h.history_next as usize;
    let n = line.len().min(HISTORY_LINE);
    h.history[i][..n].copy_from_slice(&line[..n]);
    h.history_lens[i] = n as u8;
    h.history_next = ((i + 1) % HISTORY_CAP) as u8;
    if (h.history_len as usize) < HISTORY_CAP {
        h.history_len += 1;
    }
}

fn help_cmd(console: &mut SerialClient, arg: Option<&[u8]>) {
    match arg {
        Some(b"-l") | Some(b"commands") | Some(b"--list") => {
            print(console, "lerux-shell: cmds=");
            for (i, c) in COMMANDS.iter().enumerate() {
                if i > 0 {
                    write_bytes(console, b",");
                }
                print(console, c);
            }
            write_bytes(console, b"\r\n");
        }
        Some(b"help") | None => {
            println(
                console,
                "lerux shell — type a command, or `help -l` for the full list",
            );
            println(console, "files:  ls cat write mkdir rm mv cd pwd stat df");
            println(console, "net:    ip ifconfig ping fetch");
            println(
                console,
                "sys:    time date uptime ps top status qos reboot dmesg clear history",
            );
            println(console, "config: config get|set|list|del  hostname");
            println(console, "apps:   edit chat echo help");
        }
        Some(other) => {
            print(console, "help: unknown topic ");
            write_bytes(console, other);
            write_bytes(console, b"\r\n");
        }
    }
}

fn config_call(req: ConfigRequest) -> ConfigResponse {
    match call::<ConfigRequest, ConfigResponse>(CONFIG_SERVER, req) {
        Ok(r) => r,
        Err(_) => ConfigResponse::Error,
    }
}

fn config_list_cmd(console: &mut SerialClient) {
    match config_call(ConfigRequest::List) {
        ConfigResponse::Keys { count, keys, lens } => {
            if count == 0 {
                println(console, "(empty)");
                return;
            }
            for i in 0..(count as usize) {
                let n = lens[i] as usize;
                let k = &keys[i][..n];
                write_bytes(console, k);
                if k.starts_with(CFG_SECRET_PREFIX) {
                    write_bytes(console, b" = <secret>");
                } else if let ConfigResponse::Value { val_len, value } =
                    config_call(ConfigRequest::get(k))
                {
                    write_bytes(console, b" = ");
                    write_bytes(console, &value[..val_len as usize]);
                }
                write_bytes(console, b"\r\n");
            }
        }
        _ => println(console, "config list: error"),
    }
}

fn config_get_cmd(console: &mut SerialClient, key: &[u8]) {
    match config_call(ConfigRequest::get(key)) {
        ConfigResponse::Value { val_len, value } => {
            write_bytes(console, &value[..val_len as usize]);
            write_bytes(console, b"\r\n");
        }
        _ => println(console, "config get: not found"),
    }
}

fn config_set_cmd(console: &mut SerialClient, key: &[u8], value: &[u8]) {
    match config_call(ConfigRequest::set(key, value)) {
        ConfigResponse::Ok => println(console, "config set: ok"),
        _ => println(console, "config set: failed"),
    }
}

fn config_del_cmd(console: &mut SerialClient, key: &[u8]) {
    match config_call(ConfigRequest::delete(key)) {
        ConfigResponse::Ok => println(console, "config del: ok"),
        _ => println(console, "config del: failed"),
    }
}

fn hostname_cmd(console: &mut SerialClient) {
    match config_call(ConfigRequest::get(CFG_HOSTNAME)) {
        ConfigResponse::Value { val_len, value } => {
            write_bytes(console, &value[..val_len as usize]);
            write_bytes(console, b"\r\n");
        }
        _ => println(console, "lerux"),
    }
}

fn state_name(state: u8) -> &'static str {
    match state {
        lerux_interface_types::SERVICE_STATE_READY => "ready",
        lerux_interface_types::SERVICE_STATE_STARTING => "start",
        lerux_interface_types::SERVICE_STATE_DEGRADED => "degraded",
        lerux_interface_types::SERVICE_STATE_ERROR => "error",
        _ => "?",
    }
}

fn render_services(console: &mut SerialClient) {
    match call::<SupervisorRequest, SupervisorResponse>(SUPERVISOR, SupervisorRequest::ListServices)
    {
        Ok(SupervisorResponse::ServiceList {
            count,
            name_lens,
            names,
            ready,
            states,
        }) => {
            println(console, "PID  READY  STATE     NAME");
            for i in 0..(count as usize) {
                let n = name_lens[i] as usize;
                let name =
                    core::str::from_utf8(&names[i][..n.min(MAX_SERVICE_NAME)]).unwrap_or("?");
                let flag = if ready[i] { "yes" } else { "no" };
                let _ = writeln!(
                    ConsoleWriter(console),
                    "{:>3}  {:<5}  {:<8}  {}",
                    i,
                    flag,
                    state_name(states[i]),
                    name
                );
            }
        }
        Ok(SupervisorResponse::Services { count }) => {
            let _ = writeln!(ConsoleWriter(console), "services: {count}");
        }
        Ok(
            SupervisorResponse::Ok
            | SupervisorResponse::Error
            | SupervisorResponse::Status { .. }
            | SupervisorResponse::Time { .. }
            | SupervisorResponse::Uptime { .. },
        )
        | Err(_) => println(console, "ps: error"),
    }
}

fn status_cmd(console: &mut SerialClient, id_arg: Option<&[u8]>) {
    let Some(raw) = id_arg else {
        println(console, "usage: status <id>");
        return;
    };
    let mut id: u8 = 0;
    for &b in raw {
        if !b.is_ascii_digit() {
            println(console, "status: bad id");
            return;
        }
        id = id.saturating_mul(10).saturating_add(b - b'0');
    }
    match call::<SupervisorRequest, SupervisorResponse>(
        SUPERVISOR,
        SupervisorRequest::ServiceStatus { id },
    ) {
        Ok(SupervisorResponse::Status {
            ready,
            state,
            err_len,
            err,
        }) => {
            let flag = if ready { "yes" } else { "no" };
            let _ = writeln!(
                ConsoleWriter(console),
                "id={} ready={} state={}",
                id,
                flag,
                state_name(state)
            );
            if err_len > 0 {
                write_bytes(console, b"error: ");
                write_bytes(console, &err[..err_len as usize]);
                write_bytes(console, b"\r\n");
            }
        }
        _ => println(console, "status: error"),
    }
}

fn ps(console: &mut SerialClient) {
    render_services(console);
}

fn top(console: &mut SerialClient) {
    println(console, "--- top ---");
    render_services(console);
}

/// Phase 48: print fixed priority service classes (matches workstation templates).
fn qos(console: &mut SerialClient) {
    println(console, "--- qos (Phase 48) ---");
    println(console, "class        band   examples");
    println(
        console,
        "platform     10-6   serial, virtio/genet/emmc, timers",
    );
    println(console, "services     5-4    log, fs, net");
    println(console, "control      3-2    config, supervisor");
    println(console, "bulk         2      edit, chat, http-fs");
    println(console, "interactive  1      shell (below all PPC servers)");
    println(
        console,
        "note: Microkit PPC requires callee priority > caller",
    );
    println(console, "policy: docs/qos.md");
}

fn reboot(console: &mut SerialClient) {
    let _ = call::<SupervisorRequest, SupervisorResponse>(SUPERVISOR, SupervisorRequest::Reboot);
    println(console, "reboot requested");
}

fn fetch_demo(console: &mut SerialClient) {
    for _ in 0..16 {
        match call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::udp_tx(b"lerux-fetch")) {
            Ok(NetResponse::Pending) => {
                if poll_net() == NetResponse::Ok {
                    println(console, "fetch: demo udp sent");
                    return;
                }
            }
            Ok(NetResponse::Ok) => {
                println(console, "fetch: demo udp sent");
                return;
            }
            _ => break,
        }
    }
    println(console, "fetch: error");
}

fn ip_cmd(console: &mut SerialClient) {
    match call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::GetIface) {
        Ok(NetResponse::Iface {
            addr,
            prefix,
            gateway,
            dns,
            dhcp,
        }) => {
            let mode = if dhcp { "dhcp" } else { "static" };
            let _ = writeln!(
                ConsoleWriter(console),
                "inet {}.{}.{}.{}/{} via {}.{}.{}.{} dns {}.{}.{}.{} ({})",
                addr[0],
                addr[1],
                addr[2],
                addr[3],
                prefix,
                gateway[0],
                gateway[1],
                gateway[2],
                gateway[3],
                dns[0],
                dns[1],
                dns[2],
                dns[3],
                mode
            );
        }
        _ => println(console, "ip: unavailable"),
    }
}

fn poll_net() -> NetResponse {
    loop {
        match call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::Poll) {
            Ok(NetResponse::Pending) => {}
            Ok(other) => return other,
            Err(_) => return NetResponse::Error,
        }
    }
}

fn parse_log_level(s: &[u8]) -> u8 {
    match s {
        b"error" | b"E" | b"e" => lerux_interface_types::LOG_LEVEL_ERROR,
        b"warn" | b"W" | b"w" => lerux_interface_types::LOG_LEVEL_WARN,
        b"info" | b"I" | b"i" => lerux_interface_types::LOG_LEVEL_INFO,
        b"debug" | b"D" | b"d" => lerux_interface_types::LOG_LEVEL_DEBUG,
        _ => 0,
    }
}

/// Phase 57: `dmesg`, `dmesg --pd shell`, `dmesg -l warn`.
fn dmesg_bytes<'a>(console: &mut SerialClient, args: impl Iterator<Item = &'a [u8]>) {
    let mut min_level: u8 = 0;
    let mut tag: &[u8] = b"";
    let mut args = args.peekable();
    while let Some(a) = args.next() {
        if a == b"--pd" || a == b"-p" {
            if let Some(t) = args.next() {
                tag = t;
            } else {
                println(console, "usage: dmesg [--pd TAG] [-l LEVEL]");
                return;
            }
        } else if a == b"-l" || a == b"--level" {
            if let Some(lv) = args.next() {
                min_level = parse_log_level(lv);
                if min_level == 0 && lv != b"0" {
                    println(console, "dmesg: bad level (error|warn|info|debug)");
                    return;
                }
            } else {
                println(console, "usage: dmesg [--pd TAG] [-l LEVEL]");
                return;
            }
        } else if let Some(rest) = a.strip_prefix(b"--pd=") {
            tag = rest;
        } else {
            println(console, "usage: dmesg [--pd TAG] [-l LEVEL]");
            return;
        }
    }
    let req = if tag.is_empty() && min_level == 0 {
        LogRequest::get_recent()
    } else {
        LogRequest::get_filtered(min_level, tag)
    };
    match call::<LogRequest, LogResponse>(LOG_SERVER, req) {
        Ok(LogResponse::Recent {
            count,
            lens,
            lines,
            levels,
            tag_lens,
            tags,
        }) => {
            let mut lines_on_page = 0usize;
            for i in 0..(count as usize) {
                let l = lens[i] as usize;
                if l == 0 {
                    continue;
                }
                let lc = match levels[i] {
                    lerux_interface_types::LOG_LEVEL_ERROR => b'E',
                    lerux_interface_types::LOG_LEVEL_WARN => b'W',
                    lerux_interface_types::LOG_LEVEL_INFO => b'I',
                    lerux_interface_types::LOG_LEVEL_DEBUG => b'D',
                    _ => b'?',
                };
                write_bytes(console, &[lc, b'[']);
                let tn = tag_lens[i] as usize;
                if tn > 0 {
                    write_bytes(console, &tags[i][..tn]);
                }
                write_bytes(console, b"] ");
                write_bytes(console, &lines[i][..l]);
                write_bytes(console, b"\r\n");
                lines_on_page += 1;
                if lines_on_page >= PAGE_LINES {
                    if !wait_more(console) {
                        return;
                    }
                    lines_on_page = 0;
                }
            }
        }
        _ => println(console, "dmesg: unavailable"),
    }
}

fn process_command(h: &mut HandlerImpl, line: &[u8]) {
    let line = if let Some(p) = line.iter().position(|&b| b == b'\r' || b == b'\n') {
        &line[..p]
    } else {
        line
    };
    if line.is_empty() {
        return;
    }
    push_history(h, line);
    let mut parts = line.split(|&b| b == b' ');
    let cmd = parts.next().unwrap_or(b"");
    // Copy cwd so path helpers do not borrow `h` across `cd` / `history`.
    let mut cwd_copy = [0u8; CWD_CAP];
    let cwd_len = h.cwd_len as usize;
    cwd_copy[..cwd_len].copy_from_slice(&h.cwd[..cwd_len]);
    let cwd = &cwd_copy[..cwd_len];
    match cmd {
        b"ls" => ls(&mut h.console, cwd, parts.next()),
        b"cat" => {
            if let Some(p) = parts.next() {
                cat(&mut h.console, cwd, p);
            } else {
                println(&mut h.console, "usage: cat <path>");
            }
        }
        b"write" => {
            let mut it = line.split(|&b| b == b' ');
            let _ = it.next();
            if let Some(path) = it.next() {
                let path_pos = line
                    .windows(path.len())
                    .position(|w| w == path)
                    .unwrap_or(0);
                let data_start = path_pos + path.len() + 1;
                let data = if data_start < line.len() {
                    &line[data_start..]
                } else {
                    b""
                };
                write_file(&mut h.console, cwd, path, data);
            } else {
                println(&mut h.console, "usage: write <path> <data>");
            }
        }
        b"mkdir" => {
            if let Some(p) = parts.next() {
                mkdir_cmd(&mut h.console, cwd, p);
            } else {
                println(&mut h.console, "usage: mkdir <path>");
            }
        }
        b"rm" => {
            if let Some(p) = parts.next() {
                rm_cmd(&mut h.console, cwd, p);
            } else {
                println(&mut h.console, "usage: rm <path>");
            }
        }
        b"mv" => {
            if let (Some(from), Some(to)) = (parts.next(), parts.next()) {
                mv_cmd(&mut h.console, cwd, from, to);
            } else {
                println(&mut h.console, "usage: mv <from> <to>");
            }
        }
        b"cd" => {
            if let Some(p) = parts.next() {
                cd_cmd(h, p);
            } else {
                println(&mut h.console, "usage: cd <path>");
            }
        }
        b"pwd" => pwd_cmd(&mut h.console, cwd),
        b"stat" => {
            if let Some(p) = parts.next() {
                stat_cmd(&mut h.console, cwd, p);
            } else {
                println(&mut h.console, "usage: stat <path>");
            }
        }
        b"df" => df_cmd(&mut h.console),
        b"time" | b"date" => time(&mut h.console),
        b"uptime" => uptime_cmd(&mut h.console),
        b"clear" => clear_cmd(&mut h.console),
        b"history" => history_cmd(h),
        b"ps" => ps(&mut h.console),
        b"top" => top(&mut h.console),
        b"status" => status_cmd(&mut h.console, parts.next()),
        b"qos" => qos(&mut h.console),
        b"reboot" => reboot(&mut h.console),
        b"fetch" => fetch_demo(&mut h.console),
        b"ping" => ping_cmd(&mut h.console),
        b"ip" | b"ifconfig" => ip_cmd(&mut h.console),
        b"hostname" => hostname_cmd(&mut h.console),
        b"config" => match parts.next() {
            Some(b"list") | None => config_list_cmd(&mut h.console),
            Some(b"get") => {
                if let Some(k) = parts.next() {
                    config_get_cmd(&mut h.console, k);
                } else {
                    println(&mut h.console, "usage: config get <key>");
                }
            }
            Some(b"set") => {
                if let Some(k) = parts.next() {
                    // remainder of line after "config set key "
                    let rest = line
                        .split(|&b| b == b' ')
                        .nth(3)
                        .map(|_| {
                            // find third space after "config set key"
                            let mut spaces = 0usize;
                            let mut idx = 0usize;
                            for (i, &b) in line.iter().enumerate() {
                                if b == b' ' {
                                    spaces += 1;
                                    if spaces == 3 {
                                        idx = i + 1;
                                        break;
                                    }
                                }
                            }
                            if spaces >= 3 {
                                &line[idx..]
                            } else {
                                b""
                            }
                        })
                        .unwrap_or(b"");
                    if rest.is_empty() {
                        println(&mut h.console, "usage: config set <key> <value>");
                    } else {
                        config_set_cmd(&mut h.console, k, rest);
                    }
                } else {
                    println(&mut h.console, "usage: config set <key> <value>");
                }
            }
            Some(b"del") | Some(b"delete") | Some(b"rm") => {
                if let Some(k) = parts.next() {
                    config_del_cmd(&mut h.console, k);
                } else {
                    println(&mut h.console, "usage: config del <key>");
                }
            }
            Some(_) => println(
                &mut h.console,
                "usage: config list|get|set|del …  (docs/config.md)",
            ),
        },
        b"list" => config_list_cmd(&mut h.console),
        b"get" => {
            if let Some(k) = parts.next() {
                config_get_cmd(&mut h.console, k);
            } else {
                println(&mut h.console, "usage: get <key>");
            }
        }
        b"set" => {
            if let Some(k) = parts.next() {
                let mut spaces = 0usize;
                let mut idx = 0usize;
                for (i, &b) in line.iter().enumerate() {
                    if b == b' ' {
                        spaces += 1;
                        if spaces == 2 {
                            idx = i + 1;
                            break;
                        }
                    }
                }
                let rest = if spaces >= 2 { &line[idx..] } else { b"" };
                if rest.is_empty() {
                    println(&mut h.console, "usage: set <key> <value>");
                } else {
                    config_set_cmd(&mut h.console, k, rest);
                }
            } else {
                println(&mut h.console, "usage: set <key> <value>");
            }
        }
        b"help" => help_cmd(&mut h.console, parts.next()),
        b"echo" => {
            let rest = if line.len() > 4 { &line[4..] } else { b"" };
            let rest = rest.strip_prefix(b" ").unwrap_or(rest);
            write_bytes(&mut h.console, rest);
            write_bytes(&mut h.console, b"\r\n");
        }
        b"dmesg" => dmesg_bytes(&mut h.console, parts),
        b"edit" => {
            if let Some(p) = parts.next() {
                let mut abs = [0u8; lerux_interface_types::MAX_FS_PATH];
                let Some(n) = resolve_path(cwd, p, &mut abs) else {
                    println(&mut h.console, "edit: path too long");
                    return;
                };
                let mut pb = [0u8; lerux_interface_types::MAX_FS_PATH];
                pb[..n].copy_from_slice(&abs[..n]);
                match edit_call(EditRequest::Open {
                    path_len: n as u8,
                    path: pb,
                }) {
                    EditResponse::View { .. } => {
                        h.in_edit = true;
                        if let r @ EditResponse::View { .. } = edit_call(EditRequest::GetView) {
                            edit_view_to_display(&mut h.console, &r);
                        }
                    }
                    _ => println(&mut h.console, "edit: open failed"),
                }
            } else {
                println(&mut h.console, "usage: edit <path>");
            }
        }
        b"chat" => {
            h.in_chat = true;
            let _ = chat_call(ChatRequest::Recv);
            if let r @ ChatResponse::View { .. } = chat_call(ChatRequest::GetView) {
                chat_view_to_display(&mut h.console, &r);
            }
        }
        _ => {
            print(&mut h.console, "unknown command: ");
            write_bytes(&mut h.console, cmd);
            println(&mut h.console, " (help)");
        }
    }
}

#[protection_domain]
fn init() -> HandlerImpl {
    server::init_with_tag(LOG_SERVER, b"shell").unwrap();
    let _console = SerialClient::new(SERIAL_DRIVER);
    log::info!("lerux-shell: ready");
    // Machine-readable command discovery for smokes (Phase 53).
    // Keep under MAX_LOG_MSG (~80) by logging a short marker + count.
    log::info!("lerux-shell: cmds={} (help -l)", COMMANDS.len());

    if let FsResponse::DirList { count, .. } = fs_call(FsRequest::list_root()) {
        log::info!("lerux-shell: ls count={}", count);
    }
    if let Ok(SupervisorResponse::Time { year, month, day }) =
        call::<SupervisorRequest, SupervisorResponse>(SUPERVISOR, SupervisorRequest::GetTime)
    {
        log::info!("lerux-shell: time {}-{:02}-{:02}", year, month, day);
    }
    // Exercise top/ps service list for smoke.
    if let Ok(SupervisorResponse::ServiceList { count, .. }) =
        call::<SupervisorRequest, SupervisorResponse>(SUPERVISOR, SupervisorRequest::ListServices)
    {
        log::info!("lerux-shell: top count={}", count);
    }

    let mut c = SerialClient::new(SERIAL_DRIVER);
    print_prompt(&mut c);

    let mut cwd = [0u8; CWD_CAP];
    cwd[0] = b'/';
    HandlerImpl {
        console: SerialClient::new(SERIAL_DRIVER),
        input_buf: [0; INPUT_BUF_CAP],
        input_len: 0,
        in_edit: false,
        in_chat: false,
        cwd,
        cwd_len: 1,
        history: [[0; HISTORY_LINE]; HISTORY_CAP],
        history_lens: [0; HISTORY_CAP],
        history_len: 0,
        history_next: 0,
    }
}

impl HandlerImpl {
    fn handle_byte(&mut self, b: u8) {
        if self.in_chat {
            if b == 0x11 {
                let _ = chat_call(ChatRequest::Quit);
                self.in_chat = false;
                self.input_len = 0;
                write_bytes(&mut self.console, b"\r\n[quit chat]\r\n");
                print_prompt(&mut self.console);
                return;
            }
            if b == b'\r' || b == b'\n' {
                write_bytes(&mut self.console, b"\r\n");
                let mut msg = [0u8; MAX_CHAT_MSG];
                let n = self.input_len.min(MAX_CHAT_MSG);
                msg[..n].copy_from_slice(&self.input_buf[..n]);
                self.input_len = 0;
                if n > 0 {
                    let _ = chat_call(ChatRequest::Send {
                        msg_len: n as u8,
                        msg,
                    });
                }
                let _ = chat_call(ChatRequest::Recv);
                if let r @ ChatResponse::View { .. } = chat_call(ChatRequest::GetView) {
                    chat_view_to_display(&mut self.console, &r);
                }
                return;
            }
            if b == 0x08 || b == 0x7f {
                if self.input_len > 0 {
                    self.input_len -= 1;
                    write_bytes(&mut self.console, b"\x08 \x08");
                }
                return;
            }
            if (32..127).contains(&b) && self.input_len < INPUT_BUF_CAP {
                self.input_buf[self.input_len] = b;
                self.input_len += 1;
                write_bytes(&mut self.console, &[b]);
            }
            return;
        }
        if b == b'\r' || b == b'\n' {
            if self.in_edit {
                write_bytes(&mut self.console, b"\r\n");
                let _ = edit_call(EditRequest::Newline);
                if let r @ EditResponse::View { .. } = edit_call(EditRequest::GetView) {
                    edit_view_to_display(&mut self.console, &r);
                }
                self.input_len = 0;
                return;
            }
            write_bytes(&mut self.console, b"\r\n");
            let mut line = [0u8; INPUT_BUF_CAP];
            let n = self.input_len;
            line[..n].copy_from_slice(&self.input_buf[..n]);
            self.input_len = 0;
            process_command(self, &line[..n]);
            if !self.in_edit && !self.in_chat {
                print_prompt(&mut self.console);
            }
            return;
        }
        if b == 0x08 || b == 0x7f {
            if self.in_edit {
                let _ = edit_call(EditRequest::Backspace);
                if let r @ EditResponse::View { .. } = edit_call(EditRequest::GetView) {
                    edit_view_to_display(&mut self.console, &r);
                }
                return;
            }
            if self.input_len > 0 {
                self.input_len -= 1;
                write_bytes(&mut self.console, b"\x08 \x08");
            }
            return;
        }
        if self.in_edit {
            if b == 0x13 {
                match edit_call(EditRequest::Save) {
                    EditResponse::Ok => {
                        write_bytes(&mut self.console, b"\r\n[saved]\r\n");
                    }
                    _ => {
                        println(&mut self.console, "save failed");
                    }
                }
                if let r @ EditResponse::View { .. } = edit_call(EditRequest::GetView) {
                    edit_view_to_display(&mut self.console, &r);
                }
                return;
            }
            if b == 0x11 {
                let _ = edit_call(EditRequest::Quit);
                self.in_edit = false;
                write_bytes(&mut self.console, b"\r\n[quit edit]\r\n");
                print_prompt(&mut self.console);
                return;
            }
            if b == 0x1b {
                return;
            }
            if (32..127).contains(&b) {
                let _ = edit_call(EditRequest::InsertChar(b));
                if let r @ EditResponse::View { .. } = edit_call(EditRequest::GetView) {
                    edit_view_to_display(&mut self.console, &r);
                }
                return;
            }
            return;
        }
        if self.input_len < INPUT_BUF_CAP {
            self.input_buf[self.input_len] = b;
            self.input_len += 1;
            write_bytes(&mut self.console, &[b]);
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(SERIAL_DRIVER) {
            loop {
                match self.console.read() {
                    Ok(b) => self.handle_byte(b),
                    Err(nb::Error::WouldBlock) => break,
                    Err(_) => break,
                }
            }
        }
        Ok(())
    }
}
