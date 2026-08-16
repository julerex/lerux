#![no_std]
#![no_main]

extern crate alloc;

use lerux_interface_types::{
    NetRequest, NetResponse, TlsRequest, TlsResponse, MAX_NET_TCP_PAYLOAD, MAX_TLS_NAME,
};
use lerux_ipc::{recv, send, send_unspecified_error, NetClient};
use lerux_logging::{debug, log};
use lerux_tls::{ClientSession, Status};
use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};

/// Channel IDs match `fetch-tls.system.template`.
const NET_SERVER: NetClient = NetClient::new(Channel::new(1));
const APP: Channel = Channel::new(2);

const MAX_STEPS: usize = 64;

struct HandlerImpl {
    session: Option<ClientSession>,
}

#[protection_domain(heap_size = 1024 * 1024)]
fn init() -> HandlerImpl {
    debug::init().unwrap();
    log::info!("lerux-tls: ready");
    HandlerImpl { session: None }
}

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if channel != APP {
            unreachable!();
        }

        Ok(match recv::<TlsRequest>(msg_info) {
            Ok(req) => send(self.handle_req(req)),
            Err(_) => send_unspecified_error(),
        })
    }
}

impl HandlerImpl {
    fn handle_req(&mut self, req: TlsRequest) -> TlsResponse {
        match req {
            TlsRequest::Connect {
                name_len,
                name,
                port,
            } => {
                self.close_session();
                let name_len = (name_len as usize).min(MAX_TLS_NAME);
                match connect_tls(&name[..name_len], port) {
                    Ok(session) => {
                        self.session = Some(session);
                        TlsResponse::Ok
                    }
                    Err(()) => TlsResponse::Error,
                }
            }
            TlsRequest::Send {
                payload_len,
                payload,
            } => match self.session.as_mut() {
                Some(session) => {
                    let payload_len =
                        (payload_len as usize).min(MAX_NET_TCP_PAYLOAD);
                    send_plain(session, &payload[..payload_len])
                }
                None => TlsResponse::Error,
            },
            TlsRequest::Recv => match self.session.as_mut() {
                Some(session) => recv_plain(session),
                None => TlsResponse::Error,
            },
            TlsRequest::Close => {
                self.close_session();
                TlsResponse::Ok
            }
            TlsRequest::Poll => TlsResponse::Pending,
        }
    }

    fn close_session(&mut self) {
        self.session = None;
        let _ = NET_SERVER.call(NetRequest::TcpClose);
    }
}

fn connect_tls(name: &[u8], port: u16) -> Result<ClientSession, ()> {
    let addr = match NET_SERVER.call(NetRequest::dns_resolve(name)) {
        NetResponse::Ipv4 { addr } => addr,
        _ => return Err(()),
    };
    match NET_SERVER.call(NetRequest::TcpConnect { addr, port }) {
        NetResponse::Ok => {}
        _ => return Err(()),
    }
    let name = core::str::from_utf8(name).map_err(|_| close_tcp())?;
    let mut session = ClientSession::new(name).map_err(|_| close_tcp())?;
    for _ in 0..MAX_STEPS {
        match session.drive().map_err(|_| close_tcp())? {
            Status::WantSend => tcp_send_all(&session.take_outgoing()).map_err(|_| close_tcp())?,
            Status::WantRecv => {
                let chunk = tcp_recv_one().map_err(|_| close_tcp())?;
                session.feed_incoming(&chunk);
            }
            Status::Connected => {
                log::info!("lerux-tls: handshake ok");
                return Ok(session);
            }
            Status::Closed => return Err(close_tcp()),
        }
    }
    Err(close_tcp())
}

fn close_tcp() -> () {
    let _ = NET_SERVER.call(NetRequest::TcpClose);
}

fn send_plain(session: &mut ClientSession, data: &[u8]) -> TlsResponse {
    if session.write_plain(data).is_err() {
        return TlsResponse::Error;
    }
    match tcp_send_all(&session.take_outgoing()) {
        Ok(()) => TlsResponse::Ok,
        Err(()) => TlsResponse::Error,
    }
}

fn recv_plain(session: &mut ClientSession) -> TlsResponse {
    let mut out = [0u8; MAX_NET_TCP_PAYLOAD];
    let n = session.read_plain(&mut out);
    if n > 0 {
        return tls_data(&out[..n]);
    }
    for _ in 0..MAX_STEPS {
        match tcp_recv_one() {
            Ok(chunk) => session.feed_incoming(&chunk),
            Err(()) => return TlsResponse::Error,
        }
        match session.drive() {
            Ok(Status::WantSend) => {
                if tcp_send_all(&session.take_outgoing()).is_err() {
                    return TlsResponse::Error;
                }
            }
            Ok(Status::WantRecv) => {}
            Ok(Status::Connected) => {
                let n = session.read_plain(&mut out);
                if n > 0 {
                    return tls_data(&out[..n]);
                }
            }
            Ok(Status::Closed) | Err(_) => return TlsResponse::Error,
        }
    }
    TlsResponse::Error
}

fn tls_data(bytes: &[u8]) -> TlsResponse {
    let mut data = [0u8; MAX_NET_TCP_PAYLOAD];
    let data_len = bytes.len().min(MAX_NET_TCP_PAYLOAD) as u16;
    data[..data_len as usize].copy_from_slice(&bytes[..data_len as usize]);
    TlsResponse::Data { data_len, data }
}

fn tcp_send_all(bytes: &[u8]) -> Result<(), ()> {
    if bytes.is_empty() {
        return Ok(());
    }
    for chunk in bytes.chunks(MAX_NET_TCP_PAYLOAD) {
        match NET_SERVER.call(NetRequest::tcp_send(chunk)) {
            NetResponse::Ok => {}
            _ => return Err(()),
        }
    }
    Ok(())
}

fn tcp_recv_one() -> Result<alloc::vec::Vec<u8>, ()> {
    match NET_SERVER.call(NetRequest::TcpRecv) {
        NetResponse::TcpData { data_len, data } => Ok(data[..data_len as usize].to_vec()),
        _ => Err(()),
    }
}
