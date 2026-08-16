use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Read, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};

/// One scripted host→guest serial interaction (Phase 52 hw-serial).
#[derive(Debug, Clone)]
pub struct ScriptStep {
    pub send: String,
    pub expect: String,
}

#[derive(Debug)]
pub struct SmokeTest {
    pub expects: Vec<String>,
    pub curls: Vec<(String, String)>,
    pub unordered: bool,
    pub timeout_secs: u64,
    /// After boot expects, optional write/expect pairs (hw-serial only).
    pub script: Vec<ScriptStep>,
    pub script_timeout_secs: u64,
}

impl Default for SmokeTest {
    fn default() -> Self {
        Self {
            expects: vec!["lerux: Hello from Rust on seL4 Microkit!".into()],
            curls: Vec::new(),
            unordered: false,
            timeout_secs: 60,
            script: Vec::new(),
            script_timeout_secs: 30,
        }
    }
}

pub fn run_smoke(cmd: Command, test: &SmokeTest) -> Result<()> {
    run_smoke_with_capture(cmd, test, None)
}

/// Run smoke; on failure (or when `save_log` is set) write the serial capture.
///
/// Phase 57: default failure path writes `build/smoke-logs/<board>.serial.log` when
/// `save_log` is `Some`.
pub fn run_smoke_with_capture(
    mut cmd: Command,
    test: &SmokeTest,
    save_log: Option<&std::path::Path>,
) -> Result<()> {
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn()?;

    let stdout = child.stdout.take().context("child stdout pipe")?;
    let stderr = child.stderr.take().context("child stderr pipe")?;

    let output = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let out_clone = std::sync::Arc::clone(&output);
    let err_clone = std::sync::Arc::clone(&output);

    let out_thread = std::thread::spawn(move || {
        pump_reader(BufReader::new(stdout), out_clone);
    });
    let err_thread = std::thread::spawn(move || {
        pump_reader(BufReader::new(stderr), err_clone);
    });

    let result = (|| -> Result<()> {
        if test.unordered {
            let deadline = Instant::now() + Duration::from_secs(test.timeout_secs);
            expect_unordered(&output, &test.expects, deadline)?;
        } else {
            let per = std::cmp::max(30, test.timeout_secs / test.expects.len().max(1) as u64);
            expect_ordered(&output, &test.expects, per)?;
        }
        for (url, expect) in &test.curls {
            curl_check(url, expect, 30)?;
        }
        println!("\n==> smoke test passed");
        Ok(())
    })();

    let _ = child.kill();
    let _ = child.wait();
    let _ = out_thread.join();
    let _ = err_thread.join();

    let captured = output.lock().map(|s| s.clone()).unwrap_or_default();
    if let Some(path) = save_log {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(path, &captured) {
            eprintln!(
                "warning: could not write serial log {}: {e}",
                path.display()
            );
        } else if result.is_err() {
            eprintln!("==> serial capture: {}", path.display());
            eprintln!(
                "    re-run: cargo run -q -p lerux-cli -- diagnose {}",
                path.display()
            );
        }
    } else if result.is_err() {
        // Always dump a short tail on failure for CI logs.
        let tail: String = captured
            .lines()
            .rev()
            .take(40)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        if !tail.is_empty() {
            eprintln!("==> serial tail (failure):\n{tail}");
        }
    }
    result
}

fn pump_reader<R: Read>(mut reader: BufReader<R>, sink: std::sync::Arc<std::sync::Mutex<String>>) {
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                print!("{line}");
                let _ = std::io::Write::flush(&mut std::io::stdout());
                if let Ok(mut buf) = sink.lock() {
                    buf.push_str(&line);
                }
            }
            Err(_) => break,
        }
    }
}

