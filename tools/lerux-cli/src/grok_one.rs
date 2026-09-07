//! Scripted HTTPS completions stub for the Phase 77 agent runtime smoke
//! (port 8444 by default). Not live xAI.
//!
//! Speaks the line protocol in `lerux-interface-types` (`PROMPT` / `TOOL_CALL` /
//! `TOOL_RESULT` / `TEXT`) over the smoke CA. Serves two connections then
//! exits (prompt → tool-call, then tool-result → final text).

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use lerux_interface_types::{
    AGENT_SMOKE_BODY, AGENT_SMOKE_PATH, AGENT_SMOKE_PROMPT, GROK_STUB_TEXT, GROK_STUB_TOOL_CALL,
    GROK_STUB_TOOL_RESULT,
};
use rustls::ServerConnection;

use crate::https_one::server_config;

const TURNS: usize = 2;

pub fn grok_one(port: u16) -> Result<()> {
    let config = server_config()?;
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
        .with_context(|| format!("bind 127.0.0.1:{port}"))?;
    eprintln!("grok-one-server: listening on 127.0.0.1:{port}");

    let done = Arc::new(AtomicUsize::new(0));
    let done_thread = Arc::clone(&done);
    let handle = thread::spawn(move || {
        for _ in 0..TURNS {
            match listener.accept() {
                Ok((mut sock, peer)) => {
                    eprintln!("grok-one-server: accepted {peer}");
                    if let Err(e) = serve_one(&config, &mut sock) {
                        eprintln!("grok-one-server: {e:#}");
                    }
                    let _ = sock.shutdown(std::net::Shutdown::Write);
                    done_thread.fetch_add(1, Ordering::SeqCst);
                }
                Err(e) => {
                    eprintln!("grok-one-server: accept {e}");
                    break;
                }
            }
        }
    });

    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    while done.load(Ordering::SeqCst) < TURNS {
        if std::time::Instant::now() >= deadline {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let _ = handle.join();
    Ok(())
}

pub fn start_grok_one_background(port: u16) -> Result<std::process::Child> {
    let child = std::process::Command::new(std::env::current_exe()?)
        .arg("grok-one")
        .arg(port.to_string())
        .spawn()
        .context("spawn grok-one")?;
    crate::tcp_echo::wait_for_port(port, 100);
    Ok(child)
}

fn serve_one(config: &Arc<rustls::ServerConfig>, sock: &mut std::net::TcpStream) -> Result<()> {
    sock.set_nodelay(true).ok();
    let mut conn = ServerConnection::new(Arc::clone(config)).context("ServerConnection")?;
    let mut raw = [0u8; 4096];
    let mut request = Vec::new();
    let mut saw_http = false;
    for _ in 0..64 {
        if conn.wants_write() {
            let mut out = Vec::new();
            conn.write_tls(&mut out).context("write_tls")?;
            if !out.is_empty() {
                sock.write_all(&out).context("sock write")?;
            }
        }
        if conn.wants_read() {
            match sock.read(&mut raw) {
                Ok(0) => anyhow::bail!("peer closed"),
                Ok(n) => {
                    conn.read_tls(&mut &raw[..n]).context("read_tls")?;
                    conn.process_new_packets().context("process")?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e).context("sock read"),
            }
        }
        if !conn.is_handshaking() && !saw_http {
            let mut plain = [0u8; 1024];
            match conn.reader().read(&mut plain) {
                Ok(0) => {}
                Ok(n) => request.extend_from_slice(&plain[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e).context("plain read"),
            }
            if http_message_ready(&request) {
                let body = http_body(&request);
                let reply = stub_reply(body);
                let resp = length_prefixed(b"text/plain; charset=utf-8", &reply);
                conn.writer().write_all(&resp).context("http write")?;
                saw_http = true;
            }
        }
        if saw_http && !conn.wants_write() {
            return Ok(());
        }
    }
    anyhow::bail!("grok-one handshake/serve loop exhausted")
}

fn http_message_ready(buf: &[u8]) -> bool {
    let Some((head, body)) = split_head(buf) else {
        return false;
    };
    let need = content_length(head).unwrap_or(0);
    body.len() >= need
}

fn split_head(buf: &[u8]) -> Option<(&[u8], &[u8])> {
    let pos = buf.windows(4).position(|w| w == b"\r\n\r\n")?;
    let split = pos + 4;
    Some((&buf[..split], &buf[split..]))
}

fn content_length(head: &[u8]) -> Option<usize> {
    for line in head.split(|&b| b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let Some(rest) = line
            .strip_prefix(b"Content-Length:")
            .or_else(|| line.strip_prefix(b"content-length:"))
        else {
            continue;
        };
        return core::str::from_utf8(rest.trim_ascii()).ok()?.parse().ok();
    }
    None
}

fn http_body(buf: &[u8]) -> &[u8] {
    let Some((head, body)) = split_head(buf) else {
        return b"";
    };
    let n = content_length(head).unwrap_or(0).min(body.len());
    &body[..n]
}

fn length_prefixed(content_type: &[u8], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::from(b"HTTP/1.1 200 OK\r\nContent-Type: ".as_slice());
    out.extend_from_slice(content_type);
    out.extend_from_slice(b"\r\nContent-Length: ");
    out.extend_from_slice(body.len().to_string().as_bytes());
    out.extend_from_slice(b"\r\nConnection: close\r\n\r\n");
    out.extend_from_slice(body);
    out
}

/// Scripted two-turn conversation for the runtime smoke.
pub fn stub_reply(body: &[u8]) -> Vec<u8> {
    if contains(body, GROK_STUB_TOOL_RESULT) {
        let mut out = Vec::from(GROK_STUB_TEXT);
        out.extend_from_slice(AGENT_SMOKE_BODY);
        out.push(b'\n');
        return out;
    }
    if contains(body, AGENT_SMOKE_PROMPT) {
        let mut out = Vec::from(GROK_STUB_TOOL_CALL);
        out.extend_from_slice(b"Read ");
        out.extend_from_slice(AGENT_SMOKE_PATH);
        out.push(b'\n');
        return out;
    }
    let mut out = Vec::from(GROK_STUB_TEXT);
    out.extend_from_slice(b"(unscripted)\n");
    out
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lerux_interface_types::GROK_STUB_PROMPT;

    #[test]
    fn prompt_emits_read_tool_call() {
        let mut body = Vec::from(GROK_STUB_PROMPT);
        body.extend_from_slice(AGENT_SMOKE_PROMPT);
        body.push(b'\n');
        let r = stub_reply(&body);
        assert!(r.starts_with(GROK_STUB_TOOL_CALL));
        assert!(contains(&r, AGENT_SMOKE_PATH));
    }

    #[test]
    fn tool_result_emits_file_body() {
        let mut body = Vec::from(GROK_STUB_PROMPT);
        body.extend_from_slice(AGENT_SMOKE_PROMPT);
        body.extend_from_slice(b"\n");
        body.extend_from_slice(GROK_STUB_TOOL_RESULT);
        body.extend_from_slice(b"Read ");
        body.extend_from_slice(AGENT_SMOKE_PATH);
        body.extend_from_slice(b"\n");
        body.extend_from_slice(AGENT_SMOKE_BODY);
        let r = stub_reply(&body);
        assert!(r.starts_with(GROK_STUB_TEXT));
        assert!(contains(&r, AGENT_SMOKE_BODY));
    }
}
