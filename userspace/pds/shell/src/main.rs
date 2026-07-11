#![no_std]
#![no_main]

use core::fmt::Write;

use embedded_hal_nb::{
    nb,
    serial::{Read as _, Write as _},
};
use lerux_interface_types::{
    ChatRequest, ChatResponse, EditRequest, EditResponse, FsRequest, FsResponse, LogRequest,
    LogResponse, NetRequest, NetResponse, SupervisorRequest, SupervisorResponse, MAX_CHAT_MSG,
    MAX_SERVICE_NAME,
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
const EDIT: Channel = Channel::new(6);
const CHAT: Channel = Channel::new(7);

const INPUT_BUF_CAP: usize = 128;

struct HandlerImpl {
    console: SerialClient,
    input_buf: [u8; INPUT_BUF_CAP],
    input_len: usize,
    in_edit: bool,
    in_chat: bool,
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

fn ls(console: &mut SerialClient) {
    match fs_call(FsRequest::ListDir) {
        FsResponse::DirList { count, entries } => {
            for e in entries.iter().take(count as usize) {
                let name = core::str::from_utf8(e.name_slice()).unwrap_or("?");
                let _ = writeln!(ConsoleWriter(console), "{:<20} {}", name, e.size);
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

fn cat(console: &mut SerialClient, path: &[u8]) {
    let handle = match fs_call(FsRequest::open(path)) {
        FsResponse::Handle { id } => id,
        _ => {
            println(console, "cat: open failed");
            return;
        }
    };
    let mut offset = 0u32;
    loop {
        match fs_call(FsRequest::Read {
            handle,
            offset,
            len: 128,
        }) {
            FsResponse::Data { data_len, data } if data_len > 0 => {
                write_bytes(console, &data[..data_len as usize]);
                offset += data_len as u32;
            }
            _ => break,
        }
    }
    write_bytes(console, b"\r\n");
}

fn write_file(console: &mut SerialClient, path: &[u8], data: &[u8]) {
    let handle = match fs_call(FsRequest::create(path)) {
        FsResponse::Handle { id } => id,
        _ => {
            println(console, "write: create failed");
            return;
        }
    };
    match fs_call(FsRequest::write(handle, 0, data)) {
        FsResponse::Ok => println(console, "write: ok"),
        _ => println(console, "write: failed"),
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

fn render_services(console: &mut SerialClient) {
    match call::<SupervisorRequest, SupervisorResponse>(SUPERVISOR, SupervisorRequest::ListServices)
    {
        Ok(SupervisorResponse::ServiceList {
            count,
            name_lens,
            names,
            ready,
        }) => {
            println(console, "PID  READY  NAME");
            for i in 0..(count as usize) {
                let n = name_lens[i] as usize;
                let name =
                    core::str::from_utf8(&names[i][..n.min(MAX_SERVICE_NAME)]).unwrap_or("?");
                let flag = if ready[i] { "yes" } else { "no" };
                let _ = writeln!(ConsoleWriter(console), "{:>3}  {:<5}  {}", i, flag, name);
            }
        }
        Ok(SupervisorResponse::Services { count }) => {
            let _ = writeln!(ConsoleWriter(console), "services: {count}");
        }
        _ => println(console, "ps: error"),
    }
}

fn ps(console: &mut SerialClient) {
    render_services(console);
}

fn top(console: &mut SerialClient) {
    println(console, "--- top ---");
    render_services(console);
}

fn reboot(console: &mut SerialClient) {
    let _ = call::<SupervisorRequest, SupervisorResponse>(SUPERVISOR, SupervisorRequest::Reboot);
    println(console, "reboot requested");
}

fn fetch_demo(console: &mut SerialClient) {
    let pending = call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::udp_tx(b"lerux-fetch"));
    if matches!(pending, Ok(NetResponse::Pending)) {
        let _ = poll_net();
        println(console, "fetch: demo udp sent");
    } else {
        println(console, "fetch: error");
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

fn dmesg(console: &mut SerialClient) {
    match call::<LogRequest, LogResponse>(LOG_SERVER, LogRequest::GetRecent) {
        Ok(LogResponse::Recent { count, lens, lines }) => {
            for i in 0..(count as usize) {
                let l = lens[i] as usize;
                if l > 0 {
                    write_bytes(console, &lines[i][..l]);
                    write_bytes(console, b"\r\n");
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
    let mut parts = line.split(|&b| b == b' ');
    let cmd = parts.next().unwrap_or(b"");
    match cmd {
        b"ls" => ls(&mut h.console),
        b"cat" => {
            if let Some(p) = parts.next() {
                cat(&mut h.console, p);
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
                write_file(&mut h.console, path, data);
            } else {
                println(&mut h.console, "usage: write <path> <data>");
            }
        }
        b"time" | b"date" => time(&mut h.console),
        b"ps" => ps(&mut h.console),
        b"top" => top(&mut h.console),
        b"reboot" => reboot(&mut h.console),
        b"fetch" => fetch_demo(&mut h.console),
        b"help" => println(
            &mut h.console,
            "commands: ls cat write time ps top reboot fetch dmesg edit chat help",
        ),
        b"echo" => {
            let rest = &line[4..];
            write_bytes(&mut h.console, rest);
            write_bytes(&mut h.console, b"\r\n");
        }
        b"dmesg" => dmesg(&mut h.console),
        b"edit" => {
            if let Some(p) = parts.next() {
                let mut pb = [0u8; lerux_interface_types::MAX_FS_PATH];
                let n = p.len().min(pb.len());
                pb[..n].copy_from_slice(&p[..n]);
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
    server::init(LOG_SERVER).unwrap();
    let _console = SerialClient::new(SERIAL_DRIVER);
    log::info!("lerux-shell: ready");

    if let FsResponse::DirList { count, .. } = fs_call(FsRequest::ListDir) {
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

    HandlerImpl {
        console: SerialClient::new(SERIAL_DRIVER),
        input_buf: [0; INPUT_BUF_CAP],
        input_len: 0,
        in_edit: false,
        in_chat: false,
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
