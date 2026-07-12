#![no_std]
#![no_main]

use lerux_interface_types::{
    FsRequest, FsResponse, NetRequest, NetResponse, MAX_FS_PATH, MAX_NET_TCP_PAYLOAD,
};
use lerux_ipc::{call, send_unspecified_error};
use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible, MessageInfo};

const NET_SERVER: Channel = Channel::new(0);
const FS_SERVER: Channel = Channel::new(1);

const HTTP_PORT: u16 = 8080;
const INIT_SERVE_ROUNDS: usize = 100;

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

fn poll_net() -> NetResponse {
    loop {
        match call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::Poll) {
            Ok(NetResponse::Pending) => {}
            Ok(other) => return other,
            Err(_) => return NetResponse::Error,
        }
    }
}

fn net_call(req: NetRequest) -> NetResponse {
    match call::<NetRequest, NetResponse>(NET_SERVER, req) {
        Ok(NetResponse::Pending) => poll_net(),
        Ok(other) => other,
        Err(_) => NetResponse::Error,
    }
}

/// Non-blocking poll: returns Pending after a few rounds instead of spinning forever.
fn net_call_try(req: NetRequest) -> NetResponse {
    match call::<NetRequest, NetResponse>(NET_SERVER, req) {
        Ok(NetResponse::Pending) => {
            for _ in 0..32 {
                match call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::Poll) {
                    Ok(NetResponse::Pending) => {}
                    Ok(other) => return other,
                    Err(_) => return NetResponse::Error,
                }
            }
            let _ = call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::Abort);
            NetResponse::Pending
        }
        Ok(other) => other,
        Err(_) => NetResponse::Error,
    }
}

fn write_u16_dec(buf: &mut [u8], mut n: u16) -> usize {
    let mut tmp = [0u8; 5];
    let mut i = 0usize;
    if n == 0 {
        tmp[0] = b'0';
        i = 1;
    } else {
        while n > 0 {
            tmp[i] = b'0' + (n % 10) as u8;
            n /= 10;
            i += 1;
        }
    }
    for j in 0..i {
        buf[j] = tmp[i - 1 - j];
    }
    i
}

/// Build `HTTP/1.0 200 OK` (or status line) + body into a fixed TCP payload buffer.
fn build_http_response(status: &[u8], body: &[u8], out: &mut [u8; MAX_NET_TCP_PAYLOAD]) -> usize {
    // status\r\nContent-Type: text/plain\r\nContent-Length: N\r\n\r\nBODY
    let mut pos = 0usize;
    let hdr_tail = b"\r\nContent-Type: text/plain\r\nContent-Length: ";
    let body_len = body
        .len()
        .min(MAX_NET_TCP_PAYLOAD.saturating_sub(status.len() + hdr_tail.len() + 5 + 4));

    if pos + status.len() > out.len() {
        return 0;
    }
    out[pos..pos + status.len()].copy_from_slice(status);
    pos += status.len();

    if pos + hdr_tail.len() > out.len() {
        return 0;
    }
    out[pos..pos + hdr_tail.len()].copy_from_slice(hdr_tail);
    pos += hdr_tail.len();

    let nlen = write_u16_dec(&mut out[pos..], body_len as u16);
    pos += nlen;

    if pos + 4 > out.len() {
        return 0;
    }
    out[pos..pos + 4].copy_from_slice(b"\r\n\r\n");
    pos += 4;

    out[pos..pos + body_len].copy_from_slice(&body[..body_len]);
    pos + body_len
}

fn parse_get_path(req: &[u8]) -> Option<&[u8]> {
    if req.len() < 4 || &req[..4] != b"GET " {
        return None;
    }
    let rest = &req[4..];
    let end = rest
        .iter()
        .position(|&b| b == b' ' || b == b'\r' || b == b'\n')
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

fn list_dir_body(out: &mut [u8]) -> usize {
    match fs_call(FsRequest::list_root()) {
        FsResponse::DirList { count, entries } => {
            let mut pos = 0usize;
            for e in entries.iter().take(count as usize) {
                let name = e.name_slice();
                if pos + name.len() + 1 > out.len() {
                    break;
                }
                out[pos..pos + name.len()].copy_from_slice(name);
                pos += name.len();
                out[pos] = b'\n';
                pos += 1;
            }
            pos
        }
        _ => {
            let msg = b"list error\n";
            out[..msg.len()].copy_from_slice(msg);
            msg.len()
        }
    }
}

fn read_file_body(path: &[u8], out: &mut [u8]) -> Option<usize> {
    let handle = match fs_call(FsRequest::open(path)) {
        FsResponse::Handle { id } => id,
        _ => return None,
    };
    match fs_call(FsRequest::Read {
        handle,
        offset: 0,
        len: out.len() as u16,
    }) {
        FsResponse::Data { data_len, data } => {
            let n = data_len as usize;
            out[..n].copy_from_slice(&data[..n]);
            Some(n)
        }
        FsResponse::Ok => Some(0),
        _ => None,
    }
}

fn try_serve() {
    let NetResponse::TcpData { data_len, data } = net_call_try(NetRequest::TcpRecv) else {
        return;
    };
    let req = &data[..data_len as usize];
    let Some(url_path) = parse_get_path(req) else {
        let mut resp = [0u8; MAX_NET_TCP_PAYLOAD];
        let n = build_http_response(b"HTTP/1.0 400 Bad Request", b"bad request\n", &mut resp);
        let _ = net_call(NetRequest::tcp_send(&resp[..n]));
        let _ = net_call(NetRequest::TcpClose);
        let _ = net_call(NetRequest::TcpListen { port: HTTP_PORT });
        return;
    };

    let mut body = [0u8; 400];
    let (ok, body_len) = if url_path.is_empty() || url_path == b"/" {
        (true, list_dir_body(&mut body))
    } else {
        // Keep leading '/', matching workstation paths like `/boot.log`.
        let path = if url_path.len() > MAX_FS_PATH {
            &url_path[..MAX_FS_PATH]
        } else {
            url_path
        };
        match read_file_body(path, &mut body) {
            Some(n) => (true, n),
            None => {
                let msg = b"not found\n";
                body[..msg.len()].copy_from_slice(msg);
                (false, msg.len())
            }
        }
    };
    let status = if ok {
        &b"HTTP/1.0 200 OK"[..]
    } else {
        &b"HTTP/1.0 404 Not Found"[..]
    };

    let mut resp = [0u8; MAX_NET_TCP_PAYLOAD];
    let n = build_http_response(status, &body[..body_len], &mut resp);
    let _ = net_call(NetRequest::tcp_send(&resp[..n]));
    let _ = net_call(NetRequest::TcpClose);
    let _ = net_call(NetRequest::TcpListen { port: HTTP_PORT });
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug log init");
    if net_call(NetRequest::TcpListen { port: HTTP_PORT }) == NetResponse::Ok {
        log::info!("lerux-http-fs: listening");
    }
    log::info!("lerux-http-fs: ready");
    for _ in 0..INIT_SERVE_ROUNDS {
        try_serve();
    }
    HandlerImpl
}

struct HandlerImpl;

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        _channel: Channel,
        _msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        Ok(send_unspecified_error())
    }

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(NET_SERVER) {
            try_serve();
        }
        Ok(())
    }
}
