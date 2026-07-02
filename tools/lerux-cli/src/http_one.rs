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

const RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";

pub fn http_one(port: u16) -> Result<()> {
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
        .with_context(|| format!("bind 127.0.0.1:{port}"))?;
    eprintln!("http-one-server: listening on 127.0.0.1:{port}");

    let done = Arc::new(AtomicBool::new(false));
    let done_thread = Arc::clone(&done);
    let handle = thread::spawn(move || {
        if let Ok((mut conn, _)) = listener.accept() {
            let mut buf = [0u8; 512];
            let _ = conn.read(&mut buf);
            let _ = conn.write_all(RESPONSE);
            let _ = conn.shutdown(std::net::Shutdown::Write);
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

pub fn start_http_one_background(port: u16) -> Result<std::process::Child> {
    let child = std::process::Command::new(std::env::current_exe()?)
        .arg("http-one")
        .arg(port.to_string())
        .spawn()
        .context("spawn http-one")?;
    crate::tcp_echo::wait_for_port(port, 100);
    Ok(child)
}
