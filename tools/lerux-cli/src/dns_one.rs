//! Resolver for the browser smoke.
//!
//! Listens on `127.0.0.1:53`. The name `host` gets one A record for the QEMU
//! user-net gateway. Every other name is NXDOMAIN. libslirp forwards guest
//! packets for `10.0.2.3:53` to the first nameserver in the QEMU process's
//! `resolv.conf`. The launcher bind-mounts that file, so the rest of the
//! machine keeps its own resolver.
//!
//! Binding port 53 needs a privileged helper. The parent keeps the helper's
//! stdin open and closes it to stop the server, including when the helper
//! was re-executed as root.

use std::{
    io::{self, Read},
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use anyhow::{bail, Context, Result};

const PORT: u16 = 53;
const GATEWAY: [u8; 4] = [10, 0, 2, 2];

pub(crate) fn dns_one(root: bool) -> Result<()> {
    match UdpSocket::bind((Ipv4Addr::LOCALHOST, PORT)) {
        Ok(sock) => serve(&sock),
        Err(err) if !root && err.kind() == io::ErrorKind::PermissionDenied => sudo_dns(),
        Err(err) => Err(err).context("bind 127.0.0.1:53"),
    }
}

pub(crate) fn start_dns_one_background() -> Result<std::process::Child> {
    let mut child = Command::new(std::env::current_exe().context("current exe")?)
        .arg("dns-one")
        .stdin(Stdio::piped())
        .spawn()
        .context("spawn dns-one")?;
    wait_until_serving(&mut child)?;
    Ok(child)
}

fn sudo_dns() -> Result<()> {
    let status = Command::new("sudo")
        .arg("-n")
        .arg(std::env::current_exe().context("current exe")?)
        .args(["dns-one", "--root"])
        .stdin(Stdio::inherit())
        .status()
        .context("sudo dns-one")?;
    if status.success() {
        Ok(())
    } else {
        bail!("dns-one stopped: {status}")
    }
}

fn serve(sock: &UdpSocket) -> Result<()> {
    eprintln!("dns-one: listening on 127.0.0.1:{PORT}");
    sock.set_read_timeout(Some(Duration::from_millis(200)))
        .context("dns-one timeout")?;
    let stop = watch_stdin();
    let mut buf = [0u8; 512];
    while !stop.load(Ordering::Relaxed) {
        match sock.recv_from(&mut buf) {
            Ok((n, from)) => send_reply(sock, from, &buf[..n])?,
            Err(err) if timed_out(&err) => {}
            Err(err) => return Err(err).context("dns-one recv"),
        }
    }
    Ok(())
}

fn timed_out(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
    )
}

fn watch_stdin() -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    thread::spawn(move || {
        let mut buf = [0u8; 16];
        loop {
            match io::stdin().read(&mut buf) {
                Ok(0) | Err(_) => {
                    flag.store(true, Ordering::Relaxed);
                    break;
                }
                Ok(_) => {}
            }
        }
    });
    stop
}

fn send_reply(sock: &UdpSocket, from: SocketAddr, query: &[u8]) -> Result<()> {
    let Some(packet) = dns_reply(query) else {
        return Ok(());
    };
    sock.send_to(&packet, from).context("dns-one send")?;
    Ok(())
}

