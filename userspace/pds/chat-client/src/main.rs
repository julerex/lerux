#![no_std]
#![no_main]

use lerux_interface_types::{
    ChatRequest, ChatResponse, NetRequest, NetResponse, MAX_CHAT_LINES, MAX_CHAT_MSG,
};
use lerux_ipc::{call, recv, send, send_unspecified_error};
use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};

const SHELL: Channel = Channel::new(0);
const NET_SERVER: Channel = Channel::new(1);

struct ChatRing {
    count: u8,
    line_lens: [u8; MAX_CHAT_LINES],
    lines: [[u8; MAX_CHAT_MSG]; MAX_CHAT_LINES],
}

impl Default for ChatRing {
    fn default() -> Self {
        Self {
            count: 0,
            line_lens: [0; MAX_CHAT_LINES],
            lines: [[0; MAX_CHAT_MSG]; MAX_CHAT_LINES],
        }
    }
}

impl ChatRing {
    fn push_line(&mut self, prefix: u8, msg: &[u8]) {
        if self.count as usize >= MAX_CHAT_LINES {
            for i in 0..(MAX_CHAT_LINES - 1) {
                self.lines[i] = self.lines[i + 1];
                self.line_lens[i] = self.line_lens[i + 1];
            }
            self.count = (MAX_CHAT_LINES - 1) as u8;
        }
        let idx = self.count as usize;
        let mut line = [0u8; MAX_CHAT_MSG];
        line[0] = prefix;
        let copy_len = msg.len().min(MAX_CHAT_MSG - 1);
        line[1..1 + copy_len].copy_from_slice(&msg[..copy_len]);
        self.lines[idx] = line;
        self.line_lens[idx] = (1 + copy_len) as u8;
        self.count += 1;
    }

    fn view(&self) -> ChatResponse {
        ChatResponse::View {
            count: self.count,
            line_lens: self.line_lens,
            lines: self.lines,
        }
    }
}

fn poll_net() -> NetResponse {
    loop {
        match call::<NetRequest, NetResponse>(NET_SERVER, NetRequest::Poll) {
            Ok(NetResponse::Pending) => {}
            Ok(other) => return other,
            Err(_) => return NetResponse::Error,
        }
    }
}

fn net_call(req: NetRequest) -> NetResponse {
    match call::<NetRequest, NetResponse>(NET_SERVER, req) {
        Ok(NetResponse::Pending) => poll_net(),
        Ok(other) => other,
        Err(_) => NetResponse::Error,
    }
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug log init");
    log::info!("lerux-chat: ready");
    HandlerImpl {
        ring: ChatRing::default(),
    }
}

struct HandlerImpl {
    ring: ChatRing,
}

impl HandlerImpl {
    fn handle(&mut self, req: ChatRequest) -> ChatResponse {
        match req {
            ChatRequest::Send { msg_len, msg } => {
                let text = &msg[..msg_len as usize];
                self.ring.push_line(b'>', text);
                let _ = net_call(NetRequest::udp_tx(text));
                self.ring.view()
            }
            ChatRequest::Recv => {
                if let NetResponse::UdpData { data_len, data } = net_call(NetRequest::UdpRecv) {
                    self.ring.push_line(b'<', &data[..data_len as usize]);
                }
                self.ring.view()
            }
            ChatRequest::GetView => self.ring.view(),
            ChatRequest::Quit => ChatResponse::Ok,
        }
    }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != SHELL {
            return Ok(send_unspecified_error());
        }

        Ok(match recv::<ChatRequest>(msg_info) {
            Ok(req) => send(self.handle(req)),
            Err(_) => send_unspecified_error(),
        })
    }
}
