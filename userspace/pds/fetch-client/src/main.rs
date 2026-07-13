#![no_std]
#![no_main]

use lerux_interface_types::{NetRequest, NetResponse};
use lerux_ipc::NetClient;
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

const SERIAL_DRIVER: Channel = Channel::new(0);
const NET_SERVER: NetClient = NetClient::new(Channel::new(1));

const FETCH_HOST: &[u8] = b"host";
const FETCH_PORT: u16 = 8081;
const HTTP_GET: &[u8] = b"GET / HTTP/1.1\r\nHost: host\r\nConnection: close\r\n\r\n";

struct HandlerImpl;

fn dns_resolve(name: &[u8]) -> [u8; 4] {
    match NET_SERVER.call(NetRequest::dns_resolve(name)) {
        NetResponse::Ipv4 { addr } => addr,
        _ => panic!("dns resolve failed"),
    }
}

fn tcp_connect(addr: [u8; 4], port: u16) {
    match NET_SERVER.call(NetRequest::TcpConnect { addr, port }) {
        NetResponse::Ok => {}
        _ => panic!("tcp connect failed"),
    }
}

fn tcp_send(data: &[u8]) {
    match NET_SERVER.call(NetRequest::tcp_send(data)) {
        NetResponse::Ok => {}
        _ => panic!("tcp send failed"),
    }
}

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

fn contains_http_200(buf: &[u8]) -> bool {
    buf.windows(3).any(|w| w == b"200")
}

fn probe_fetch() {
    let addr = dns_resolve(FETCH_HOST);
    tcp_connect(addr, FETCH_PORT);
    tcp_send(HTTP_GET);
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
