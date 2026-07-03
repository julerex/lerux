use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};

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

/// Optional hardware smoke: read serial from `LERUX_HW_SERIAL` after image build.
///
/// Configure the port first (115200 8N1 raw). Example:
/// `LERUX_HW_SERIAL=/dev/ttyUSB0 BOARD=rpi4b_4gb_workstation just test`
pub fn run_hw_serial_smoke(test: &SmokeTest) -> Result<()> {
    let device = std::env::var("LERUX_HW_SERIAL").context("LERUX_HW_SERIAL not set")?;
    println!("==> Hardware serial smoke on {device:?}");

    let stty = Command::new("stty")
        .args(["-F", &device, "115200", "raw", "-echo"])
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

    let _ = reader_thread.join();
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

pub fn default_expects(board: &str) -> Vec<String> {
    match board {
        "qemu_virt_aarch64_echo" | "qemu_virt_riscv64_echo" | "x86_64_generic_echo" => {
            vec![
                "echo-server ready".into(),
                "lerux-echo: pong".into(),
                "lerux-echo: lerux".into(),
            ]
        }
        "qemu_virt_aarch64_blk_composed" => vec![
            "lerux-supervisor: RTC".into(),
            "lerux-supervisor: timer ok".into(),
            "lerux-supervisor: init ok".into(),
            "lerux-blk: ready".into(),
            "virtio-blk:".into(),
            "lerux-blk: MBR sig".into(),
            "lerux-blk: write round-trip ok".into(),
        ],
        "qemu_virt_aarch64_blk" | "qemu_virt_riscv64_blk" | "x86_64_generic_blk" => {
            vec![
                "lerux-blk: ready".into(),
                "virtio-blk:".into(),
                "lerux-blk: MBR sig".into(),
                "lerux-blk: write round-trip ok".into(),
            ]
        }
        "qemu_virt_aarch64_net_composed" => vec![
            "lerux-supervisor: RTC".into(),
            "lerux-supervisor: timer ok".into(),
            "lerux-supervisor: init ok".into(),
            "lerux-net: ready".into(),
            "virtio-net: MAC".into(),
            "lerux-net: TX ok".into(),
            "lerux-net: IPC ok".into(),
        ],
        "qemu_virt_aarch64_ipc_composed" => vec![
            "lerux-supervisor: RTC".into(),
            "lerux-supervisor: timer ok".into(),
            "lerux-supervisor: init ok".into(),
            "lerux-blk: ready".into(),
            "virtio-blk:".into(),
            "lerux-blk: MBR sig".into(),
            "lerux-blk: write round-trip ok".into(),
            "lerux-net: ready".into(),
            "virtio-net: MAC".into(),
            "lerux-net: TX ok".into(),
            "lerux-net: IPC ok".into(),
        ],
        "qemu_virt_aarch64_workstation" => vec![
            "lerux-supervisor: RTC".into(),
            "lerux-supervisor: timer ok".into(),
            "lerux-supervisor: init ok".into(),
            "lerux-fs: ready".into(),
            "virtio-blk:".into(),
            "lerux-net: ready".into(),
            "virtio-net: MAC".into(),
            "lerux-supervisor: ready".into(),
            "lerux-shell: ready".into(),
            "lerux-edit: ready".into(),
        ],
        "rpi4b_4gb_workstation" => vec![
            "lerux-supervisor: init ok".into(),
            "genet:".into(),
            "emmc2:".into(),
            "lerux-fs: ready".into(),
            "lerux-net: ready".into(),
            "lerux-supervisor: ready".into(),
            "lerux-shell: ready".into(),
            "lerux-edit: ready".into(),
        ],
        "rpi4b_4gb_net" => vec![
            "lerux-net: ready".into(),
            "genet:".into(),
            "genet-driver: ready".into(),
        ],
        "rpi4b_4gb_blk" => vec![
            "lerux-blk: ready".into(),
            "emmc2:".into(),
            "emmc2-driver: ready".into(),
        ],
        "qemu_virt_aarch64_net" | "qemu_virt_riscv64_net" | "x86_64_generic_net" => vec![
            "lerux-net: ready".into(),
            "virtio-net: MAC".into(),
            "lerux-net: TX ok".into(),
            "lerux-net: IPC ok".into(),
        ],
        "qemu_virt_aarch64_fetch" => vec![
            "lerux-net: ready".into(),
            "virtio-net: MAC".into(),
            "lerux-fetch: 200".into(),
        ],
        "qemu_virt_aarch64_fs" => vec![
            "lerux-fs: ready".into(),
            "virtio-blk:".into(),
            "lerux-fs: round-trip ok".into(),
        ],
        "qemu_virt_aarch64_init" => vec![
            "lerux-supervisor: RTC".into(),
            "lerux-supervisor: timer ok".into(),
            "lerux-supervisor: init ok".into(),
        ],
        "qemu_virt_aarch64_virtio" | "qemu_virt_riscv64_virtio" | "x86_64_generic_virtio" => {
            vec![
                "lerux: Hello from Rust on seL4 Microkit!".into(),
                "virtio-blk:".into(),
                "virtio-net: MAC".into(),
                "virtio-net: TX ok".into(),
                "virtio-net: TCP RX ok".into(),
                "virtio-blk: MBR sig".into(),
            ]
        }
        "qemu_virt_aarch64_composed" => vec![
            "lerux-supervisor: RTC".into(),
            "lerux-supervisor: timer ok".into(),
            "lerux-supervisor: init ok".into(),
            "lerux: Hello from Rust on seL4 Microkit!".into(),
            "virtio-blk:".into(),
            "virtio-net: MAC".into(),
            "virtio-net: TX ok".into(),
            "virtio-net: TCP RX ok".into(),
            "virtio-blk: MBR sig".into(),
        ],
        "qemu_virt_aarch64_http" | "qemu_virt_riscv64_http" | "x86_64_generic_http" => {
            vec!["lerux-http: listening".into()]
        }
        "qemu_virt_aarch64_http_composed" => vec![
            "lerux-supervisor: RTC".into(),
            "lerux-supervisor: timer ok".into(),
            "lerux-supervisor: init ok".into(),
            "lerux-http: listening".into(),
        ],
        _ => vec!["lerux: Hello from Rust on seL4 Microkit!".into()],
    }
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
    if crate::qemu::is_http_board(board) {
        crate::qemu::cleanup_http_conflicts();
    }

    let ctx = crate::qemu::load_qemu_context(root, board, build_dir, config)?;

    if crate::qemu::is_hardware_board(&ctx) {
        if std::env::var_os("LERUX_HW_SERIAL").is_some() {
            let test = SmokeTest {
                expects: default_expects(board),
                curls: Vec::new(),
                unordered: matches!(board, "rpi4b_4gb_workstation"),
                timeout_secs: 120,
            };
            return run_hw_serial_smoke(&test);
        }
        println!(
            "==> Hardware board {:?}: image built successfully.\n    No QEMU profile. Perform manual smoke/verification on the device.\n    Optional: LERUX_HW_SERIAL=/dev/ttyUSB0 just test (serial capture).\n    See docs/boards.md for deployment (e.g. U-Boot on RPi4).",
            board
        );
        return Ok(());
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

    let test = SmokeTest {
        expects: default_expects(board),
        curls: default_curls(board),
        unordered: false,
        timeout_secs: 60,
    };

    let result = run_smoke(cmd, &test);
    if let Some(mut child) = helper {
        let _ = child.kill();
    }
    result
}
