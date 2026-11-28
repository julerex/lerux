//! Host checks for `socket`, `connect`, `listen`, `accept`, `send`, and `recv`.
//!
//! The fake pipe accepts `10.0.2.2:8443` and returns the fixture mark after
//! it sees a `GET`. The listener pipe accepts one inbound request on port 8080.

use std::sync::{Arc, Mutex};

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
fn close_lets_the_next_socket_connect() {
    let mut sockets = Sockets::new(Pipe::open());
    let fd = sockets.socket().expect("first socket");
    sockets.connect(fd, HOST, PORT).expect("connect");
    sockets.close(fd).expect("close");
    let fd = sockets.socket().expect("socket after close");
    sockets.connect(fd, HOST, PORT).expect("second connect");
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

struct Listener {
    open: bool,
    stalled: bool,
    inbox: &'static [u8],
    pos: usize,
    sent: Arc<Mutex<Vec<u8>>>,
}

impl Listener {
    fn ready(sent: Arc<Mutex<Vec<u8>>>) -> Self {
        Self {
            open: false,
            stalled: false,
            inbox: b"GET / HTTP/1.1\r\n\r\n",
            pos: 0,
            sent,
        }
    }

    fn stalled() -> Self {
        Self {
            open: false,
            stalled: true,
            inbox: b"",
            pos: 0,
            sent: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Tcp for Listener {
    fn start(&mut self) {}

    fn poll(&mut self) -> bool {
        if self.stalled {
            return false;
        }
        self.open = true;
        true
    }

    fn connect(&mut self, _addr: [u8; 4], _port: u16) -> Result<(), Errno> {
        Err(Errno(Errno::EOPNOTSUPP))
    }

    fn listen(&mut self, port: u16) -> Result<(), Errno> {
        if port != 8080 {
            return Err(Errno(Errno::EADDRINUSE));
        }
        Ok(())
    }

    fn ready(&self) -> bool {
        self.open
    }

    fn failed(&self) -> bool {
        false
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, Errno> {
        self.sent
            .lock()
            .expect("sent buffer")
            .extend_from_slice(data);
        Ok(data.len())
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Errno> {
        if self.pos >= self.inbox.len() {
            return Err(Errno(Errno::EAGAIN));
        }
        let n = (self.inbox.len() - self.pos).min(buf.len());
        buf[..n].copy_from_slice(&self.inbox[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }

    fn shutdown(&mut self) -> Result<bool, Errno> {
        Ok(true)
    }
}

#[test]
fn listen_accept_send_shutdown_passes_the_body() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let mut sockets = Sockets::new(Listener::ready(Arc::clone(&sent)));
    let fd = sockets.socket().expect("socket");
    sockets.listen(fd, 8080).expect("listen");
    let err = sockets.socket().expect_err("second socket");
    assert_eq!(err, Errno(Errno::EMFILE));
    sockets.accept(fd).expect("accept");
    let mut buf = [0u8; 32];
    let n = sockets.recv(fd, &mut buf).expect("recv");
    assert_eq!(&buf[..n], b"GET / HTTP/1.1\r\n\r\n");
    let wrote = sockets.send(fd, b"lerux").expect("send");
    assert_eq!(wrote, 5);
    sockets.shutdown(fd).expect("shutdown");
    assert_eq!(sent.lock().expect("sent buffer").as_slice(), b"lerux");
}

#[test]
fn listen_on_a_connect_pipe_is_eopnotsupp() {
    let mut sockets = Sockets::new(Pipe::open());
    let fd = sockets.socket().expect("socket");
    let err = sockets.listen(fd, 8080).expect_err("listen");
    assert_eq!(err, Errno(Errno::EOPNOTSUPP));
}

#[test]
fn accept_timeout_is_etimedout() {
    let mut sockets = Sockets::new(Listener::stalled());
    let fd = sockets.socket().expect("socket");
    sockets.listen(fd, 8080).expect("listen");
    let err = sockets.accept(fd).expect_err("accept");
    assert_eq!(err, Errno(Errno::ETIMEDOUT));
}

#[test]
fn send_while_listening_is_enotconn() {
    let mut sockets = Sockets::new(Listener::stalled());
    let fd = sockets.socket().expect("socket");
    sockets.listen(fd, 8080).expect("listen");
    let err = sockets.send(fd, b"x").expect_err("send");
    assert_eq!(err, Errno(Errno::ENOTCONN));
}
