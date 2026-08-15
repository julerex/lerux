//! One-shot HTTPS origin for the fetch-tls smoke (port 8443 by default).
//!
//! Serves the same `200 OK` body as [`crate::http_one`], with the committed
//! smoke server cert in `support/tls/`.

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
                Ok(_) => {
                    conn.writer().write_all(RESPONSE).context("http write")?;
                    saw_http = true;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e).context("plain read"),
            }
        }
        if saw_http && !conn.wants_write() {
            return Ok(());
        }
    }
    anyhow::bail!("https-one handshake/serve loop exhausted")
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
