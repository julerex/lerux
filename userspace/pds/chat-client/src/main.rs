//! UDP chat client with multi-room support (Phase 40 / 58).

#![no_std]
#![no_main]

use lerux_interface_types::{
    ChatRequest, ChatResponse, NetRequest, NetResponse, MAX_CHAT_LINES, MAX_CHAT_MSG,
    MAX_CHAT_ROOM, MAX_CHAT_ROOMS,
};
use lerux_ipc::{recv, send, send_unspecified_error, NetClient};
use lerux_logging::{debug, log};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};

const SHELL: Channel = Channel::new(0);
const NET_SERVER: NetClient = NetClient::new(Channel::new(1));

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
    fn push_line(&mut self, prefix: u8, room: &[u8], msg: &[u8]) {
        if self.count as usize >= MAX_CHAT_LINES {
            for i in 0..(MAX_CHAT_LINES - 1) {
                self.lines[i] = self.lines[i + 1];
                self.line_lens[i] = self.line_lens[i + 1];
            }
            self.count = (MAX_CHAT_LINES - 1) as u8;
        }
        let idx = self.count as usize;
        let mut line = [0u8; MAX_CHAT_MSG];
        // Format: `>room|msg` or `<room|msg`
        line[0] = prefix;
        let mut pos = 1usize;
        let rn = room.len().min(MAX_CHAT_ROOM).min(MAX_CHAT_MSG - 3);
        line[pos..pos + rn].copy_from_slice(&room[..rn]);
        pos += rn;
        if pos < MAX_CHAT_MSG {
            line[pos] = b'|';
            pos += 1;
        }
        let copy_len = msg.len().min(MAX_CHAT_MSG - pos);
        line[pos..pos + copy_len].copy_from_slice(&msg[..copy_len]);
        pos += copy_len;
        self.lines[idx] = line;
        self.line_lens[idx] = pos as u8;
        self.count += 1;
    }

    fn view(&self, room: &[u8]) -> ChatResponse {
        let mut rbuf = [0u8; MAX_CHAT_ROOM];
        let room_len = room.len().min(MAX_CHAT_ROOM) as u8;
        rbuf[..room_len as usize].copy_from_slice(&room[..room_len as usize]);
        ChatResponse::View {
            count: self.count,
            line_lens: self.line_lens,
            lines: self.lines,
            room_len,
            room: rbuf,
        }
    }
}

fn net_call(req: NetRequest) -> NetResponse {
    NET_SERVER.call(req)
}

#[protection_domain]
fn init() -> HandlerImpl {
    debug::init().expect("debug log init");
    log::info!("lerux-chat: ready rooms=lobby");
    let mut h = HandlerImpl {
        ring: ChatRing::default(),
        room_len: 5,
        room: *b"lobby\0\0\0",
        rooms: [[0u8; MAX_CHAT_ROOM]; MAX_CHAT_ROOMS],
        room_lens: [0u8; MAX_CHAT_ROOMS],
        n_rooms: 0,
    };
    h.remember_room(b"lobby");
    h
}

struct HandlerImpl {
    ring: ChatRing,
    room_len: u8,
    room: [u8; MAX_CHAT_ROOM],
    rooms: [[u8; MAX_CHAT_ROOM]; MAX_CHAT_ROOMS],
    room_lens: [u8; MAX_CHAT_ROOMS],
    n_rooms: u8,
}

impl HandlerImpl {
    fn room_bytes(&self) -> ([u8; MAX_CHAT_ROOM], u8) {
        (self.room, self.room_len)
    }

    fn remember_room(&mut self, room: &[u8]) {
        let n = room.len().min(MAX_CHAT_ROOM);
        for i in 0..self.n_rooms as usize {
            let rl = self.room_lens[i] as usize;
            if self.rooms[i][..rl] == room[..n] {
                return;
            }
        }
        if (self.n_rooms as usize) < MAX_CHAT_ROOMS {
            let i = self.n_rooms as usize;
            self.rooms[i][..n].copy_from_slice(&room[..n]);
            self.room_lens[i] = n as u8;
            self.n_rooms += 1;
        }
    }

    fn handle(&mut self, req: ChatRequest) -> ChatResponse {
        match req {
            ChatRequest::Join { room_len, room } => {
                let r = if room_len == 0 {
                    b"lobby".as_slice()
                } else {
                    &room[..room_len as usize]
                };
                let n = r.len().min(MAX_CHAT_ROOM);
                self.room = [0u8; MAX_CHAT_ROOM];
                self.room[..n].copy_from_slice(&r[..n]);
                self.room_len = n as u8;
                self.remember_room(r);
                let (rb, rl) = self.room_bytes();
                self.ring.push_line(b'*', &rb[..rl as usize], b"joined");
                self.ring.view(&rb[..rl as usize])
            }
            ChatRequest::Send { msg_len, msg } => {
                let text = &msg[..msg_len as usize];
                let (rb, rl) = self.room_bytes();
                self.ring.push_line(b'>', &rb[..rl as usize], text);
                // Wire format: room\0msg for multi-room demux on recv.
                let mut wire = [0u8; MAX_CHAT_MSG];
                let copy_r = (rl as usize).min(MAX_CHAT_MSG / 2);
                wire[..copy_r].copy_from_slice(&rb[..copy_r]);
                let mut pos = copy_r;
                if pos < MAX_CHAT_MSG {
                    wire[pos] = b'\0';
                    pos += 1;
                }
                let copy_m = text.len().min(MAX_CHAT_MSG - pos);
                wire[pos..pos + copy_m].copy_from_slice(&text[..copy_m]);
                pos += copy_m;
                let _ = net_call(NetRequest::udp_tx(&wire[..pos]));
                self.ring.view(&rb[..rl as usize])
            }
            ChatRequest::Recv => {
                if let NetResponse::UdpData { data_len, data } = net_call(NetRequest::UdpRecv) {
                    let raw = &data[..data_len as usize];
                    if let Some(sep) = raw.iter().position(|&b| b == 0) {
                        let room = &raw[..sep];
                        let msg = &raw[sep + 1..];
                        self.remember_room(room);
                        self.ring.push_line(b'<', room, msg);
                    } else {
                        let (rb, rl) = self.room_bytes();
                        self.ring.push_line(b'<', &rb[..rl as usize], raw);
                    }
                }
                let (rb, rl) = self.room_bytes();
                self.ring.view(&rb[..rl as usize])
            }
            ChatRequest::GetView => {
                let (rb, rl) = self.room_bytes();
                self.ring.view(&rb[..rl as usize])
            }
            ChatRequest::ListRooms => {
                let mut ring = ChatRing::default();
                for i in 0..self.n_rooms as usize {
                    let rl = self.room_lens[i] as usize;
                    ring.push_line(b'#', b"", &self.rooms[i][..rl]);
                }
                let (rb, rl) = self.room_bytes();
                ring.view(&rb[..rl as usize])
            }
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
