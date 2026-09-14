//! Scripted HTTPS completions stub for the Phase 77–79 agent smokes
//! (port 8444 by default). Not live xAI.
//!
//! Speaks the line protocol in `lerux-interface-types` (`PROMPT` / `TOOL_CALL` /
//! `TOOL_RESULT` / `TEXT`) over the smoke CA. Also serves `GET /fixture.html`
//! for WebFetch. Serves several connections then exits.

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use lerux_interface_types::{
    AGENT_EDIT_FROM, AGENT_EDIT_TO, AGENT_SMOKE_BODY, AGENT_SMOKE_PATH, AGENT_SMOKE_PROMPT,
    AGENT_TOOLS_OK, AGENT_TOOLS_PROMPT, AGENT_WEBFETCH_URL, AGENT_WORK_HELLO, GROK_STUB_TEXT,
    GROK_STUB_TOOL_CALL, GROK_STUB_TOOL_RESULT,
};
use rustls::ServerConnection;

use crate::https_one::server_config;

/// Runtime smoke: two POSTs. Tools smoke: four POSTs + one GET. A few extra
/// slots cover interactive follow-ups during `lerux run`.
const TURNS: usize = 12;

pub fn grok_one(port: u16) -> Result<()> {
    let config = server_config()?;
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
        .with_context(|| format!("bind 127.0.0.1:{port}"))?;
    listener
        .set_nonblocking(true)
        .context("grok-one nonblocking accept")?;
    eprintln!("grok-one-server: listening on 127.0.0.1:{port}");

    let deadline = Instant::now() + Duration::from_secs(90);
    let mut served = 0usize;
    while served < TURNS && Instant::now() < deadline {
        match listener.accept() {
            Ok((mut sock, peer)) => {
                eprintln!("grok-one-server: accepted {peer}");
                if let Err(e) = serve_one(&config, &mut sock) {
                    eprintln!("grok-one-server: {e:#}");
                }
                let _ = sock.shutdown(std::net::Shutdown::Write);
                served += 1;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                eprintln!("grok-one-server: accept {e}");
                break;
            }
        }
    }
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
        // Return as soon as the response is on the wire. Waiting for TLS
        // close_notify / peer close is what blocked accept() on turn 3.
        if saw_http && !conn.wants_write() {
            return Ok(());
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
                let reply = http_reply(&request);
                conn.writer().write_all(&reply).context("http write")?;
                saw_http = true;
            }
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

fn http_reply(request: &[u8]) -> Vec<u8> {
    let (method, path) = request_line(request);
    if method == b"GET" {
        if path == b"/fixture.html" {
            return length_prefixed(b"text/plain; charset=utf-8", b"lerux-agent-fetch\n");
        }
        return b"HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nnot found"
            .to_vec();
    }
    length_prefixed(
        b"text/plain; charset=utf-8",
        &stub_reply(http_body(request)),
    )
}

fn request_line(buf: &[u8]) -> (&[u8], &[u8]) {
    let line_end = buf.iter().position(|&b| b == b'\n').unwrap_or(buf.len());
    let line = buf[..line_end]
        .strip_suffix(b"\r")
        .unwrap_or(&buf[..line_end]);
    let mut parts = line.split(|&b| b == b' ');
    (parts.next().unwrap_or(b""), parts.next().unwrap_or(b"/"))
}

/// Scripted completions: runtime smoke (Read /hello.txt) or tools smoke
/// (Read → Edit → WebFetch).
pub fn stub_reply(body: &[u8]) -> Vec<u8> {
    if contains(body, AGENT_TOOLS_PROMPT) || contains(body, AGENT_WORK_HELLO) {
        return tools_script(body);
    }
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

fn tools_script(body: &[u8]) -> Vec<u8> {
    if contains(body, b"WebFetch") && contains(body, GROK_STUB_TOOL_RESULT) {
        let mut out = Vec::from(GROK_STUB_TEXT);
        out.extend_from_slice(AGENT_TOOLS_OK);
        out.push(b'\n');
        return out;
    }
    if contains(body, b"Edit") && contains(body, GROK_STUB_TOOL_RESULT) {
        let mut out = Vec::from(GROK_STUB_TOOL_CALL);
        out.extend_from_slice(b"WebFetch ");
        out.extend_from_slice(AGENT_WEBFETCH_URL);
        out.push(b'\n');
        return out;
    }
    if contains(body, b"Read") && contains(body, GROK_STUB_TOOL_RESULT) {
        let mut out = Vec::from(GROK_STUB_TOOL_CALL);
        out.extend_from_slice(b"Edit ");
        out.extend_from_slice(AGENT_WORK_HELLO);
        out.push(b'|');
        out.extend_from_slice(AGENT_EDIT_FROM);
        out.push(b'|');
        out.extend_from_slice(AGENT_EDIT_TO);
        out.push(b'\n');
        return out;
    }
    let mut out = Vec::from(GROK_STUB_TOOL_CALL);
    out.extend_from_slice(b"Read ");
    out.extend_from_slice(AGENT_WORK_HELLO);
    out.push(b'\n');
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

    #[test]
    fn tools_prompt_emits_workspace_read() {
        let mut body = Vec::from(GROK_STUB_PROMPT);
        body.extend_from_slice(AGENT_TOOLS_PROMPT);
        let r = stub_reply(&body);
        assert!(contains(&r, AGENT_WORK_HELLO));
        assert!(r.starts_with(GROK_STUB_TOOL_CALL));
    }

    #[test]
    fn tools_script_ends_with_ok() {
        let mut body = Vec::from(GROK_STUB_PROMPT);
        body.extend_from_slice(AGENT_TOOLS_PROMPT);
        body.extend_from_slice(b"\n");
        body.extend_from_slice(GROK_STUB_TOOL_RESULT);
        body.extend_from_slice(b"WebFetch ");
        body.extend_from_slice(AGENT_WEBFETCH_URL);
        let r = stub_reply(&body);
        assert!(contains(&r, AGENT_TOOLS_OK));
    }

    #[test]
    fn tools_script_edit_then_webfetch() {
        let mut body = Vec::from(GROK_STUB_PROMPT);
        body.extend_from_slice(AGENT_TOOLS_PROMPT);
        body.extend_from_slice(b"\n");
        body.extend_from_slice(GROK_STUB_TOOL_RESULT);
        body.extend_from_slice(b"Read ");
        body.extend_from_slice(AGENT_WORK_HELLO);
        let r = stub_reply(&body);
        assert!(r.starts_with(GROK_STUB_TOOL_CALL));
        assert!(contains(&r, b"Edit "));
        body.extend_from_slice(b"\n");
        body.extend_from_slice(GROK_STUB_TOOL_RESULT);
        body.extend_from_slice(b"Edit ");
        body.extend_from_slice(AGENT_WORK_HELLO);
        let r = stub_reply(&body);
        assert!(contains(&r, AGENT_WEBFETCH_URL));
    }

    #[test]
    fn get_fixture_returns_fetch_mark() {
        let r = http_reply(b"GET /fixture.html HTTP/1.1\r\nHost: host\r\n\r\n");
        assert!(contains(&r, b"lerux-agent-fetch"));
        let miss = http_reply(b"GET /nope HTTP/1.1\r\n\r\n");
        assert!(contains(&miss, b"not found"));
    }
}
