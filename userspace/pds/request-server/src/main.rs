#![no_std]
#![no_main]

extern crate alloc;

mod http;

use alloc::vec::Vec;

use lerux_interface_types::{
    http_content_length, http_status_code, split_http_head, HttpMethod, HttpRequest, HttpResponse,
    HttpUrl, TlsRequest, TlsResponse, MAX_HTTP_HEADER_NAME, MAX_HTTP_HEADER_VALUE, MAX_HTTP_URL,
    MAX_NET_TCP_PAYLOAD,
};
use lerux_ipc::{recv, send, send_unspecified_error, TlsClient};
use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};

use crate::http::{build_request, ExtraHeader, MAX_EXTRA_HEADERS};

/// Channel IDs match request / agent / browser / interactive templates.
const TLS_PROXY: TlsClient = TlsClient::new(Channel::new(1));
const APP: Channel = Channel::new(2);
/// Second untrusted HTTP client (agent on the joint profile; unwired elsewhere).
const APP2: Channel = Channel::new(3);

const MAX_STEPS: usize = 64;
const MAX_HTTP_RAW: usize = 8192;
const MAX_HTTP_BODY: usize = 4096;

struct Building {
    method: HttpMethod,
    url: HttpUrl,
    headers: [Option<ExtraHeader>; MAX_EXTRA_HEADERS],
    header_count: usize,
    body: Vec<u8>,
}

struct Exchange {
    body: Vec<u8>,
    offset: usize,
}

/// Per-client HTTP session state. Phase 80 wires web-content and agent to the
/// same request-server; shared state would let one client reset or steal the
/// other's in-flight request or response body.
struct ClientSession {
    building: Option<Building>,
    exchange: Option<Exchange>,
}

impl ClientSession {
    const fn new() -> Self {
        Self {
            building: None,
            exchange: None,
        }
    }
}

struct HandlerImpl {
    app: ClientSession,
    app2: ClientSession,
}

#[protection_domain(heap_size = 64 * 1024)]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    log::info!("lerux-http: ready");
    HandlerImpl {
        app: ClientSession::new(),
        app2: ClientSession::new(),
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        let session = match channel {
            APP => &mut self.app,
            APP2 => &mut self.app2,
            _ => unreachable!(),
        };

        Ok(match recv::<HttpRequest>(msg_info) {
            Ok(req) => send(handle_req(session, req)),
            Err(_) => send_unspecified_error(),
        })
    }
}

fn handle_req(session: &mut ClientSession, req: HttpRequest) -> HttpResponse {
    match req {
        HttpRequest::Start {
            method,
            url_len,
            url,
        } => {
            session.reset();
            let url_len = (url_len as usize).min(MAX_HTTP_URL);
            let Some(parsed) = HttpUrl::parse(&url[..url_len]) else {
                return HttpResponse::Error;
            };
            session.building = Some(Building {
                method,
                url: parsed,
                headers: core::array::from_fn(|_| None),
                header_count: 0,
                body: Vec::new(),
            });
            HttpResponse::Ok
        }
        HttpRequest::Header {
            name_len,
            name,
            value_len,
            value,
        } => {
            let Some(building) = session.building.as_mut() else {
                return HttpResponse::Error;
            };
            if building.header_count >= MAX_EXTRA_HEADERS {
                return HttpResponse::Error;
            }
            let name_len = (name_len as usize).min(MAX_HTTP_HEADER_NAME);
            let value_len = (value_len as usize).min(MAX_HTTP_HEADER_VALUE);
            building.headers[building.header_count] = Some(ExtraHeader::from_parts(
                &name[..name_len],
                &value[..value_len],
            ));
            building.header_count += 1;
            HttpResponse::Ok
        }
        HttpRequest::Body {
            payload_len,
            payload,
            ..
        } => {
            let Some(building) = session.building.as_mut() else {
                return HttpResponse::Error;
            };
            let add = payload_len as usize;
            if building.body.len().saturating_add(add) > MAX_HTTP_BODY {
                return HttpResponse::Error;
            }
            building
                .body
                .extend_from_slice(&payload[..add.min(payload.len())]);
            HttpResponse::Ok
        }
        HttpRequest::Finish => finish(session),
        HttpRequest::Recv => recv_body(session),
        HttpRequest::Close => {
            session.reset();
            HttpResponse::Ok
        }
        HttpRequest::Poll => HttpResponse::Pending,
    }
}

