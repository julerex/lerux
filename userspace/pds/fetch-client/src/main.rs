#![no_std]
#![no_main]

#[cfg(not(feature = "tls"))]
use lerux_interface_types::{NetRequest, NetResponse};
#[cfg(feature = "tls")]
use lerux_interface_types::{TlsRequest, TlsResponse};
#[cfg(not(feature = "tls"))]
use lerux_ipc::NetClient;
#[cfg(feature = "tls")]
use lerux_ipc::TlsClient;
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

const SERIAL_DRIVER: Channel = Channel::new(0);
#[cfg(not(feature = "tls"))]
const NET_SERVER: NetClient = NetClient::new(Channel::new(1));
#[cfg(feature = "tls")]
const TLS_PROXY: TlsClient = TlsClient::new(Channel::new(1));

const FETCH_HOST: &[u8] = b"host";
#[cfg(not(feature = "tls"))]
const FETCH_PORT: u16 = 8081;
#[cfg(feature = "tls")]
const FETCH_PORT: u16 = 8443;
const HTTP_GET: &[u8] = b"GET / HTTP/1.1\r\nHost: host\r\nConnection: close\r\n\r\n";

struct HandlerImpl;

#[cfg(not(feature = "tls"))]
fn dns_resolve(name: &[u8]) -> [u8; 4] {
    match NET_SERVER.call(NetRequest::dns_resolve(name)) {
        NetResponse::Ipv4 { addr } => addr,
        _ => panic!("dns resolve failed"),
    }
}

#[cfg(not(feature = "tls"))]
fn tcp_connect(addr: [u8; 4], port: u16) {
    match NET_SERVER.call(NetRequest::TcpConnect { addr, port }) {
        NetResponse::Ok => {}
        _ => panic!("tcp connect failed"),
    }
}

#[cfg(not(feature = "tls"))]
fn tcp_send(data: &[u8]) {
    match NET_SERVER.call(NetRequest::tcp_send(data)) {
        NetResponse::Ok => {}
        _ => panic!("tcp send failed"),
    }
}

#[cfg(not(feature = "tls"))]
fn recv_until_status_200() {
    let mut buf = [0u8; 256];
    let mut total = 0usize;
    for _ in 0..32 {
        match NET_SERVER.call(NetRequest::TcpRecv) {
            NetResponse::TcpData { data_len, data } => {
                let len = data_len as usize;
                if total + len <= buf.len() {
                    buf[total..total + len].copy_from_slice(&data[..len]);
                    total += len;
                }
                if contains_http_200(&buf[..total]) {
                    log::info!("lerux-fetch: 200");
                    return;
                }
            }
            NetResponse::Ok => {
                if contains_http_200(&buf[..total]) {
                    log::info!("lerux-fetch: 200");
                    return;
                }
                break;
            }
            NetResponse::Pending
            | NetResponse::Error
            | NetResponse::Ipv4 { .. }
            | NetResponse::Iface { .. }
            | NetResponse::UdpData { .. } => {
                panic!("tcp recv failed")
            }
        }
    }
    panic!("fetch did not see HTTP 200");
}

#[cfg(feature = "tls")]
fn tls_connect(name: &[u8], port: u16) {
    match TLS_PROXY.call(TlsRequest::connect(name, port)) {
        TlsResponse::Ok => {}
        _ => panic!("tls connect failed"),
    }
}

#[cfg(feature = "tls")]
fn tls_send(data: &[u8]) {
    match TLS_PROXY.call(TlsRequest::send(data)) {
        TlsResponse::Ok => {}
        _ => panic!("tls send failed"),
    }
}

#[cfg(feature = "tls")]
fn recv_until_status_200() {
    let mut buf = [0u8; 256];
    let mut total = 0usize;
    for _ in 0..32 {
        match TLS_PROXY.call(TlsRequest::Recv) {
            TlsResponse::Data { data_len, data } => {
                let len = data_len as usize;
                if total + len <= buf.len() {
                    buf[total..total + len].copy_from_slice(&data[..len]);
                    total += len;
                }
                if contains_http_200(&buf[..total]) {
                    log::info!("lerux-fetch: 200");
                    return;
                }
            }
            TlsResponse::Ok => {
                if contains_http_200(&buf[..total]) {
                    log::info!("lerux-fetch: 200");
                    return;
                }
                break;
            }
            TlsResponse::Pending | TlsResponse::Error => panic!("tls recv failed"),
        }
    }
    panic!("fetch did not see HTTP 200");
}

fn contains_http_200(buf: &[u8]) -> bool {
    buf.windows(3).any(|w| w == b"200")
}

fn probe_fetch() {
    #[cfg(not(feature = "tls"))]
    {
        let addr = dns_resolve(FETCH_HOST);
        tcp_connect(addr, FETCH_PORT);
        tcp_send(HTTP_GET);
    }
    #[cfg(feature = "tls")]
    {
        tls_connect(FETCH_HOST, FETCH_PORT);
        tls_send(HTTP_GET);
    }
    recv_until_status_200();
}

#[protection_domain]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();
    probe_fetch();
    HandlerImpl
}

impl Handler for HandlerImpl {
    type Error = Infallible;
}
