//! One-shot HTTPS origin for the fetch, browser, agent, and program smokes
//! (port 8443 by default).
//!
//! The matcher compares the raw request-target. The method is ignored. A query
//! on an otherwise matching path is a 404: `GET /fixture.html?q=hi` does not
//! serve `/fixture.html`. One connection carries one response, then
//! `Connection: close`. Eight connections, then the helper stops.
//!
//! `GET /` keeps the same `200 OK` body as [`crate::http_one`]. `GET /fixture.html`
//! serves `support/browser/fixture.html`. `GET /paint.html` serves
//! `support/browser/paint.html`. `GET /frames.html` serves
//! `support/browser/frames.html`. `GET /child.html` serves
//! `support/browser/child.html`. `GET /image.html` serves
//! `support/browser/image.html`. `GET /grid.png` serves
//! `support/browser/grid.png`. `GET /smoke.lrw` serves
//! `support/prog/smoke.lrw`. `GET /go` is a `302` whose `Location` is
//! `/fixture.html` and whose body is empty. Desk pages (`desk.html`,
//! `desk-link.html`, `desk-form.html`) are not in `support/browser/`. Smoke
//! server cert lives in `support/tls/`.

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    sync::Arc,
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
const GO: &[u8] = b"HTTP/1.1 302 Found\r\nLocation: /fixture.html\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

/// Joint interactive smoke may GET twice (retries). One-shot boards still work.
const TURNS: usize = 8;

pub fn https_one(port: u16) -> Result<()> {
    let config = server_config()?;
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
        .with_context(|| format!("bind 127.0.0.1:{port}"))?;
    listener
        .set_nonblocking(true)
        .context("https-one nonblocking accept")?;
    eprintln!("https-one-server: listening on 127.0.0.1:{port}");

    let deadline = std::time::Instant::now() + Duration::from_secs(90);
    let mut served = 0usize;
    while served < TURNS && std::time::Instant::now() < deadline {
        match listener.accept() {
            Ok((mut sock, peer)) => {
                eprintln!("https-one-server: accepted {peer}");
                if let Err(e) = serve_one(&config, &mut sock) {
                    eprintln!("https-one-server: {e:#}");
                }
                let _ = sock.shutdown(std::net::Shutdown::Write);
                served += 1;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                eprintln!("https-one-server: accept {e}");
                break;
            }
        }
    }
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
        write_tls_if_wanted(&mut conn, sock)?;
        if saw_http && !conn.wants_write() {
            return Ok(());
        }
        read_tls_if_wanted(&mut conn, sock, &mut raw)?;
        if !conn.is_handshaking() && !saw_http {
            saw_http = read_http_request(&mut conn, &mut request)?;
        }
    }
    anyhow::bail!("https-one handshake/serve loop exhausted")
}

fn write_tls_if_wanted(conn: &mut ServerConnection, sock: &mut std::net::TcpStream) -> Result<()> {
    if conn.wants_write() {
        let mut out = Vec::new();
        conn.write_tls(&mut out).context("write_tls")?;
        if !out.is_empty() {
            sock.write_all(&out).context("sock write")?;
        }
    }
    Ok(())
}

fn read_tls_if_wanted(
    conn: &mut ServerConnection,
    sock: &mut std::net::TcpStream,
    raw: &mut [u8],
) -> Result<()> {
    if conn.wants_read() {
        match sock.read(raw) {
            Ok(0) => anyhow::bail!("peer closed"),
            Ok(n) => {
                conn.read_tls(&mut &raw[..n]).context("read_tls")?;
                conn.process_new_packets().context("process")?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e).context("sock read"),
        }
    }
    Ok(())
}

