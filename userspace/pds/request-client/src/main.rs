#![no_std]
#![no_main]

use lerux_interface_types::{HttpRequest, HttpResponse};
use lerux_ipc::HttpClient;
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

/// Channel 0: serial-driver (`<end pd="request_client" id="0" pp="true" />`).
const SERIAL_DRIVER: Channel = Channel::new(0);
/// Channel 1: request-server (`<end pd="request_client" id="1" pp="true" />`).
const REQUEST_SERVER: HttpClient = HttpClient::new(Channel::new(1));

const FETCH_URL: &[u8] = b"https://host:8443/fixture.html";
const FIXTURE_MARK: &[u8] = b"lerux-http-fixture";

struct HandlerImpl;

#[protection_domain]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();
    probe_fixture();
    HandlerImpl
}

fn probe_fixture() {
    match REQUEST_SERVER.call(HttpRequest::get(FETCH_URL)) {
        HttpResponse::Ok => {}
        _ => panic!("http start failed"),
    }
    match REQUEST_SERVER.call(HttpRequest::Finish) {
        HttpResponse::Status { code: 200 } => {}
        _ => panic!("http finish failed"),
    }
    let mut buf = [0u8; 2048];
    let mut total = 0usize;
    loop {
        match REQUEST_SERVER.call(HttpRequest::Recv) {
            HttpResponse::Data {
                data_len,
                data,
                last,
            } => {
                let len = data_len as usize;
                if total + len <= buf.len() {
                    buf[total..total + len].copy_from_slice(&data[..len]);
                    total += len;
                }
                if last {
                    break;
                }
            }
            _ => panic!("http recv failed"),
        }
    }
    let _ = REQUEST_SERVER.call(HttpRequest::Close);
    if contains_mark(&buf[..total], FIXTURE_MARK) {
        log::info!("lerux-http: fixture ok");
        return;
    }
    panic!("http fixture mark missing");
}

fn contains_mark(buf: &[u8], mark: &[u8]) -> bool {
    buf.windows(mark.len()).any(|w| w == mark)
}

impl Handler for HandlerImpl {
    type Error = Infallible;
}
