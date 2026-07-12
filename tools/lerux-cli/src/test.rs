use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};

#[derive(Debug)]
pub struct SmokeTest {
    pub expects: Vec<String>,
    pub curls: Vec<(String, String)>,
    pub unordered: bool,
    pub timeout_secs: u64,
}

impl Default for SmokeTest {
    fn default() -> Self {
        Self {
            expects: vec!["lerux: Hello from Rust on seL4 Microkit!".into()],
            curls: Vec::new(),
            unordered: false,
            timeout_secs: 60,
        }
    }
}

pub fn run_smoke(mut cmd: Command, test: &SmokeTest) -> Result<()> {
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
            if output.lock().map(|s| s.contains(pattern)).unwrap_or(false) {
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
            remaining.retain(|p| !buf.contains(p));
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

    let file = File::open(&device).with_context(|| format!("open serial {device:?}"))?;
    let output = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let out_clone = std::sync::Arc::clone(&output);

    let reader_thread = std::thread::spawn(move || {
        pump_reader(BufReader::new(file), out_clone);
    });

    let deadline = Instant::now() + Duration::from_secs(test.timeout_secs);
    let result = if test.unordered {
        expect_unordered(&output, &test.expects, deadline)
    } else {
        let per = std::cmp::max(30, test.timeout_secs / test.expects.len().max(1) as u64);
        expect_ordered(&output, &test.expects, per)
    };

    // Detach: reader may block on serial; we don't join forever.
    drop(reader_thread);
    result?;
    println!("\n==> hardware serial smoke passed");
    Ok(())
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

pub fn default_curls(board: &str) -> Vec<(String, String)> {
    if matches!(
        board,
        "qemu_virt_aarch64_http"
            | "qemu_virt_aarch64_http_composed"
            | "qemu_virt_riscv64_http"
            | "x86_64_generic_http"
    ) {
        vec![("http://127.0.0.1:18080/".into(), "lerux: HTTP ok".into())]
    } else if board == "qemu_virt_aarch64_workstation" {
        // Supervisor writes /boot.log; listing body includes the name.
        vec![("http://127.0.0.1:18080/".into(), "boot.log".into())]
    } else {
        Vec::new()
    }
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
    if crate::qemu::is_http_board(board) {
        crate::qemu::cleanup_http_conflicts();
    }

    let ctx = crate::qemu::load_qemu_context(root, board, build_dir, config)?;
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
             \x20   No QEMU profile. Deploy loader.img (U-Boot) for manual checks.\n\
             \x20   Golden path: LERUX_HW_SERIAL=/dev/ttyUSB0 BOARD={board} just test-hw\n\
             \x20   See docs/boards.md and docs/ci.md (Phase 47 hw-serial)."
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

    crate::qemu::ensure_qemu_binary(&ctx.root, ctx.board.qemu.as_deref().unwrap_or_default())?;
    crate::qemu::print_http_hint(&ctx);

    if matches!(
        board,
        "qemu_virt_aarch64_virtio"
            | "qemu_virt_aarch64_composed"
            | "qemu_virt_aarch64_blk_composed"
            | "qemu_virt_aarch64_ipc_composed"
            | "qemu_virt_riscv64_virtio"
            | "x86_64_generic_virtio"
    ) {
        let disk = root.join("support/disk.img");
        if !disk.is_file() {
            crate::disk_img::disk_img(root)?;
        }
    }

    let helper = crate::qemu::setup_test_helpers(&ctx)?;
    let cmd = crate::qemu::qemu_command(&ctx)?;
    let test = crate::smoke_expects::smoke_test_for_board(root, board)?;

    let result = run_smoke(cmd, &test);
    if let Some(mut child) = helper {
        let _ = child.kill();
    }
    result
}
