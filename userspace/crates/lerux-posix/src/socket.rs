//! One Transmission Control Protocol socket.
//!
//! [`Sockets`] owns the call order. The caller owns the stack through [`Tcp`].
//! A second `socket` returns errno 24. `send` and `recv` run only while the
//! connection is open.

use super::Errno;

/// Local socket number. This table is not the file table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SockFd(u8);

impl SockFd {
    /// The one socket this client stores.
    pub fn number(self) -> i32 {
        i32::from(self.0)
    }
}

/// Byte pipe for one connection.
///
/// `start` begins the time budget for the call that follows. `poll` returns
/// false when that budget is spent. `recv` returns [`Errno::EAGAIN`] when the
/// peer is still open and no byte is waiting, and `Ok(0)` when the peer closed.
pub trait Tcp {
    fn start(&mut self);
    fn poll(&mut self) -> bool;
    fn connect(&mut self, addr: [u8; 4], port: u16) -> Result<(), Errno>;
    fn ready(&self) -> bool;
    fn failed(&self) -> bool;
    fn send(&mut self, data: &[u8]) -> Result<usize, Errno>;
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Errno>;
}

enum Phase {
    Empty,
    Idle,
    Connecting,
    Open,
    Closed,
}

/// `socket`, `connect`, `send`, and `recv` for one [`Tcp`] pipe.
pub struct Sockets<T: Tcp> {
    tcp: T,
    phase: Phase,
    fd: u8,
}

impl<T: Tcp> Sockets<T> {
    /// Wrap a pipe that has not been connected.
    pub fn new(tcp: T) -> Self {
        Self {
            tcp,
            phase: Phase::Empty,
            fd: 3,
        }
    }

    /// Allocate the one socket. The number is 3.
    pub fn socket(&mut self) -> Result<SockFd, Errno> {
        if !matches!(self.phase, Phase::Empty) {
            return Err(Errno(Errno::EMFILE));
        }
        self.phase = Phase::Idle;
        Ok(SockFd(self.fd))
    }

    /// Connect `fd` and wait until the pipe is ready.
    pub fn connect(&mut self, fd: SockFd, addr: [u8; 4], port: u16) -> Result<(), Errno> {
        self.live(fd)?;
        match self.phase {
            Phase::Idle => {}
            Phase::Open | Phase::Connecting => return Err(Errno(Errno::EISCONN)),
            Phase::Empty | Phase::Closed => return Err(Errno(Errno::ENOTCONN)),
        }
        self.tcp.start();
        self.tcp.connect(addr, port)?;
        self.phase = Phase::Connecting;
        loop {
            if self.tcp.ready() {
                self.phase = Phase::Open;
                return Ok(());
            }
            if self.tcp.failed() {
                self.phase = Phase::Closed;
                return Err(Errno(Errno::ECONNREFUSED));
            }
            if !self.tcp.poll() {
                self.phase = Phase::Closed;
                return Err(Errno(Errno::ETIMEDOUT));
            }
        }
    }

    /// Send `data` on an open socket. An empty slice sends nothing.
    pub fn send(&mut self, fd: SockFd, mut data: &[u8]) -> Result<usize, Errno> {
        self.open(fd)?;
        if data.is_empty() {
            return Ok(0);
        }
        self.tcp.start();
        let mut sent = 0;
        while !data.is_empty() {
            match self.tcp.send(data) {
                Ok(0) | Err(Errno(Errno::EAGAIN)) => {
                    if !self.tcp.poll() {
                        return Err(Errno(Errno::ETIMEDOUT));
                    }
                }
                Ok(n) => {
                    let n = n.min(data.len());
                    data = &data[n..];
                    sent += n;
                }
                Err(err) => return Err(err),
            }
        }
        Ok(sent)
    }

    /// Receive at least one byte, or `Ok(0)` when the peer has closed.
    pub fn recv(&mut self, fd: SockFd, buf: &mut [u8]) -> Result<usize, Errno> {
        self.open(fd)?;
        if buf.is_empty() {
            return Ok(0);
        }
        self.tcp.start();
        loop {
            match self.tcp.recv(buf) {
                Ok(0) => {
                    self.phase = Phase::Closed;
                    return Ok(0);
                }
                Ok(n) => return Ok(n),
                Err(Errno(Errno::EAGAIN)) => {
                    if !self.tcp.poll() {
                        return Err(Errno(Errno::ETIMEDOUT));
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }

    fn live(&self, fd: SockFd) -> Result<(), Errno> {
        if fd.0 == self.fd {
            Ok(())
        } else {
            Err(Errno(Errno::EBADF))
        }
    }

    fn open(&self, fd: SockFd) -> Result<(), Errno> {
        self.live(fd)?;
        if matches!(self.phase, Phase::Open) {
            Ok(())
        } else {
            Err(Errno(Errno::ENOTCONN))
        }
    }
}
