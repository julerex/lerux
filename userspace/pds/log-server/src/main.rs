//! Central log ring + serial mux (Phase 36 / 57).
//!
//! Clients append tagged lines via postcard RPC. Shell `dmesg` fetches a
//! filtered window; supervisor persists the boot ring to `/boot.log`.

#![no_std]
#![no_main]

use embedded_hal_nb::serial::Write as _;
use heapless::Deque;
use lerux_interface_types::{
    LogRequest, LogResponse, LOG_LEVEL_DEBUG, LOG_LEVEL_ERROR, LOG_LEVEL_INFO, LOG_LEVEL_WARN,
    MAX_LOG_LINES, MAX_LOG_MSG, MAX_LOG_TAG,
};
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

/// Phase 57: larger ring than the dmesg window (window is still MAX_LOG_LINES).
const RING_CAP: usize = 48;

#[derive(Clone, Copy)]
struct LogLine {
    level: u8,
    tag_len: u8,
    tag: [u8; MAX_LOG_TAG],
    len: u8,
    text: [u8; MAX_LOG_MSG],
}

struct HandlerImpl {
    out: SerialClient,
    ring: Deque<LogLine, RING_CAP>,
    /// Drop appends strictly above this level (higher number = more verbose).
    min_level: u8,
}

impl HandlerImpl {
    fn tag_for_channel(channel: Channel) -> (&'static [u8], u8) {
        if channel == SHELL {
            (b"shell", 5)
        } else if channel == SUPERVISOR {
            (b"supervis", 8)
        } else if channel == FS_SERVER {
            (b"fs", 2)
        } else if channel == NET_SERVER {
            (b"net", 3)
        } else {
            (b"?", 1)
        }
    }

    fn level_char(level: u8) -> u8 {
        match level {
            LOG_LEVEL_ERROR => b'E',
            LOG_LEVEL_WARN => b'W',
            LOG_LEVEL_INFO => b'I',
            LOG_LEVEL_DEBUG => b'D',
            _ => b'?',
        }
    }

    fn append(
        &mut self,
        channel: Channel,
        level: u8,
        tag_len: u8,
        tag: &[u8; MAX_LOG_TAG],
        text: &[u8],
    ) {
        if level > 0 && self.min_level > 0 && level > self.min_level {
            return;
        }

        let (fallback, fallback_len) = Self::tag_for_channel(channel);
        let (use_tag, use_tag_len) = if tag_len > 0 {
            (&tag[..tag_len as usize], tag_len)
        } else {
            (fallback, fallback_len as u8)
        };

        let mut line = LogLine {
            level: if level == 0 { LOG_LEVEL_INFO } else { level },
            tag_len: use_tag_len,
            tag: [0u8; MAX_LOG_TAG],
            len: 0,
            text: [0u8; MAX_LOG_MSG],
        };
        let tn = (use_tag_len as usize).min(MAX_LOG_TAG);
        line.tag[..tn].copy_from_slice(&use_tag[..tn]);
        let n = text.len().min(MAX_LOG_MSG);
        line.len = n as u8;
        line.text[..n].copy_from_slice(&text[..n]);

        if self.ring.is_full() {
            let _ = self.ring.pop_front();
        }
        let _ = self.ring.push_back(line);

        // Structured serial: `E[shell] message\n` (newline required for host smoke capture).
        let _ = self.out.write(Self::level_char(line.level));
        let _ = self.out.write(b'[');
        for &b in &line.tag[..line.tag_len as usize] {
            let _ = self.out.write(b);
        }
        let _ = self.out.write(b']');
        let _ = self.out.write(b' ');
        for &b in &text[..n] {
            let _ = self.out.write(b);
        }
        let _ = self.out.write(b'\n');
        let _ = self.out.flush();
    }

    fn get_recent(&self, min_level: u8, tag_len: u8, tag: &[u8; MAX_LOG_TAG]) -> LogResponse {
        let mut matched: heapless::Vec<&LogLine, RING_CAP> = heapless::Vec::new();
        for entry in self.ring.iter() {
            if min_level > 0 && entry.level > 0 && entry.level > min_level {
                continue;
            }
            if tag_len > 0 {
                let want = &tag[..tag_len as usize];
                let have = &entry.tag[..entry.tag_len as usize];
                if have != want {
                    continue;
                }
            }
            let _ = matched.push(entry);
        }

        let total = matched.len();
        let take = total.min(MAX_LOG_LINES);
        let skip = total - take;

        let mut count: u8 = 0;
        let mut lens = [0u8; MAX_LOG_LINES];
        let mut lines = [[0u8; MAX_LOG_MSG]; MAX_LOG_LINES];
        let mut levels = [0u8; MAX_LOG_LINES];
        let mut tag_lens = [0u8; MAX_LOG_LINES];
        let mut tags = [[0u8; MAX_LOG_TAG]; MAX_LOG_LINES];

        for (j, entry) in matched.iter().enumerate() {
            if j < skip {
                continue;
            }
            if (count as usize) >= MAX_LOG_LINES {
                break;
            }
            let i = count as usize;
            lens[i] = entry.len;
            lines[i] = entry.text;
            levels[i] = entry.level;
            tag_lens[i] = entry.tag_len;
            tags[i] = entry.tag;
            count += 1;
        }

        LogResponse::Recent {
            count,
            lens,
            lines,
            levels,
            tag_lens,
            tags,
        }
    }
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    let out = SerialClient::new(SERIAL_DRIVER);
    // Own logs go via debug sink (sel4 debug_print), not the log service.
    log::info!(
        "log-server: ready ring={} window={}",
        RING_CAP,
        MAX_LOG_LINES
    );
    HandlerImpl {
        out,
        ring: Deque::new(),
        min_level: LOG_LEVEL_INFO,
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        let allowed = channel == SHELL
            || channel == SUPERVISOR
            || channel == FS_SERVER
            || channel == NET_SERVER;
        if !allowed {
            return Ok(send_unspecified_error());
        }

        Ok(match recv::<LogRequest>(msg_info) {
            Ok(req) => match req {
                LogRequest::Append {
                    level,
                    tag_len,
                    tag,
                    len,
                    text,
                } => {
                    let n = len as usize;
                    self.append(channel, level, tag_len, &tag, &text[..n]);
                    send(LogResponse::Ok)
                }
                LogRequest::Subscribe => send(LogResponse::Ok),
                LogRequest::GetRecent {
                    min_level,
                    tag_len,
                    tag,
                } => send(self.get_recent(min_level, tag_len, &tag)),
                LogRequest::SetMinLevel { level } => {
                    self.min_level = if level == 0 { LOG_LEVEL_INFO } else { level };
                    log::info!("log-server: min_level={}", self.min_level);
                    send(LogResponse::Ok)
                }
            },
            Err(_) => send_unspecified_error(),
        })
    }
}
