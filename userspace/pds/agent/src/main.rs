#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;

use embedded_hal_nb::{
    nb,
    serial::{Read as _, Write as _},
};
use lerux_interface_types::{
    AgentRequest, AgentResponse, AgentToolKind, GrokStubReply, HttpRequest, HttpResponse,
    AGENT_GROK_ONE_URL, AGENT_SMOKE_BODY, AGENT_SMOKE_PATH, AGENT_SMOKE_PROMPT, GROK_STUB_PROMPT,
    GROK_STUB_TOOL_RESULT, MAX_AGENT_PROMPT, MAX_AGENT_TEXT, MAX_NET_TCP_PAYLOAD,
};
use lerux_ipc::{recv, send, send_unspecified_error, HttpClient};
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible, MessageInfo};
use sel4_microkit_driver_adapters::serial::client::Client as SerialClient;

/// Channel 0: serial-driver (`<end pd="agent" id="0" pp="true" />`).
const SERIAL_DRIVER: Channel = Channel::new(0);
/// Channel 1: request-server (`<end pd="agent" id="1" pp="true" />`).
const REQUEST_SERVER: Channel = Channel::new(1);
/// Channel 2: shell `grok` (`<end pd="agent" id="2" />`). Unwired on v1 smoke.
const CLIENT: Channel = Channel::new(2);

const LINE_CAP: usize = MAX_AGENT_PROMPT;

#[protection_domain(heap_size = 64 * 1024)]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();
    log::info!("lerux-agent: ready");

    let mut h = HandlerImpl {
        console: SerialClient::new(SERIAL_DRIVER),
        line: [0u8; LINE_CAP],
        line_len: 0,
        last_user: [0u8; MAX_AGENT_PROMPT],
        last_user_len: 0,
        last_reply: [0u8; MAX_AGENT_TEXT],
        last_reply_len: 0,
    };
    h.draw(b"stub");
    log::info!("lerux-agent: chrome ok");

    match handle_prompt(AGENT_SMOKE_PROMPT) {
        AgentResponse::Text { text_len, text } => {
            h.set_exchange(AGENT_SMOKE_PROMPT, &text[..text_len as usize]);
            h.draw(b"stub");
        }
        _ => panic!("agent smoke prompt"),
    }
    h
}

struct HandlerImpl {
    console: SerialClient,
    line: [u8; LINE_CAP],
    line_len: usize,
    last_user: [u8; MAX_AGENT_PROMPT],
    last_user_len: u8,
    last_reply: [u8; MAX_AGENT_TEXT],
    last_reply_len: u8,
}

impl HandlerImpl {
    fn set_exchange(&mut self, user: &[u8], reply: &[u8]) {
        let n = user.len().min(MAX_AGENT_PROMPT);
        self.last_user[..n].copy_from_slice(&user[..n]);
        self.last_user_len = n as u8;
        let m = reply.len().min(MAX_AGENT_TEXT);
        self.last_reply[..m].copy_from_slice(&reply[..m]);
        self.last_reply_len = m as u8;
    }

    fn draw(&mut self, status: &[u8]) {
        draw_grok_tui(
            &mut self.console,
            status,
            &self.last_user[..self.last_user_len as usize],
            &self.last_reply[..self.last_reply_len as usize],
        );
    }

    fn submit_line(&mut self) {
        let n = self.line_len;
        self.line_len = 0;
        if n == 0 {
            self.draw(b"stub");
            return;
        }
        let mut prompt = [0u8; MAX_AGENT_PROMPT];
        prompt[..n].copy_from_slice(&self.line[..n]);
        match handle_prompt(&prompt[..n]) {
            AgentResponse::Text { text_len, text } => {
                self.set_exchange(&prompt[..n], &text[..text_len as usize]);
                self.draw(b"stub");
            }
            _ => {
                self.set_exchange(&prompt[..n], b"(error)");
                self.draw(b"error");
            }
        }
    }
}

fn write_bytes(console: &mut SerialClient, bytes: &[u8]) {
    for &b in bytes {
        let _ = console.write(b);
    }
    let _ = console.flush();
}

/// Fullscreen ANSI chrome (grok-pager-inspired: header, transcript, prompt).
fn draw_grok_tui(console: &mut SerialClient, status: &[u8], user: &[u8], reply: &[u8]) {
    write_bytes(console, b"\x1b[2J\x1b[H");
    write_bytes(console, b"\x1b[7m grok \x1b[0m ");
    write_bytes(console, status);
    write_bytes(console, b"\r\n--------------------------------\r\n");
    if !user.is_empty() {
        write_bytes(console, b"> ");
        write_bytes(console, user);
        write_bytes(console, b"\r\n");
    }
    if !reply.is_empty() {
        write_bytes(console, reply);
        write_bytes(console, b"\r\n");
    }
    write_bytes(console, b"--------------------------------\r\n> ");
}