fn expect_ordered(
    output: &std::sync::Arc<std::sync::Mutex<String>>,
    patterns: &[String],
    per: u64,
) -> Result<()> {
    for pattern in patterns {
        let deadline = Instant::now() + Duration::from_secs(per);
        loop {
            if output
                .lock()
                .map(|s| capture_contains(&s, pattern))
                .unwrap_or(false)
            {
                break;
            }
            if Instant::now() >= deadline {
                bail!("timed out waiting for {pattern:?}");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    Ok(())
}

fn expect_unordered(
    output: &std::sync::Arc<std::sync::Mutex<String>>,
    patterns: &[String],
    deadline: Instant,
) -> Result<()> {
    let mut remaining: Vec<_> = patterns.to_vec();
    while !remaining.is_empty() {
        if Instant::now() >= deadline {
            let missing = remaining
                .iter()
                .map(|p| format!("{p:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            bail!("timed out waiting for: {missing}");
        }
        if let Ok(buf) = output.lock() {
            remaining.retain(|p| !capture_contains(&buf, p));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

/// How `lerux test` drives the board (Phase 47).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TestMode {
    /// QEMU boards → qemu; hardware boards → hw-serial if `LERUX_HW_SERIAL` set, else image-only.
    #[default]
    Auto,
    /// Force QEMU (errors on hardware-only boards).
    Qemu,
    /// Force serial capture (`LERUX_HW_SERIAL` required).
    HwSerial,
}

impl TestMode {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "qemu" => Ok(Self::Qemu),
            "hw-serial" | "hw_serial" | "hw" => Ok(Self::HwSerial),
            other => bail!("unknown test mode {other:?}; use auto|qemu|hw-serial"),
        }
    }

    /// CLI flag, then `LERUX_TEST_MODE`, then Auto.
    pub fn from_env_or_flag(flag: Option<&str>) -> Result<Self> {
        if let Some(s) = flag {
            return Self::parse(s);
        }
        if let Ok(s) = std::env::var("LERUX_TEST_MODE")
            && !s.is_empty()
        {
            return Self::parse(&s);
        }
        Ok(Self::Auto)
    }
}

/// Hardware serial smoke: read from `LERUX_HW_SERIAL` (115200 8N1 raw by default).
///
/// Golden path:
/// `BOARD=rpi4b_4gb_workstation LERUX_HW_SERIAL=/dev/ttyUSB0 just test-hw`
///
/// Phase 52: optional `script` steps in smoke-expects.toml send shell commands after boot.
pub fn run_hw_serial_smoke(test: &SmokeTest) -> Result<()> {
    let device = std::env::var("LERUX_HW_SERIAL")
        .context("LERUX_HW_SERIAL not set (e.g. /dev/ttyUSB0). Required for --mode hw-serial")?;
    let baud = std::env::var("LERUX_HW_BAUD").unwrap_or_else(|_| "115200".into());
    println!("==> Hardware serial smoke on {device:?} ({baud} 8N1 raw)");

    let stty = Command::new("stty")
        .args(["-F", &device, &baud, "raw", "-echo"])
        .status()
        .context("stty serial config")?;
    if !stty.success() {
        bail!("stty failed configuring {device:?}");
    }

    // RDWR so Phase 52 script steps can inject REPL commands.
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&device)
        .with_context(|| format!("open serial {device:?}"))?;
    let reader = file
        .try_clone()
        .with_context(|| format!("clone serial {device:?} for reader"))?;
    let mut writer = file;

    let output = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let out_clone = std::sync::Arc::clone(&output);

    let reader_thread = std::thread::spawn(move || {
        pump_reader(BufReader::new(reader), out_clone);
    });

    let deadline = Instant::now() + Duration::from_secs(test.timeout_secs);
    let result = if test.unordered {
        expect_unordered(&output, &test.expects, deadline)
    } else {
        let per = std::cmp::max(30, test.timeout_secs / test.expects.len().max(1) as u64);
        expect_ordered(&output, &test.expects, per)
    };

    if let Err(e) = result {
        drop(reader_thread);
        return Err(e);
    }
    println!("==> boot expects matched");

    // Scripted REPL (Phase 52): send commands, wait for substrings in the serial log.
    if !test.script.is_empty() {
        println!("==> running {} scripted serial step(s)…", test.script.len());
        for (i, step) in test.script.iter().enumerate() {
            let mark = output.lock().map(|s| s.len()).unwrap_or(0);
            print!(
                "    [{}] send {:?} expect {:?}… ",
                i + 1,
                step.send.trim_end_matches(['\r', '\n']),
                step.expect
            );
            let _ = std::io::Write::flush(&mut std::io::stdout());
            writer
                .write_all(step.send.as_bytes())
                .with_context(|| format!("write serial step {}", i + 1))?;
            writer.flush().context("flush serial")?;
            let step_deadline = Instant::now() + Duration::from_secs(test.script_timeout_secs);
            loop {
                let found = output
                    .lock()
                    .map(|s| s.len() > mark && capture_contains(&s[mark..], &step.expect))
                    .unwrap_or(false);
                if found {
                    println!("ok");
                    break;
                }
                if Instant::now() >= step_deadline {
                    bail!(
                        "script step {} timed out waiting for {:?} after send {:?}",
                        i + 1,
                        step.expect,
                        step.send
                    );
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        println!("==> scripted REPL steps passed");
    }

    // Detach: reader may block on serial; we don't join forever.
    drop(reader_thread);
    println!("\n==> hardware serial smoke passed");
    Ok(())
}

/// sel4-logging line prefix: `{level:<5} [{target}] `.
fn sel4_log_prefix_len(s: &str) -> Option<usize> {
    let level = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"]
        .iter()
        .find(|l| s.starts_with(*l))?;
    let after_level = &s[level.len()..];
    let spaces = after_level.bytes().take_while(|&b| b == b' ').count();
    if spaces == 0 {
        return None;
    }
    let after_spaces = &after_level[spaces..];
    let rest = after_spaces.strip_prefix('[')?;
    let close = rest.find(']')?;
    let after_brack = &rest[close + 1..];
    if !after_brack.starts_with(' ') {
        return None;
    }
    Some(level.len() + spaces + 1 + close + 1 + 1)
}

/// Rebuild sel4-logging lines after concurrent debug_print spliced one
/// message inside another (chunked write callbacks).
///
/// `INFO  [net_server] virtio-nINFO  [blk_server] …\net: MAC …` becomes
/// a haystack that contains `virtio-net: MAC`.
fn collapse_interleaved_sel4_logs(haystack: &str) -> String {
    let bytes = haystack.as_bytes();
    let mut starts = Vec::new();
    let mut i = 0;
    while i < haystack.len() {
        if let Some(len) = sel4_log_prefix_len(&haystack[i..]) {
            starts.push((i, len));
            i += len;
            continue;
        }
        i += match haystack[i..].chars().next() {
            Some(c) => c.len_utf8(),
            None => break,
        };
    }
    if starts.is_empty() {
        return haystack.to_string();
    }

    let mut parts: Vec<(String, String)> = Vec::new();
    if starts[0].0 > 0 {
        parts.push((
            String::new(),
            String::from_utf8_lossy(&bytes[..starts[0].0]).into_owned(),
        ));
    }
    for (idx, &(off, plen)) in starts.iter().enumerate() {
        let body_start = off + plen;
        let body_end = starts
            .get(idx + 1)
            .map(|(n, _)| *n)
            .unwrap_or(haystack.len());
        parts.push((
            String::from_utf8_lossy(&bytes[off..off + plen]).into_owned(),
            String::from_utf8_lossy(&bytes[body_start..body_end]).into_owned(),
        ));
    }

    let mut leftovers = Vec::new();
    for (_prefix, body) in &mut parts {
        if let Some((head, tail)) = body.split_once('\n') {
            if !tail.is_empty() {
                leftovers.push(tail.to_string());
            }
            *body = format!("{head}\n");
        }
    }
    let mut li = 0;
    for (_prefix, body) in &mut parts {
        if !body.ends_with('\n') && li < leftovers.len() {
            body.push_str(&leftovers[li]);
            li += 1;
        }
    }

    let mut out = String::with_capacity(haystack.len());
    for (prefix, body) in parts {
        out.push_str(&prefix);
        out.push_str(&body);
    }
    out
}

/// Substring match that also accepts tokens split by concurrent sel4-logging.
pub(crate) fn capture_contains(haystack: &str, pattern: &str) -> bool {
    haystack.contains(pattern) || collapse_interleaved_sel4_logs(haystack).contains(pattern)
}

fn curl_check(url: &str, expect_substr: &str, timeout_secs: u64) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let mut last_error = String::new();
    while Instant::now() < deadline {
        match ureq::get(url).call() {
            Ok(response) => {
                let body = response.into_body().read_to_string().unwrap_or_default();
                if body.contains(expect_substr) {
                    println!("\n==> curl {url} ok");
                    return Ok(());
                }
                last_error = body;
            }
            Err(e) => last_error = e.to_string(),
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    bail!("curl {url} failed: expected {expect_substr:?}, last={last_error:?}");
}

/// Host curls after boot, from the board's `curl_expect` in boards.toml.
pub fn default_curls(root: &std::path::Path, board: &str) -> Vec<(String, String)> {
    let Ok(boards) = crate::board::load_boards(root) else {
        return Vec::new();
    };
    boards
        .get(board)
        .and_then(|b| b.curl_expect.clone())
        .map(|expect| vec![("http://127.0.0.1:18080/".into(), expect)])
        .unwrap_or_default()
}

pub fn run_board_test(
    root: &std::path::Path,
    board: &str,
    build_dir: &str,
    config: &str,
) -> Result<()> {
    run_board_test_with_mode(root, board, build_dir, config, TestMode::Auto)
}

pub fn run_board_test_with_mode(
    root: &std::path::Path,
    board: &str,
    build_dir: &str,
    config: &str,
    mode: TestMode,
) -> Result<()> {
    let ctx = crate::qemu::load_qemu_context(root, board, build_dir, config)?;
    if crate::qemu::is_http_board(&ctx.board) {
        crate::qemu::cleanup_http_conflicts();
    }
    let hardware = crate::qemu::is_hardware_board(&ctx);
    let hw_serial_set = std::env::var_os("LERUX_HW_SERIAL").is_some();

    let use_hw = match mode {
        TestMode::HwSerial => true,
        TestMode::Qemu => {
            if hardware {
                bail!(
                    "board {board:?} is hardware-only; cannot use --mode qemu (use --mode hw-serial with LERUX_HW_SERIAL)"
                );
            }
            false
        }
        TestMode::Auto => hardware && hw_serial_set,
    };

    if hardware && !use_hw {
        if mode == TestMode::HwSerial {
            // unreachable: use_hw true
        }
        println!(
            "==> Hardware board {board:?}: image built successfully.\n\
             \x20   No QEMU profile.\n\
             \x20   Deploy: just deploy-rpi4 DEST=/path/to/sd-boot\n\
             \x20   Golden path: LERUX_HW_SERIAL=/dev/ttyUSB0 BOARD={board} just test-hw\n\
             \x20   Install path: docs/boards.md#rpi4-workstation-install-path-phase-52"
        );
        return Ok(());
    }

    if use_hw {
        if !hardware && mode == TestMode::HwSerial {
            println!(
                "==> Note: {board:?} has a QEMU profile; hw-serial will only read LERUX_HW_SERIAL (no QEMU)."
            );
        }
        let mut test = crate::smoke_expects::smoke_test_for_board(root, board)?;
        // Host curls do not apply over bare serial.
        test.curls.clear();
        let _lock = crate::hw_lock::BoardLock::acquire(board)?;
        return run_hw_serial_smoke(&test);
    }

    crate::qemu::ensure_qemu_binary(&ctx.root, &ctx.board)?;
    crate::qemu::print_http_hint(&ctx);

    if ctx.board.needs_disk() {
        let disk = root.join("support/disk.img");
        if !disk.is_file() {
            crate::disk_img::disk_img(root)?;
        }
    }

    let helper = crate::qemu::setup_test_helpers(&ctx)?;
    let cmd = crate::qemu::qemu_command(&ctx)?;
    let test = crate::smoke_expects::smoke_test_for_board(root, board)?;

    // Phase 57: always capture serial under build/smoke-logs/ for diagnose.
    let log_path = root
        .join(build_dir)
        .join("smoke-logs")
        .join(format!("{board}.serial.log"));
    let result = run_smoke_with_capture(cmd, &test, Some(&log_path));
    if let Some(mut child) = helper {
        let _ = child.kill();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exact splice from the 2026-08-16 ipc-composed CI serial capture.
    const IPC_COMPOSED_CI_SNIP: &str = "\
INFO  [net_server] virtio-nINFO  [blk_server] virtio-blk: 8192 blocks x 512 bytes
et: MAC 52:54:00:12:34:56
";

    #[test]
    fn capture_contains_recovers_virtio_net_mac_from_ci_interleave() {
        assert!(
            !IPC_COMPOSED_CI_SNIP.contains("virtio-net: MAC"),
            "fixture must reproduce the raw split"
        );
        assert!(capture_contains(IPC_COMPOSED_CI_SNIP, "virtio-net: MAC"));
        assert!(capture_contains(
            IPC_COMPOSED_CI_SNIP,
            "virtio-blk: 8192 blocks"
        ));
    }

    #[test]
    fn capture_contains_keeps_intact_lines() {
        let intact = "INFO  [net_server] virtio-net: MAC 52:54:00:12:34:56\n";
        assert!(capture_contains(intact, "virtio-net: MAC"));
    }

    #[test]
    fn capture_contains_rejects_missing_token() {
        assert!(!capture_contains(IPC_COMPOSED_CI_SNIP, "lerux-net: ready"));
    }
}
