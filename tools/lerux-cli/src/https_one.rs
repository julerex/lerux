//! One-shot HTTPS origin for the fetch-tls and request-server smokes
//! (port 8443 by default).
//!
//! `GET /` keeps the same `200 OK` body as [`crate::http_one`]. `GET /fixture.html`
//! serves `support/browser/fixture.html` (Phase 73). Smoke server cert lives in
//! `support/tls/`.

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    ServerConfig, ServerConnection,
};

const RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
const NOT_FOUND: &[u8] =
    b"HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nnot found";

pub fn https_one(port: u16) -> Result<()> {
    let config = server_config()?;
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
        .with_context(|| format!("bind 127.0.0.1:{port}"))?;
    eprintln!("https-one-server: listening on 127.0.0.1:{port}");

    let done = Arc::new(AtomicBool::new(false));
    let done_thread = Arc::clone(&done);
    let handle = thread::spawn(move || {
        if let Ok((mut sock, peer)) = listener.accept() {
            eprintln!("https-one-server: accepted {peer}");
            if let Err(e) = serve_one(&config, &mut sock) {
                eprintln!("https-one-server: {e:#}");
            }
            let _ = sock.shutdown(std::net::Shutdown::Write);
            done_thread.store(true, Ordering::SeqCst);
        }
    });

    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    while !done.load(Ordering::SeqCst) {
        if std::time::Instant::now() >= deadline {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let _ = handle.join();
    Ok(())
}

pub fn start_https_one_background(port: u16) -> Result<std::process::Child> {
    let child = std::process::Command::new(std::env::current_exe()?)
        .arg("https-one")
        .arg(port.to_string())
        .spawn()
        .context("spawn https-one")?;
    crate::tcp_echo::wait_for_port(port, 100);
    Ok(child)
}

fn serve_one(config: &Arc<ServerConfig>, sock: &mut std::net::TcpStream) -> Result<()> {
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
            let mut plain = [0u8; 512];
            match conn.reader().read(&mut plain) {
                Ok(0) => {}
                Ok(n) => request.extend_from_slice(&plain[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e).context("plain read"),
            }
            if request_headers_complete(&request) {
                let path = path_from_http_request(&request);
                let fixture = if path == b"/fixture.html" {
                    load_fixture()?
                } else {
                    Vec::new()
                };
                let body = http_response_for_path(path, &fixture);
                conn.writer().write_all(&body).context("http write")?;
                saw_http = true;
            }
        }
        if saw_http && !conn.wants_write() {
            return Ok(());
        }
    }
    anyhow::bail!("https-one handshake/serve loop exhausted")
}

fn request_headers_complete(buf: &[u8]) -> bool {
    buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.windows(2).any(|w| w == b"\n\n")
}

fn path_from_http_request(buf: &[u8]) -> &[u8] {
    let line_end = buf.iter().position(|&b| b == b'\n').unwrap_or(buf.len());
    let line = buf[..line_end]
        .strip_suffix(b"\r")
        .unwrap_or(&buf[..line_end]);
    let mut parts = line.split(|&b| b == b' ');
    let _method = parts.next();
    parts.next().unwrap_or(b"/")
}

fn http_response_for_path(path: &[u8], fixture: &[u8]) -> Vec<u8> {
    if path == b"/" || path.is_empty() {
        return RESPONSE.to_vec();
    }
    if path == b"/fixture.html" {
        return length_prefixed(b"text/html; charset=utf-8", fixture);
    }
    NOT_FOUND.to_vec()
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

fn load_fixture() -> Result<Vec<u8>> {
    let path = crate::process::repo_root()?.join("support/browser/fixture.html");
    std::fs::read(&path).with_context(|| format!("read {}", path.display()))
}

fn server_config() -> Result<Arc<ServerConfig>> {
    let root = crate::process::repo_root()?;
    let cert_path = root.join("support/tls/lerux-smoke-server.pem");
    let key_path = root.join("support/tls/lerux-smoke-server.key");
    let cert_pem =
        std::fs::read(&cert_path).with_context(|| format!("read {}", cert_path.display()))?;
    let key_pem =
        std::fs::read(&key_path).with_context(|| format!("read {}", key_path.display()))?;

    let mut cert_reader = cert_pem.as_slice();
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .context("parse server cert")?;
    let mut key_reader = key_pem.as_slice();
    let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut key_reader)
        .context("parse server key")?
        .context("no private key in smoke-server.key")?;

    let cfg = ServerConfig::builder_with_provider(rustls::crypto::ring::default_provider().into())
        .with_safe_default_protocol_versions()
        .context("tls versions")?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .context("tls server config")?;
    Ok(Arc::new(cfg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_from_get() {
        assert_eq!(
            path_from_http_request(b"GET /fixture.html HTTP/1.1\r\n\r\n"),
            b"/fixture.html"
        );
        assert_eq!(path_from_http_request(b"GET / HTTP/1.1\r\n"), b"/");
    }

    #[test]
    fn root_keeps_fetch_tls_body() {
        let r = http_response_for_path(b"/", b"unused");
        assert_eq!(r, RESPONSE);
    }

    #[test]
    fn fixture_path_embeds_body() {
        let r = http_response_for_path(b"/fixture.html", b"<p>lerux-http-fixture</p>");
        assert!(r.starts_with(b"HTTP/1.1 200"));
        assert!(r
            .windows(b"lerux-http-fixture".len())
            .any(|w| w == b"lerux-http-fixture"));
    }

    #[test]
    fn unknown_path_is_404() {
        let r = http_response_for_path(b"/nope", b"x");
        assert!(r.starts_with(b"HTTP/1.1 404"));
    }
}
