#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;

use lerux_interface_types::{
    HttpRequest, HttpResponse, WebContentRequest, WebContentResponse, DISPLAY_HEIGHT,
    DISPLAY_STRIDE, DISPLAY_VISIBLE_BYTES, DISPLAY_WIDTH,
};
use lerux_ipc::{recv, send, send_unspecified_error, HttpClient};
use lerux_logging::{log, serial};
use lerux_web::{fixture_signatures_ok, render, Bitmap};
use sel4_microkit::{protection_domain, var, Channel, Handler, Infallible, MessageInfo};

/// Channel 0: serial-virt (`<end pd="web_content" id="0" pp="true" />`).
const SERIAL_DRIVER: Channel = Channel::new(0);
/// Channel 1: request-server (`<end pd="web_content" id="1" pp="true" />`).
const REQUEST_SERVER: Channel = Channel::new(1);
/// Channel 2: browser-ui (`<end pd="web_content" id="2" />`).
const BROWSER_UI: Channel = Channel::new(2);

#[protection_domain(heap_size = 128 * 1024)]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();

    let bitmap = *var!(bitmap_vaddr: usize = 0) as *mut u8;
    assert!(!bitmap.is_null());

    HandlerImpl { bitmap }
}

struct HandlerImpl {
    bitmap: *mut u8,
}

impl HandlerImpl {
    fn handle(&self, req: WebContentRequest) -> WebContentResponse {
        match req {
            WebContentRequest::Navigate { .. } => self.navigate(req.url()),
        }
    }

    fn navigate(&self, url: &[u8]) -> WebContentResponse {
        let Ok((status, body)) = fetch_html(url) else {
            log::info!("lerux-web: fetch failed");
            return WebContentResponse::Error;
        };
        if status != 200 {
            log::info!("lerux-web: status {status}");
            return WebContentResponse::Error;
        }
        let Ok(html) = core::str::from_utf8(&body) else {
            log::info!("lerux-web: body not utf-8");
            return WebContentResponse::Error;
        };

        // SAFETY: `bitmap` is the 2 MiB shared MR; we only write the visible prefix.
        let slice = unsafe { core::slice::from_raw_parts_mut(self.bitmap, DISPLAY_VISIBLE_BYTES) };
        let mut bmp =
            Bitmap::new(slice, DISPLAY_WIDTH, DISPLAY_HEIGHT, DISPLAY_STRIDE).expect("bitmap mr");
        render(html, &mut bmp);

        let signatures = fixture_signatures_ok(&bmp);
        if signatures {
            log::info!("lerux-web: paint ok");
        } else {
            log::info!("lerux-web: paint mismatch");
        }
        WebContentResponse::Painted { status, signatures }
    }
}

fn fetch_html(url: &[u8]) -> Result<(u16, Vec<u8>), ()> {
    let client = HttpClient::new(REQUEST_SERVER);
    match client.call(HttpRequest::get(url)) {
        HttpResponse::Ok => {}
        _ => return Err(()),
    }
    let code = match client.call(HttpRequest::Finish) {
        HttpResponse::Status { code } => code,
        _ => {
            let _ = client.call(HttpRequest::Close);
            return Err(());
        }
    };
    let mut body = Vec::new();
    loop {
        match client.call(HttpRequest::Recv) {
            HttpResponse::Data {
                data_len,
                data,
                last,
            } => {
                body.extend_from_slice(&data[..data_len as usize]);
                if last {
                    break;
                }
            }
            _ => {
                let _ = client.call(HttpRequest::Close);
                return Err(());
            }
        }
    }
    let _ = client.call(HttpRequest::Close);
    Ok((code, body))
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != BROWSER_UI {
            unreachable!();
        }
        Ok(match recv::<WebContentRequest>(msg_info) {
            Ok(req) => send(self.handle(req)),
            Err(_) => send_unspecified_error(),
        })
    }
}