fn handle_prompt(prompt: &[u8]) -> AgentResponse {
    let mut req = Vec::from(GROK_STUB_PROMPT);
    req.extend_from_slice(prompt);
    req.push(b'\n');

    let Ok(raw) = http_post(&req) else {
        log::info!("lerux-agent: complete failed");
        return AgentResponse::Error;
    };
    let Some(reply) = GrokStubReply::parse(&raw) else {
        log::info!("lerux-agent: bad stub");
        return AgentResponse::Error;
    };

    let GrokStubReply::ToolCall { kind, arg_len, arg } = reply else {
        return final_text(&reply);
    };
    let arg = &arg[..arg_len as usize];
    if let Ok(s) = core::str::from_utf8(kind.as_bytes())
        && let Ok(a) = core::str::from_utf8(arg)
    {
        log::info!("lerux-agent: tool {s} {a}");
    }

    let Some(result) = run_tool(kind, arg) else {
        log::info!("lerux-agent: tool failed");
        return AgentResponse::Error;
    };

    let mut req2 = Vec::from(GROK_STUB_PROMPT);
    req2.extend_from_slice(prompt);
    req2.push(b'\n');
    req2.extend_from_slice(GROK_STUB_TOOL_RESULT);
    req2.extend_from_slice(kind.as_bytes());
    req2.push(b' ');
    req2.extend_from_slice(arg);
    req2.push(b'\n');
    req2.extend_from_slice(result);

    let Ok(raw2) = http_post(&req2) else {
        log::info!("lerux-agent: complete2 failed");
        return AgentResponse::Error;
    };
    let Some(reply2) = GrokStubReply::parse(&raw2) else {
        log::info!("lerux-agent: bad stub2");
        return AgentResponse::Error;
    };
    final_text(&reply2)
}

fn final_text(reply: &GrokStubReply) -> AgentResponse {
    let text = reply.text();
    if text.is_empty() {
        return AgentResponse::Error;
    }
    if let Ok(s) = core::str::from_utf8(text) {
        log::info!("lerux-agent: {s}");
    }
    if text == AGENT_SMOKE_BODY {
        log::info!("lerux-agent: runtime ok");
    }
    AgentResponse::text(text)
}

/// Phase 77: only baked-in Read of `/hello.txt`. Phase 79 replaces this with FsRequest.
fn run_tool(kind: AgentToolKind, arg: &[u8]) -> Option<&'static [u8]> {
    match kind {
        AgentToolKind::Read if arg == AGENT_SMOKE_PATH => Some(AGENT_SMOKE_BODY),
        _ => None,
    }
}

fn http_post(body: &[u8]) -> Result<Vec<u8>, ()> {
    let client = HttpClient::new(REQUEST_SERVER);
    match client.call(HttpRequest::post(AGENT_GROK_ONE_URL)) {
        HttpResponse::Ok => {}
        _ => return Err(()),
    }
    for (i, chunk) in body.chunks(MAX_NET_TCP_PAYLOAD).enumerate() {
        let last = (i + 1) * MAX_NET_TCP_PAYLOAD >= body.len() || body.is_empty();
        match client.call(HttpRequest::body(chunk, last)) {
            HttpResponse::Ok => {}
            _ => {
                let _ = client.call(HttpRequest::Close);
                return Err(());
            }
        }
    }
    if body.is_empty() {
        match client.call(HttpRequest::body(b"", true)) {
            HttpResponse::Ok => {}
            _ => {
                let _ = client.call(HttpRequest::Close);
                return Err(());
            }
        }
    }
    match client.call(HttpRequest::Finish) {
        HttpResponse::Status { code: 200 } => {}
        _ => {
            let _ = client.call(HttpRequest::Close);
            return Err(());
        }
    }
    let mut out = Vec::new();
    loop {
        match client.call(HttpRequest::Recv) {
            HttpResponse::Data {
                data_len,
                data,
                last,
            } => {
                out.extend_from_slice(&data[..data_len as usize]);
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
    Ok(out)
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if !channels.contains(SERIAL_DRIVER) {
            return Ok(());
        }
        loop {
            match self.console.read() {
                Ok(b) => {
                    if b == b'\r' {
                        continue;
                    }
                    if b == b'\n' {
                        self.submit_line();
                        continue;
                    }
                    if b == 0x08 || b == 0x7f {
                        if self.line_len > 0 {
                            self.line_len -= 1;
                            write_bytes(&mut self.console, b"\x08 \x08");
                        }
                        continue;
                    }
                    if (32..127).contains(&b) && self.line_len < LINE_CAP {
                        self.line[self.line_len] = b;
                        self.line_len += 1;
                        write_bytes(&mut self.console, &[b]);
                    }
                }
                Err(nb::Error::WouldBlock) => break,
                Err(_) => break,
            }
        }
        Ok(())
    }

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != CLIENT {
            unreachable!();
        }
        Ok(match recv::<AgentRequest>(msg_info) {
            Ok(req) => send(handle_prompt(req.text())),
            Err(_) => send_unspecified_error(),
        })
    }
}
