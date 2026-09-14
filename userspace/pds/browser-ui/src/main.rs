#![no_std]
#![no_main]

#[cfg(not(feature = "interactive"))]
use lerux_driver_protocols::serial::{
    NonBlocking, Request as SerialRequest, Response as SerialResponse, SuccessResponse,
};
use lerux_interface_types::{
    DisplayRequest, DisplayResponse, WebContentRequest, WebContentResponse,
};
use lerux_ipc::call;
#[cfg(feature = "interactive")]
use lerux_logging::debug;
use lerux_logging::log;
#[cfg(not(feature = "interactive"))]
use lerux_logging::serial;
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible};

/// Channel 0: serial-virt (`<end pd="browser_ui" id="0" pp="true" />`).
#[cfg(not(feature = "interactive"))]
const SERIAL_DRIVER: Channel = Channel::new(0);
/// Channel 1: web-content (`<end pd="browser_ui" id="1" pp="true" />`).
const WEB_CONTENT: Channel = Channel::new(1);
/// Channel 2: display-server (`<end pd="browser_ui" id="2" pp="true" />`).
const DISPLAY_SERVER: Channel = Channel::new(2);

const SMOKE_URL: &[u8] = b"https://host:8443/paint.html";
const LINE_CAP: usize = 160;

#[protection_domain]
fn init() -> HandlerImpl {
    #[cfg(feature = "interactive")]
    debug::init().unwrap();
    #[cfg(not(feature = "interactive"))]
    serial::init(SERIAL_DRIVER).unwrap();
    log::info!("lerux-browser: ready");

    let handler = HandlerImpl {
        line: [0u8; LINE_CAP],
        line_len: 0,
    };
    handler.navigate(SMOKE_URL);
    handler
}

struct HandlerImpl {
    #[cfg_attr(
        feature = "interactive",
        expect(dead_code, reason = "serial open line is unused without serial-virt")
    )]
    line: [u8; LINE_CAP],
    #[cfg_attr(
        feature = "interactive",
        expect(dead_code, reason = "serial open line is unused without serial-virt")
    )]
    line_len: usize,
}

impl HandlerImpl {
    fn navigate(&self, url: &[u8]) {
        if let Ok(s) = core::str::from_utf8(url) {
            log::info!("lerux-browser: open {s}");
        } else {
            log::info!("lerux-browser: open <bin>");
        }

        match call::<WebContentRequest, WebContentResponse>(
            WEB_CONTENT,
            WebContentRequest::navigate(url),
        ) {
            Ok(WebContentResponse::Painted { signatures, .. }) => {
                match call::<DisplayRequest, DisplayResponse>(
                    DISPLAY_SERVER,
                    DisplayRequest::Present,
                ) {
                    Ok(DisplayResponse::Ok) if signatures => {
                        log::info!("lerux-browser: paint ok")
                    }
                    Ok(DisplayResponse::Ok) => log::info!("lerux-browser: loaded"),
                    _ => panic!("Present"),
                }
            }
            _ => log::info!("lerux-browser: navigate failed"),
        }
    }

    #[cfg(not(feature = "interactive"))]
    fn push_byte(&mut self, b: u8) {
        if b == b'\r' {
            return;
        }
        if b == b'\n' {
            self.handle_line();
            self.line_len = 0;
            return;
        }
        if self.line_len < self.line.len() {
            self.line[self.line_len] = b;
            self.line_len += 1;
        }
    }

    #[cfg(not(feature = "interactive"))]
    fn handle_line(&mut self) {
        let line = self.line[..self.line_len].trim_ascii();
        let Some(url) = line.strip_prefix(b"open ") else {
            return;
        };
        let url = url.trim_ascii();
        if !url.is_empty() {
            self.navigate(url);
        }
    }

    #[cfg(not(feature = "interactive"))]
    fn drain_serial(&mut self) {
        while let Ok(Ok(SuccessResponse::Read(NonBlocking::Ready(b)))) =
            call::<SerialRequest, SerialResponse>(SERIAL_DRIVER, SerialRequest::Read)
        {
            self.push_byte(b);
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, _channels: ChannelSet) -> Result<(), Self::Error> {
        #[cfg(not(feature = "interactive"))]
        self.drain_serial();
        Ok(())
    }
}