fn read_http_request(conn: &mut ServerConnection, request: &mut Vec<u8>) -> Result<bool> {
    let mut plain = [0u8; 512];
    match conn.reader().read(&mut plain) {
        Ok(0) => {}
        Ok(n) => request.extend_from_slice(&plain[..n]),
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        Err(e) => return Err(e).context("plain read"),
    }
    if request_headers_complete(request) {
        let path = path_from_http_request(request);
        let file_body = load_browser_path(path)?;
        let body = http_response_for_path(path, &file_body);
        conn.writer().write_all(&body).context("http write")?;
        return Ok(true);
    }
    Ok(false)
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

fn http_response_for_path(path: &[u8], file_body: &[u8]) -> Vec<u8> {
    if path == b"/" || path.is_empty() {
        return RESPONSE.to_vec();
    }
    if path == b"/fixture.html"
        || path == b"/paint.html"
        || path == b"/frames.html"
        || path == b"/child.html"
        || path == b"/image.html"
    {
        return length_prefixed(b"text/html; charset=utf-8", file_body);
    }
    if path == b"/grid.png" {
        return length_prefixed(b"image/png", file_body);
    }
    if path == b"/smoke.lrw" {
        return length_prefixed(b"application/octet-stream", file_body);
    }
    if path == b"/go" {
        return GO.to_vec();
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

fn load_browser_path(url_path: &[u8]) -> Result<Vec<u8>> {
    let (dir, name) = match url_path {
        b"/fixture.html" => ("browser", "fixture.html"),
        b"/paint.html" => ("browser", "paint.html"),
        b"/frames.html" => ("browser", "frames.html"),
        b"/child.html" => ("browser", "child.html"),
        b"/image.html" => ("browser", "image.html"),
        b"/grid.png" => ("browser", "grid.png"),
        b"/smoke.lrw" => ("prog", "smoke.lrw"),
        _ => return Ok(Vec::new()),
    };
    load_support_file(dir, name)
}

fn load_support_file(dir: &str, name: &str) -> Result<Vec<u8>> {
    let path = crate::process::repo_root()?
        .join("support")
        .join(dir)
        .join(name);
    std::fs::read(&path).with_context(|| format!("read {}", path.display()))
}

pub(crate) fn server_config() -> Result<Arc<ServerConfig>> {
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
    fn smoke_lrw_embeds_body_and_length() {
        let body = b"LRW1-test";
        let r = http_response_for_path(b"/smoke.lrw", body);
        assert!(r.starts_with(b"HTTP/1.1 200"));
        assert!(r
            .windows(b"Content-Length: 9".len())
            .any(|w| w == b"Content-Length: 9"));
        assert!(r.ends_with(body));
    }

    #[test]
    fn paint_path_embeds_body() {
        let r = http_response_for_path(b"/paint.html", b"<h1>lerux</h1>");
        assert!(r.starts_with(b"HTTP/1.1 200"));
        assert!(r
            .windows(b"<h1>lerux</h1>".len())
            .any(|w| w == b"<h1>lerux</h1>"));
    }

    #[test]
    fn frames_and_child_embed_their_bodies() {
        let frames = http_response_for_path(b"/frames.html", b"<iframe src=\"/child.html\">");
        assert!(frames.starts_with(b"HTTP/1.1 200"));
        assert!(frames.ends_with(b"<iframe src=\"/child.html\">"));
        let child = http_response_for_path(b"/child.html", b"<a href=\"#mark\">x</a>");
        assert!(child.starts_with(b"HTTP/1.1 200"));
        assert!(child.ends_with(b"<a href=\"#mark\">x</a>"));
        let image = http_response_for_path(b"/image.html", b"<img src=\"/grid.png\">");
        assert!(image.starts_with(b"HTTP/1.1 200"));
        assert!(image.ends_with(b"<img src=\"/grid.png\">"));
        let png = http_response_for_path(b"/grid.png", b"\x89PNG");
        assert!(png.windows(b"image/png".len()).any(|w| w == b"image/png"));
        assert!(png.ends_with(b"\x89PNG"));
    }

    #[test]
    fn unknown_path_is_404() {
        let r = http_response_for_path(b"/nope", b"x");
        assert!(r.starts_with(b"HTTP/1.1 404"));
    }

    #[test]
    fn go_redirects_once_to_the_fixture() {
        let r = http_response_for_path(b"/go", b"unused");
        assert!(r.starts_with(b"HTTP/1.1 302"));
        assert!(r
            .windows(b"Location: /fixture.html".len())
            .any(|w| w == b"Location: /fixture.html"));
        assert!(r
            .windows(b"Content-Length: 0".len())
            .any(|w| w == b"Content-Length: 0"));
        assert!(r.ends_with(b"\r\n\r\n"));
        assert!(!r.windows(b"unused".len()).any(|w| w == b"unused"));
    }
}
