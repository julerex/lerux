//! HTTP file browser over virtio-net + FS (Phase 40 / 58 v2).
//!
//! GET /           — HTML directory listing  
//! GET /path       — file body with MIME from extension  
//! PUT /path       — write request body to path  

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

fn mime_for_path(path: &[u8]) -> &'static [u8] {
    let ext = path
        .iter()
        .rposition(|&b| b == b'.')
        .map(|i| &path[i + 1..])
        .unwrap_or(b"");
    match ext {
        b"html" | b"htm" => b"text/html",
        b"css" => b"text/css",
        b"js" => b"application/javascript",
        b"json" => b"application/json",
        b"png" => b"image/png",
        b"jpg" | b"jpeg" => b"image/jpeg",
        b"gif" => b"image/gif",
        b"svg" => b"image/svg+xml",
        b"txt" | b"log" | b"md" | b"toml" => b"text/plain",
        _ => b"application/octet-stream",
    }
}

fn build_http_response(
    status: &[u8],
    content_type: &[u8],
    body: &[u8],
    out: &mut [u8; MAX_NET_TCP_PAYLOAD],
) -> usize {
    let mut pos = 0usize;
    let mid = b"\r\nContent-Type: ";
    let cl = b"\r\nContent-Length: ";
    let body_budget = MAX_NET_TCP_PAYLOAD
        .saturating_sub(status.len() + mid.len() + content_type.len() + cl.len() + 8 + 4);
    let body_len = body.len().min(body_budget);

    if pos + status.len() > out.len() {
        return 0;
    }
    out[pos..pos + status.len()].copy_from_slice(status);
    pos += status.len();
    if pos + mid.len() + content_type.len() + cl.len() > out.len() {
        return 0;
    }
    out[pos..pos + mid.len()].copy_from_slice(mid);
    pos += mid.len();
    out[pos..pos + content_type.len()].copy_from_slice(content_type);
    pos += content_type.len();
    out[pos..pos + cl.len()].copy_from_slice(cl);
    pos += cl.len();
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

fn parse_request_line(req: &[u8]) -> Option<(&[u8], &[u8])> {
    // METHOD path HTTP/…
    let sp1 = req.iter().position(|&b| b == b' ')?;
    let method = &req[..sp1];
    let rest = &req[sp1 + 1..];
    let sp2 = rest
        .iter()
        .position(|&b| b == b' ' || b == b'\r' || b == b'\n')
        .unwrap_or(rest.len());
    Some((method, &rest[..sp2]))
}

fn find_body(req: &[u8]) -> &[u8] {
    if let Some(i) = req.windows(4).position(|w| w == b"\r\n\r\n") {
        &req[i + 4..]
    } else {
        b""
    }
}

fn list_dir_html(out: &mut [u8]) -> usize {
    match fs_call(FsRequest::list_root()) {
        FsResponse::DirList { count, entries } => {
            let mut pos = 0usize;
            let head = b"<html><body><h1>lerux fs</h1><ul>\n";
            if head.len() > out.len() {
                return 0;
            }
            out[..head.len()].copy_from_slice(head);
            pos += head.len();
            for e in entries.iter().take(count as usize) {
                let name = e.name_slice();
                // <li><a href="/name">name</a></li>\n
                let prefix = b"<li><a href=\"/";
                let mid = b"\">";
                let suffix = b"</a></li>\n";
                let need = prefix.len() + name.len() + mid.len() + name.len() + suffix.len();
                if pos + need > out.len() {
                    break;
                }
                out[pos..pos + prefix.len()].copy_from_slice(prefix);
                pos += prefix.len();
                out[pos..pos + name.len()].copy_from_slice(name);
                pos += name.len();
                out[pos..pos + mid.len()].copy_from_slice(mid);
                pos += mid.len();
                out[pos..pos + name.len()].copy_from_slice(name);
                pos += name.len();
                out[pos..pos + suffix.len()].copy_from_slice(suffix);
                pos += suffix.len();
            }
            let tail = b"</ul></body></html>\n";
            if pos + tail.len() <= out.len() {
                out[pos..pos + tail.len()].copy_from_slice(tail);
                pos += tail.len();
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

fn open_or_create(path: &[u8]) -> Option<u8> {
    match fs_call(FsRequest::create(path)) {
        FsResponse::Handle { id } => Some(id),
        _ => match fs_call(FsRequest::open(path)) {
            FsResponse::Handle { id } => Some(id),
            _ => None,
        },
    }
}

fn put_file(path: &[u8], body: &[u8]) -> bool {
    let Some(handle) = open_or_create(path) else {
        return false;
    };
    matches!(fs_call(FsRequest::write(handle, 0, body)), FsResponse::Ok)
}

fn try_serve() {
    let NetResponse::TcpData { data_len, data } = net_call_try(NetRequest::TcpRecv) else {
        return;
    };
    let req = &data[..data_len as usize];
    let Some((method, url_path)) = parse_request_line(req) else {
        let mut resp = [0u8; MAX_NET_TCP_PAYLOAD];
        let n = build_http_response(
            b"HTTP/1.0 400 Bad Request",
            b"text/plain",
            b"bad request\n",
            &mut resp,
        );
        let _ = net_call(NetRequest::tcp_send(&resp[..n]));
        let _ = net_call(NetRequest::TcpClose);
        let _ = net_call(NetRequest::TcpListen { port: HTTP_PORT });
        return;
    };

    let mut body = [0u8; 400];
    let mut ctype: &[u8] = b"text/plain";
    let (status, body_len) = if method == b"PUT" {
        let path = if url_path.is_empty() || url_path == b"/" {
            b"/upload"
        } else if url_path.len() > MAX_FS_PATH {
            &url_path[..MAX_FS_PATH]
        } else {
            url_path
        };
        let payload = find_body(req);
        if put_file(path, payload) {
            log::info!("lerux-http-fs: PUT ok");
            let msg = b"created\n";
            body[..msg.len()].copy_from_slice(msg);
            (b"HTTP/1.0 201 Created".as_slice(), msg.len())
        } else {
            let msg = b"put failed\n";
            body[..msg.len()].copy_from_slice(msg);
            (b"HTTP/1.0 500 Internal Server Error".as_slice(), msg.len())
        }
    } else if method != b"GET" {
        let msg = b"method not allowed\n";
        body[..msg.len()].copy_from_slice(msg);
        (b"HTTP/1.0 405 Method Not Allowed".as_slice(), msg.len())
    } else if url_path.is_empty() || url_path == b"/" {
        ctype = b"text/html";
        (b"HTTP/1.0 200 OK".as_slice(), list_dir_html(&mut body))
    } else {
        let path = if url_path.len() > MAX_FS_PATH {
            &url_path[..MAX_FS_PATH]
        } else {
            url_path
        };
        match read_file_body(path, &mut body) {
            Some(n) => {
                ctype = mime_for_path(path);
                (b"HTTP/1.0 200 OK".as_slice(), n)
            }
            None => {
                let msg = b"not found\n";
                body[..msg.len()].copy_from_slice(msg);
                (b"HTTP/1.0 404 Not Found".as_slice(), msg.len())
            }
        }
    };

    let mut resp = [0u8; MAX_NET_TCP_PAYLOAD];
    let n = build_http_response(status, ctype, &body[..body_len], &mut resp);
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
    log::info!("lerux-http-fs: ready (v2 mime/put)");
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
