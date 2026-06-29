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

pub fn port_is_listening(port: u16) -> bool {
    TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).is_err()
}

pub fn tcp_echo(port: u16) -> Result<()> {
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
        .with_context(|| format!("bind 127.0.0.1:{port}"))?;
    eprintln!("tcp-echo-server: listening on 127.0.0.1:{port}");

    let done = Arc::new(AtomicBool::new(false));
    let done_thread = Arc::clone(&done);
    let handle = thread::spawn(move || {
        if let Ok((mut conn, _)) = listener.accept() {
            let mut buf = [0u8; 64];
            if let Ok(n) = conn.read(&mut buf)
                && n > 0
            {
                let _ = conn.write_all(&buf[..n]);
            }
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

pub fn wait_for_port(port: u16, attempts: u32) {
    for _ in 0..attempts {
        if port_is_listening(port) {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

pub fn start_tcp_echo_background(port: u16) -> Result<std::process::Child> {
    let child = std::process::Command::new(std::env::current_exe()?)
        .arg("tcp-echo")
        .arg(port.to_string())
        .spawn()
        .context("spawn tcp-echo")?;
    wait_for_port(port, 100);
    Ok(child)
}