fn wait_until_serving(child: &mut std::process::Child) -> Result<()> {
    let probe = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).context("dns probe")?;
    probe
        .set_read_timeout(Some(Duration::from_millis(100)))
        .context("dns probe timeout")?;
    let query = host_query();
    for _ in 0..50 {
        if let Some(status) = child.try_wait().context("dns-one status")? {
            bail!("dns-one exited {status}");
        }
        let _ = probe.send_to(&query, (Ipv4Addr::LOCALHOST, PORT));
        let mut buf = [0u8; 512];
        if let Ok((n, _)) = probe.recv_from(&mut buf)
            && answers_gateway(&buf[..n])
        {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    bail!("dns-one did not answer")
}

fn answers_gateway(packet: &[u8]) -> bool {
    let code = packet.get(3).copied().unwrap_or(0xff) & 0x0f;
    code == 0 && packet.ends_with(&GATEWAY)
}

fn host_query() -> Vec<u8> {
    query_for(b"host")
}

fn query_for(name: &[u8]) -> Vec<u8> {
    let mut out = vec![0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
    out.push(u8::try_from(name.len()).unwrap_or(0));
    out.extend_from_slice(name);
    out.extend_from_slice(&[0, 0x00, 0x01, 0x00, 0x01]);
    out
}

/// Build a reply. `None` means the query is too short to answer.
pub(crate) fn dns_reply(query: &[u8]) -> Option<Vec<u8>> {
    let qend = question_end(query)?;
    let name = question_name(query)?;
    let qtype = u16::from_be_bytes([query[qend - 4], query[qend - 3]]);
    let qclass = u16::from_be_bytes([query[qend - 2], query[qend - 1]]);
    if qclass == 1 && qtype == 1 && name.eq_ignore_ascii_case(b"host") {
        Some(a_reply(query, qend))
    } else {
        Some(error_reply(query, qend, 3))
    }
}

fn question_end(query: &[u8]) -> Option<usize> {
    if query.len() < 12 || u16::from_be_bytes([query[4], query[5]]) != 1 {
        return None;
    }
    let mut at = 12usize;
    for _ in 0..16 {
        let len = usize::from(*query.get(at)?);
        if len == 0 {
            let end = at.checked_add(5)?;
            return (end <= query.len()).then_some(end);
        }
        if len >= 64 {
            return None;
        }
        at = at.checked_add(1 + len)?;
    }
    None
}

fn question_name(query: &[u8]) -> Option<Vec<u8>> {
    let mut at = 12usize;
    let mut name = Vec::new();
    for _ in 0..16 {
        let len = usize::from(*query.get(at)?);
        if len == 0 {
            return Some(name);
        }
        if len >= 64 {
            return None;
        }
        let start = at.checked_add(1)?;
        let end = start.checked_add(len)?;
        let label = query.get(start..end)?;
        if !name.is_empty() {
            name.push(b'.');
        }
        name.extend_from_slice(label);
        at = end;
    }
    None
}

fn a_reply(query: &[u8], qend: usize) -> Vec<u8> {
    let mut out = header(query, 1, 0);
    out.extend_from_slice(&query[12..qend]);
    out.extend_from_slice(&[
        0xc0, 0x0c, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 60, 0x00, 0x04,
    ]);
    out.extend_from_slice(&GATEWAY);
    out
}

fn error_reply(query: &[u8], qend: usize, rcode: u8) -> Vec<u8> {
    let mut out = header(query, 0, rcode);
    out.extend_from_slice(&query[12..qend]);
    out
}

fn header(query: &[u8], answers: u8, rcode: u8) -> Vec<u8> {
    vec![
        query[0],
        query[1],
        0x84 | (query[2] & 0x01),
        0x80 | (rcode & 0x0f),
        query[4],
        query[5],
        0,
        answers,
        0,
        0,
        0,
        0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_answers_with_the_query_id() {
        let reply = dns_reply(&host_query()).expect("reply");
        assert_eq!(&reply[..2], &[0x12, 0x34]);
        assert_eq!(reply[3] & 0x0f, 0);
        assert!(answers_gateway(&reply));
    }

    #[test]
    fn other_names_are_nxdomain() {
        let reply = dns_reply(&query_for(b"other")).expect("reply");
        assert_eq!(reply[3] & 0x0f, 3);
        assert!(!answers_gateway(&reply));
    }

    #[test]
    fn uppercase_host_is_the_same_name() {
        let reply = dns_reply(&query_for(b"HOST")).expect("reply");
        assert!(answers_gateway(&reply));
    }

    #[test]
    fn short_packet_has_no_reply() {
        assert!(dns_reply(&[0, 1, 0, 0]).is_none());
    }
}
