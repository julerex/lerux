#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;

use lerux_interface_types::{HttpRequest, HttpResponse};
use lerux_ipc::HttpClient;
use lerux_logging::{log, serial};
use lerux_prog::{run, verify, MEMORY_LEN, SMOKE_LOG};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible};

/// Channel 0: serial-driver (`<end pd="program_runtime" id="0" pp="true" />`).
const SERIAL_DRIVER: Channel = Channel::new(0);
/// Channel 1: request-server (`<end pd="program_runtime" id="1" pp="true" />`).
const REQUEST_SERVER: HttpClient = HttpClient::new(Channel::new(1));

const FETCH_URL: &[u8] = b"https://host:8443/smoke.lrw";
/// Header (7) + max Wasm payload + ed25519 signature.
const MAX_BLOB: usize = lerux_prog::HEADER_LEN + lerux_prog::MAX_WASM_LEN + lerux_prog::SIG_LEN;
const SMOKE_VK: [u8; 32] = *include_bytes!("../../../../support/keys/smoke.ed25519.pub");

struct HandlerImpl;

#[protection_domain(heap_size = 128 * 1024)]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).expect("serial");
    match fetch_and_run() {
        Ok(()) => log::info!("lerux-prog: ran"),
        Err(_) => log::info!("lerux-prog: error"),
    }
    HandlerImpl
}

fn fetch_and_run() -> Result<(), ()> {
    let mut blob = [0u8; MAX_BLOB];
    let n = fetch(&mut blob)?;
    let wasm = verify(&blob[..n], &SMOKE_VK).map_err(|_| ())?;
    let mut memory = vec![0u8; MEMORY_LEN];
    let mut saw_log = false;
    run(wasm, &mut memory, |msg| {
        if msg == SMOKE_LOG {
            saw_log = true;
        }
    })
    .map_err(|_| ())?;
    if saw_log {
        Ok(())
    } else {
        Err(())
    }
}

fn fetch(buf: &mut [u8]) -> Result<usize, ()> {
    if REQUEST_SERVER.call(HttpRequest::get(FETCH_URL)) != HttpResponse::Ok {
        return Err(());
    }
    if REQUEST_SERVER.call(HttpRequest::Finish) != (HttpResponse::Status { code: 200 }) {
        return Err(());
    }
    let mut total = 0usize;
    loop {
        match REQUEST_SERVER.call(HttpRequest::Recv) {
            HttpResponse::Data {
                data_len,
                data,
                last,
            } => {
                let len = data_len as usize;
                let Some(end) = total.checked_add(len) else {
                    return Err(());
                };
                if end > buf.len() {
                    return Err(());
                }
                buf[total..end].copy_from_slice(&data[..len]);
                total = end;
                if last {
                    break;
                }
            }
            _ => return Err(()),
        }
    }
    let _ = REQUEST_SERVER.call(HttpRequest::Close);
    Ok(total)
}

impl Handler for HandlerImpl {
    type Error = Infallible;
}
