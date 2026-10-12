//! Host checks for `socket`, `connect`, `send`, and `recv`.
//!
//! The fake pipe accepts `10.0.2.2:8443` and returns the fixture mark after
//! it sees a `GET`.

use lerux_posix::{Errno, Sockets, Tcp};

const HOST: [u8; 4] = [10, 0, 2, 2];
const PORT: u16 = 8443;
const MARK: &[u8] = b"lerux-http-fixture";

struct Pipe {
    polls: u32,
    limit: u32,
    ready: bool,
    failed: bool,
    sent: Vec<u8>,
    inbox: Vec<u8>,
    pos: usize,
}

impl Pipe {
    fn open() -> Self {
        Self {
            polls: 0,
            limit: 4,
            ready: false,
            failed: false,
            sent: Vec::new(),
            inbox: Vec::new(),
            pos: 0,
        }
    }

    fn stalled() -> Self {
        let mut pipe = Self::open();
        pipe.limit = 0;
        pipe
    }
}

impl Tcp for Pipe {
    fn start(&mut self) {
        self.polls = 0;
    }

    fn poll(&mut self) -> bool {
        if self.polls >= self.limit {
            return false;
        }
        self.polls += 1;
        self.ready = true;
        true
    }

    fn connect(&mut self, addr: [u8; 4], port: u16) -> Result<(), Errno> {
        if addr != HOST || port != PORT {
            self.failed = true;
            return Err(Errno(Errno::ECONNREFUSED));
        }
        Ok(())
    }

    fn ready(&self) -> bool {
        self.ready
    }

    fn failed(&self) -> bool {
        self.failed
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, Errno> {
        self.sent.extend_from_slice(data);
        if self.sent.windows(4).any(|window| window == b"GET ") {
            self.inbox = MARK.to_vec();
        }
        Ok(data.len())
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Errno> {
        if self.pos >= self.inbox.len() {
            return Err(Errno(Errno::EAGAIN));
        }
        let n = (self.inbox.len() - self.pos).min(buf.len());
        buf[..n].copy_from_slice(&self.inbox[self.pos..self.pos + n]);
        self.pos += n;
        if self.pos == self.inbox.len() {
            self.inbox.clear();
            self.pos = 0;
        }
        Ok(n)
    }
}

#[test]
fn socket_connect_send_recv_round_trip() {
    let mut sockets = Sockets::new(Pipe::open());
    let fd = sockets.socket().expect("first socket");
    assert_eq!(fd.number(), 3);
    sockets.connect(fd, HOST, PORT).expect("connect");
    let sent = sockets
        .send(fd, b"GET /fixture.html HTTP/1.1\r\n\r\n")
        .expect("send");
    assert_eq!(sent, b"GET /fixture.html HTTP/1.1\r\n\r\n".len());
    let mut buf = [0u8; 64];
    let n = sockets.recv(fd, &mut buf).expect("recv");
    assert_eq!(&buf[..n], MARK);
}

#[test]
fn second_socket_is_emfile() {
    let mut sockets = Sockets::new(Pipe::open());
    sockets.socket().expect("first socket");
    let err = sockets.socket().expect_err("second socket");
    assert_eq!(err, Errno(Errno::EMFILE));
}

#[test]
fn send_before_connect_is_enotconn() {
    let mut sockets = Sockets::new(Pipe::open());
    let fd = sockets.socket().expect("socket");
    let err = sockets.send(fd, b"x").expect_err("send");
    assert_eq!(err, Errno(Errno::ENOTCONN));
}

#[test]
fn connect_timeout_is_etimedout() {
    let mut sockets = Sockets::new(Pipe::stalled());
    let fd = sockets.socket().expect("socket");
    let err = sockets.connect(fd, HOST, PORT).expect_err("connect");
    assert_eq!(err, Errno(Errno::ETIMEDOUT));
}

#[test]
fn second_connect_is_eisconn() {
    let mut sockets = Sockets::new(Pipe::open());
    let fd = sockets.socket().expect("socket");
    sockets.connect(fd, HOST, PORT).expect("connect");
    let err = sockets.connect(fd, HOST, PORT).expect_err("second connect");
    assert_eq!(err, Errno(Errno::EISCONN));
}
