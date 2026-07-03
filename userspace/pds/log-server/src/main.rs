#![no_std]
#![no_main]

use embedded_hal_nb::serial::Write as _;
use heapless::Deque;
use lerux_interface_types::{LogRequest, LogResponse, MAX_LOG_LINES, MAX_LOG_MSG};
use lerux_ipc::{recv, send, send_unspecified_error};
use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};
use sel4_microkit_driver_adapters::serial::client::Client as SerialClient;

const SERIAL_DRIVER: Channel = Channel::new(0);
// Server-side channels for log clients (wiring matches .system)
const SHELL: Channel = Channel::new(1);
const SUPERVISOR: Channel = Channel::new(2);
const FS_SERVER: Channel = Channel::new(3);
const NET_SERVER: Channel = Channel::new(4);

const RING_CAP: usize = 32;

#[derive(Clone, Copy)]
struct LogLine {
    len: u8,
    text: [u8; MAX_LOG_MSG],
}

struct HandlerImpl {
    out: SerialClient,
    ring: Deque<LogLine, RING_CAP>,
}

impl HandlerImpl {
    fn append(&mut self, _level: u8, text: &[u8]) {
        let mut line = LogLine {
            len: 0,
            text: [0u8; MAX_LOG_MSG],
        };
        let n = text.len().min(MAX_LOG_MSG);
        line.len = n as u8;
        line.text[..n].copy_from_slice(&text[..n]);

        if self.ring.is_full() {
            let _ = self.ring.pop_front();
        }
        let _ = self.ring.push_back(line);

        // Emit to serial (raw, like prior direct sinks)
        for &b in &text[..n] {
            let _ = self.out.write(b);
        }
        let _ = self.out.flush();
    }

    fn get_recent(&self) -> LogResponse {
        let total = self.ring.len();
        let take = total.min(MAX_LOG_LINES);
        let skip = total - take;

        let mut count: u8 = 0;
        let mut lens = [0u8; MAX_LOG_LINES];
        let mut lines = [[0u8; MAX_LOG_MSG]; MAX_LOG_LINES];

        for (j, entry) in self.ring.iter().enumerate() {
            if j < skip {
                continue;
            }
            if (count as usize) >= MAX_LOG_LINES {
                break;
            }
            lens[count as usize] = entry.len;
            lines[count as usize] = entry.text;
            count += 1;
        }

        LogResponse::Recent { count, lens, lines }
    }
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    let out = SerialClient::new(SERIAL_DRIVER);
    // Our own logs go via debug sink (sel4 debug_print), not the log service to avoid loops.
    log::info!("log-server: ready");
    HandlerImpl {
        out,
        ring: Deque::new(),
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        // Only accept from known log clients or future internal
        let allowed = channel == SHELL
            || channel == SUPERVISOR
            || channel == FS_SERVER
            || channel == NET_SERVER;
        if !allowed {
            // Still allow? For now treat unexpected as error but don't panic per convention
            return Ok(send_unspecified_error());
        }

        Ok(match recv::<LogRequest>(msg_info) {
            Ok(req) => match req {
                LogRequest::Append { level, len, text } => {
                    let n = len as usize;
                    self.append(level, &text[..n]);
                    send(LogResponse::Ok)
                }
                LogRequest::Subscribe => {
                    // Stub: ack. Future: register channel for notify on append.
                    send(LogResponse::Ok)
                }
                LogRequest::GetRecent => send(self.get_recent()),
            },
            Err(_) => send_unspecified_error(),
        })
    }
}
