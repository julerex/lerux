#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;

use lerux_interface_types::{
    AgentRequest, AgentResponse, AgentToolKind, GrokStubReply, HttpRequest, HttpResponse,
    AGENT_GROK_ONE_URL, AGENT_SMOKE_BODY, AGENT_SMOKE_PATH, AGENT_SMOKE_PROMPT, GROK_STUB_PROMPT,
    GROK_STUB_TOOL_RESULT, MAX_NET_TCP_PAYLOAD,
};
use lerux_ipc::{recv, send, send_unspecified_error, HttpClient};
use lerux_logging::{log, serial};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};

/// Channel 0: serial-driver (`<end pd="agent" id="0" pp="true" />`).
const SERIAL_DRIVER: Channel = Channel::new(0);
/// Channel 1: request-server (`<end pd="agent" id="1" pp="true" />`).
const REQUEST_SERVER: Channel = Channel::new(1);

const CLIENT: Channel = Channel::new(2);

#[protection_domain(heap_size = 64 * 1024)]
fn init() -> HandlerImpl {
    serial::init(SERIAL_DRIVER).unwrap();
    log::info!("lerux-agent: ready");
    run_smoke();
    HandlerImpl
}

struct HandlerImpl;

fn run_smoke() {
    match handle_prompt(AGENT_SMOKE_PROMPT) {
        AgentResponse::Text { .. } => {}
        _ => panic!("agent smoke prompt"),
    }
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
