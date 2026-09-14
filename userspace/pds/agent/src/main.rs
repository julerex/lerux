#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;

use embedded_hal_nb::{
    nb,
    serial::{Read as _, Write as _},
};
#[cfg(feature = "browse")]
use lerux_interface_types::AGENT_INTERACTIVE_PROMPT;
#[cfg(all(feature = "tools", not(feature = "browse")))]
use lerux_interface_types::AGENT_TOOLS_PROMPT;
use lerux_interface_types::{
    AgentRequest, AgentResponse, AgentToolKind, GrokStubReply, HttpRequest, HttpResponse,
    AGENT_GROK_ONE_URL, AGENT_INTERACTIVE_OK, AGENT_SMOKE_BODY, AGENT_TOOLS_OK, GROK_STUB_PROMPT,
    GROK_STUB_TOOL_RESULT, MAX_AGENT_PROMPT, MAX_AGENT_TEXT, MAX_NET_TCP_PAYLOAD,
};
#[cfg(not(feature = "tools"))]
use lerux_interface_types::{AGENT_SMOKE_PATH, AGENT_SMOKE_PROMPT};
use lerux_ipc::{recv, send, send_unspecified_error, HttpClient};
#[cfg(feature = "browse")]
use lerux_logging::debug;
use lerux_logging::log;
#[cfg(not(feature = "browse"))]
use lerux_logging::serial;
use sel4_microkit::{protection_domain, Channel, ChannelSet, Handler, Infallible, MessageInfo};
use sel4_microkit_driver_adapters::serial::client::Client as SerialClient;

#[cfg(feature = "tools")]
mod tools;

/// Channel 0: serial-driver (`<end pd="agent" id="0" pp="true" />`).
const SERIAL_DRIVER: Channel = Channel::new(0);
/// Channel 1: request-server (`<end pd="agent" id="1" pp="true" />`).
pub(crate) const REQUEST_SERVER: Channel = Channel::new(1);
/// Channel 2: shell `grok` (`<end pd="agent" id="2" />`). Unwired on v1 smoke.
const CLIENT: Channel = Channel::new(2);
/// Channel 3: fs-server (`<end pd="agent" id="3" pp="true" />`).
#[cfg(feature = "tools")]
pub(crate) const FS_SERVER: Channel = Channel::new(3);
/// Channel 4: web-content Browse (`<end pd="agent" id="4" pp="true" />`).
#[cfg(feature = "browse")]
pub(crate) const WEB_CONTENT: Channel = Channel::new(4);

const LINE_CAP: usize = MAX_AGENT_PROMPT;

#[protection_domain(heap_size = 64 * 1024)]
fn init() -> HandlerImpl {
    #[cfg(feature = "browse")]
    debug::init().unwrap();
    #[cfg(not(feature = "browse"))]
    serial::init(SERIAL_DRIVER).unwrap();
    log::info!("lerux-agent: ready");

    #[cfg_attr(
        feature = "browse",
        expect(unused_mut, reason = "joint profile skips the serial TUI")
    )]
    let mut h = HandlerImpl {
        console: SerialClient::new(SERIAL_DRIVER),
        line: [0u8; LINE_CAP],
        line_len: 0,
        last_user: [0u8; MAX_AGENT_PROMPT],
        last_user_len: 0,
        last_reply: [0u8; MAX_AGENT_TEXT],
        last_reply_len: 0,
    };
    #[cfg(not(feature = "browse"))]
    {
        h.draw(b"stub");
        log::info!("lerux-agent: chrome ok");
    }

    #[cfg(feature = "tools")]
    tools::seed_workspace();

    let prompt = smoke_prompt();
    match handle_prompt(prompt) {
        AgentResponse::Text { text_len, text } => {
            #[cfg(not(feature = "browse"))]
            {
                h.set_exchange(prompt, &text[..text_len as usize]);
                h.draw(b"stub");
            }
            #[cfg(feature = "browse")]
            let _ = (text_len, text);
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

fn smoke_prompt() -> &'static [u8] {
    #[cfg(feature = "browse")]
    {
        AGENT_INTERACTIVE_PROMPT
    }
    #[cfg(all(feature = "tools", not(feature = "browse")))]
    {
        AGENT_TOOLS_PROMPT
    }
    #[cfg(not(feature = "tools"))]
    {
        AGENT_SMOKE_PROMPT
    }
}

fn handle_prompt(prompt: &[u8]) -> AgentResponse {
    let mut conv = Vec::from(GROK_STUB_PROMPT);
    conv.extend_from_slice(prompt);
    conv.push(b'\n');
    for _ in 0..8 {
        let Ok(raw) = http_post(&conv) else {
            log::info!("lerux-agent: complete failed");
            return AgentResponse::Error;
        };
        let Some(reply) = GrokStubReply::parse(&raw) else {
            log::info!("lerux-agent: bad stub");
            return AgentResponse::Error;
        };
        match reply {
            GrokStubReply::Text { .. } => return final_text(&reply),
            GrokStubReply::ToolCall { kind, arg_len, arg } => {
                let arg = &arg[..arg_len as usize];
                if let Ok(s) = core::str::from_utf8(kind.as_bytes())
                    && let Ok(a) = core::str::from_utf8(arg)
                {
                    log::info!("lerux-agent: tool {s} {a}");
                }
                let Ok(result) = run_tool(kind, arg) else {
                    log::info!("lerux-agent: tool failed");
                    return AgentResponse::Error;
                };
                conv.extend_from_slice(GROK_STUB_TOOL_RESULT);
                conv.extend_from_slice(kind.as_bytes());
                conv.push(b' ');
                conv.extend_from_slice(arg);
                conv.push(b'\n');
                conv.extend_from_slice(&result);
                conv.push(b'\n');
            }
        }
    }
    AgentResponse::Error
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
    if text == AGENT_TOOLS_OK {
        log::info!("lerux-agent: tools ok");
    }
    if text == AGENT_INTERACTIVE_OK {
        log::info!("lerux-agent: interactive ok");
    }
    AgentResponse::text(text)
}

fn run_tool(kind: AgentToolKind, arg: &[u8]) -> Result<Vec<u8>, ()> {
    #[cfg(feature = "tools")]
    {
        tools::run_tool(kind, arg)
    }
    #[cfg(not(feature = "tools"))]
    {
        match kind {
            AgentToolKind::Read if arg == AGENT_SMOKE_PATH => Ok(AGENT_SMOKE_BODY.to_vec()),
            _ => Err(()),
        }
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