fn finish(session: &mut ClientSession) -> HttpResponse {
    let Some(building) = session.building.take() else {
        return HttpResponse::Error;
    };
    if !building.url.https || !matches!(building.method, HttpMethod::Get | HttpMethod::Post) {
        close_tls();
        return HttpResponse::Error;
    }
    match https_exchange(
        building.method,
        &building.url,
        &building.headers,
        &building.body,
    ) {
        Ok((code, body)) => {
            session.exchange = Some(Exchange { body, offset: 0 });
            HttpResponse::Status { code }
        }
        Err(()) => {
            log::info!("lerux-http: exchange failed");
            HttpResponse::Error
        }
    }
}

fn recv_body(session: &mut ClientSession) -> HttpResponse {
    let Some(ex) = session.exchange.as_mut() else {
        return HttpResponse::Error;
    };
    if ex.offset >= ex.body.len() {
        return HttpResponse::data(&[], true);
    }
    let end = (ex.offset + MAX_NET_TCP_PAYLOAD).min(ex.body.len());
    let chunk = &ex.body[ex.offset..end];
    ex.offset = end;
    HttpResponse::data(chunk, ex.offset >= ex.body.len())
}

impl ClientSession {
    fn reset(&mut self) {
        self.building = None;
        self.exchange = None;
        close_tls();
    }
}

fn https_exchange(
    method: HttpMethod,
    url: &HttpUrl,
    extra: &[Option<ExtraHeader>],
    body: &[u8],
) -> Result<(u16, Vec<u8>), ()> {
    match TLS_PROXY.call(TlsRequest::connect(url.host(), url.port)) {
        TlsResponse::Ok => {}
        _ => {
            close_tls();
            return Err(());
        }
    }
    let req = build_request(method, url, extra, body);
    for chunk in req.chunks(MAX_NET_TCP_PAYLOAD) {
        match TLS_PROXY.call(TlsRequest::send(chunk)) {
            TlsResponse::Ok => {}
            _ => {
                close_tls();
                return Err(());
            }
        }
    }
    let raw = recv_http_raw().inspect_err(|_| {
        close_tls();
    })?;
    close_tls();
    parse_response(&raw)
}

fn recv_http_raw() -> Result<Vec<u8>, ()> {
    let mut buf = Vec::new();
    for _ in 0..MAX_STEPS {
        match TLS_PROXY.call(TlsRequest::Recv) {
            TlsResponse::Data { data_len, data } => {
                buf.extend_from_slice(&data[..data_len as usize]);
                if buf.len() > MAX_HTTP_RAW {
                    return Err(());
                }
                if http_message_complete(&buf) {
                    return Ok(buf);
                }
            }
            TlsResponse::Pending => {}
            TlsResponse::Error | TlsResponse::Ok => {
                return if split_http_head(&buf).is_some() {
                    Ok(buf)
                } else {
                    Err(())
                };
            }
        }
    }
    Err(())
}

fn http_message_complete(buf: &[u8]) -> bool {
    let Some((head, body)) = split_http_head(buf) else {
        return false;
    };
    let Some(len) = http_content_length(head) else {
        return false;
    };
    body.len() >= len
}

fn parse_response(raw: &[u8]) -> Result<(u16, Vec<u8>), ()> {
    let (head, body) = split_http_head(raw).ok_or(())?;
    let code = http_status_code(head).ok_or(())?;
    let mut body = if let Some(len) = http_content_length(head) {
        body.get(..len.min(body.len())).ok_or(())?.to_vec()
    } else {
        body.to_vec()
    };
    if body.len() > MAX_HTTP_BODY {
        body.truncate(MAX_HTTP_BODY);
    }
    Ok((code, body))
}

fn close_tls() {
    let _ = TLS_PROXY.call(TlsRequest::Close);
}
