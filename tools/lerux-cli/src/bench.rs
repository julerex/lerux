//! Phase 49: host-driven microbench runner for QEMU boards.
//!
//! Guests print `lerux-bench: <name> start n=N` / `done n=N` markers. The host
//! measures wall-clock between those lines (CNTVCT is not available in seL4 EL0
//! on stock configs).

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::{build, smoke_expects, test::SmokeTest};

const BENCH_BOARDS: &[&str] = &[
    "qemu_virt_aarch64_bench_echo",
    "qemu_virt_aarch64_bench_blk",
    "qemu_virt_aarch64_bench_net",
];

#[derive(Debug, Serialize)]
struct BenchReport {
    generated_unix: u64,
    host: HostInfo,
    results: Vec<BenchResult>,
}

#[derive(Debug, Serialize)]
struct HostInfo {
    uname: String,
    qemu_version: String,
    note: String,
}

#[derive(Debug, Serialize)]
struct BenchResult {
    board: String,
    metric: String,
    value: u64,
    unit: String,
    n: u64,
    total_ns: u64,
    raw_start: String,
    raw_done: String,
}

/// Run all Phase 49 bench boards, parse metrics, write markdown + JSON summaries.
///
/// When `check` is true, compare against `support/bench-thresholds.toml` (Phase 57).
pub fn run_bench(
    root: &Path,
    build_dir: &str,
    config: &str,
    out_dir: Option<&Path>,
    check: bool,
) -> Result<()> {
    let out = out_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(build_dir).join("bench"));
    fs::create_dir_all(&out).with_context(|| format!("create {}", out.display()))?;

    let disk = root.join("support/disk.img");
    if !disk.is_file() {
        crate::disk_img::disk_img(root)?;
    }

    let mut results = Vec::new();
    for board in BENCH_BOARDS {
        println!("\n==> bench board {board}");
        build::image(root, board, build_dir, config)?;
        let (captured, wall) = run_board_capture(root, board, build_dir, config)?;
        // Always keep raw serial for diagnose (Phase 57).
        let log_path = out.join(format!("{board}.serial.log"));
        fs::write(&log_path, &captured).with_context(|| format!("write {}", log_path.display()))?;
        let parsed = derive_metrics(board, &captured, wall)?;
        for r in &parsed {
            println!(
                "    {} {} {} (n={}, total_ns={})",
                r.metric, r.value, r.unit, r.n, r.total_ns
            );
        }
        results.extend(parsed);
    }

    let report = BenchReport {
        generated_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        host: HostInfo {
            uname: capture_cmd(&["uname", "-a"]).unwrap_or_else(|_| "unknown".into()),
            qemu_version: capture_qemu_version(root),
            note: "Host wall-clock between guest start/done markers on QEMU TCG; relative only."
                .into(),
        },
        results,
    };

    let json_path = out.join("bench-results.json");
    let md_path = out.join("bench-results.md");
    let json = serde_json::to_string_pretty(&report).context("serialize bench json")?;
    fs::write(&json_path, json).with_context(|| format!("write {}", json_path.display()))?;
    fs::write(&md_path, render_markdown(&report))
        .with_context(|| format!("write {}", md_path.display()))?;

    let docs_snapshot = root.join("docs/bench-results.latest.md");
    fs::write(&docs_snapshot, render_markdown(&report))
        .with_context(|| format!("write {}", docs_snapshot.display()))?;

    println!("\n==> bench complete");
    println!("    {}", json_path.display());
    println!("    {}", md_path.display());
    println!("    {}", docs_snapshot.display());

    if check {
        check_thresholds(root, &report)?;
    }
    Ok(())
}

#[derive(Debug, Default, serde::Deserialize)]
struct ThresholdsFile {
    echo_rtt: Option<EchoRttThresh>,
    blk_read: Option<MinMetric>,
    udp_tx: Option<MinMetric>,
}

#[derive(Debug, serde::Deserialize)]
struct EchoRttThresh {
    max_ns: u64,
}

#[derive(Debug, serde::Deserialize)]
struct MinMetric {
    #[serde(default)]
    min_iops: Option<u64>,
    #[serde(default)]
    min_pps: Option<u64>,
}

fn check_thresholds(root: &Path, report: &BenchReport) -> Result<()> {
    let path = root.join("support/bench-thresholds.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let thr: ThresholdsFile = toml::from_str(&text).context("parse bench-thresholds.toml")?;
    let mut failures = Vec::new();
    for r in &report.results {
        match r.metric.as_str() {
            "echo_rtt" => {
                if let Some(t) = &thr.echo_rtt
                    && r.value > t.max_ns
                {
                    failures.push(format!(
                        "echo_rtt {} ns exceeds max_ns {}",
                        r.value, t.max_ns
                    ));
                }
            }
            "blk_read" => {
                if let Some(min) = thr.blk_read.as_ref().and_then(|t| t.min_iops)
                    && r.value < min
                {
                    failures.push(format!("blk_read {} iops below min_iops {min}", r.value));
                }
            }
            "udp_tx" => {
                if let Some(min) = thr.udp_tx.as_ref().and_then(|t| t.min_pps)
                    && r.value < min
                {
                    failures.push(format!("udp_tx {} pps below min_pps {min}", r.value));
                }
            }
            _ => {}
        }
    }
    if failures.is_empty() {
        println!("==> bench thresholds OK ({})", path.display());
        Ok(())
    } else {
        for f in &failures {
            eprintln!("bench threshold fail: {f}");
        }
        bail!("{} bench threshold failure(s)", failures.len());
    }
}

/// Returns (serial log, wall-clock from first start marker to matching done).
fn run_board_capture(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
) -> Result<(String, Option<WallSpan>)> {
    let ctx = crate::qemu::load_qemu_context(root, board, build_dir, config)?;
    crate::qemu::ensure_qemu_binary(&ctx.root, &ctx.board)?;
    let cmd = crate::qemu::qemu_command(&ctx)?;
    let mut test: SmokeTest = smoke_expects::smoke_test_for_board(root, board)?;
    test.curls.clear();
    capture_smoke(cmd, &test)
}

#[derive(Debug, Clone)]
struct WallSpan {
    kind: String,
    n: u64,
    elapsed: Duration,
    start_line: String,
    done_line: String,
}

fn capture_smoke(mut cmd: Command, test: &SmokeTest) -> Result<(String, Option<WallSpan>)> {
    use std::{
        io::{BufRead, BufReader},
        sync::{Arc, Mutex},
    };

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().context("stdout")?;
    let stderr = child.stderr.take().context("stderr")?;
    let output = Arc::new(Mutex::new(String::new()));
    let wall = Arc::new(Mutex::new(None::<WallSpan>));
    let o1 = Arc::clone(&output);
    let o2 = Arc::clone(&output);
    let w1 = Arc::clone(&wall);
    let w2 = Arc::clone(&wall);

    // stdout
    let t1 = std::thread::spawn({
        let sink = o1;
        let wall_slot = w1;
        move || {
            let mut r = BufReader::new(stdout);
            let mut line = String::new();
            let mut start_at: Option<(Instant, String, String, u64)> = None;
            while r.read_line(&mut line).unwrap_or(0) > 0 {
                print!("{line}");
                let _ = std::io::Write::flush(&mut std::io::stdout());
                if let Ok(mut b) = sink.lock() {
                    b.push_str(&line);
                }
                observe_bench_line(&line, &mut start_at, &wall_slot);
                line.clear();
            }
        }
    });
    // stderr (QEMU often merges)
    let t2 = std::thread::spawn({
        let sink = o2;
        let wall_slot = w2;
        move || {
            let mut r = BufReader::new(stderr);
            let mut line = String::new();
            let mut start_at: Option<(Instant, String, String, u64)> = None;
            while r.read_line(&mut line).unwrap_or(0) > 0 {
                print!("{line}");
                let _ = std::io::Write::flush(&mut std::io::stdout());
                if let Ok(mut b) = sink.lock() {
                    b.push_str(&line);
                }
                observe_bench_line(&line, &mut start_at, &wall_slot);
                line.clear();
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(test.timeout_secs);
    let mut remaining = test.expects.clone();
    let result = loop {
        if remaining.is_empty() {
            break Ok(());
        }
        if Instant::now() >= deadline {
            break Err(anyhow::anyhow!(
                "timed out waiting for: {}",
                remaining.join(", ")
            ));
        }
        if let Ok(buf) = output.lock() {
            remaining.retain(|p| !buf.contains(p.as_str()));
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let _ = child.kill();
    let _ = child.wait();
    let _ = t1.join();
    let _ = t2.join();
    result?;
    println!("\n==> smoke test passed");
    let buf = output.lock().map(|b| b.clone()).unwrap_or_default();
    let span = wall.lock().ok().and_then(|g| g.clone());
    Ok((buf, span))
}

fn observe_bench_line(
    line: &str,
    start_at: &mut Option<(Instant, String, String, u64)>,
    wall_slot: &std::sync::Arc<std::sync::Mutex<Option<WallSpan>>>,
) {
    if let Some(kind) = bench_kind_start(line) {
        let n = extract_u64(line, "n=").unwrap_or(0);
        *start_at = Some((Instant::now(), kind, line.trim().to_string(), n));
        return;
    }
    if let Some(kind) = bench_kind_done(line)
        && let Some((t0, start_kind, start_line, n)) = start_at.take()
        && start_kind == kind
    {
        let elapsed = t0.elapsed();
        if let Ok(mut slot) = wall_slot.lock() {
            *slot = Some(WallSpan {
                kind,
                n,
                elapsed,
                start_line,
                done_line: line.trim().to_string(),
            });
        }
    }
}

fn bench_kind_start(line: &str) -> Option<String> {
    if line.contains("lerux-bench: echo start") {
        Some("echo".into())
    } else if line.contains("lerux-bench: blk_read start") {
        Some("blk_read".into())
    } else if line.contains("lerux-bench: udp_tx start") {
        Some("udp_tx".into())
    } else {
        None
    }
}

fn bench_kind_done(line: &str) -> Option<String> {
    if line.contains("lerux-bench: echo done") {
        Some("echo".into())
    } else if line.contains("lerux-bench: blk_read done") {
        Some("blk_read".into())
    } else if line.contains("lerux-bench: udp_tx done") {
        Some("udp_tx".into())
    } else {
        None
    }
}

fn derive_metrics(board: &str, _log: &str, wall: Option<WallSpan>) -> Result<Vec<BenchResult>> {
    let Some(span) = wall else {
        bail!("missing start/done wall-clock span for {board}");
    };
    let total_ns = span.elapsed.as_nanos().min(u128::from(u64::MAX)) as u64;
    let total_ns = total_ns.max(1);
    let n = span.n.max(1);
    let (metric, value, unit) = match span.kind.as_str() {
        "echo" => ("echo_rtt".into(), total_ns / n, "ns".into()),
        "blk_read" => (
            "blk_read".into(),
            (n * 1_000_000_000) / total_ns,
            "iops".into(),
        ),
        "udp_tx" => (
            "udp_tx".into(),
            (n * 1_000_000_000) / total_ns,
            "pps".into(),
        ),
        other => bail!("unknown bench kind {other}"),
    };
    Ok(vec![BenchResult {
        board: board.into(),
        metric,
        value,
        unit,
        n,
        total_ns,
        raw_start: span.start_line,
        raw_done: span.done_line,
    }])
}

fn extract_u64(line: &str, key: &str) -> Option<u64> {
    let idx = line.find(key)?;
    let rest = &line[idx + key.len()..];
    let s = rest.split(|c: char| c.is_whitespace() || c == ',').next()?;
    s.parse().ok()
}

fn capture_cmd(args: &[&str]) -> Result<String> {
    let out = Command::new(args[0])
        .args(&args[1..])
        .output()
        .with_context(|| format!("run {}", args.join(" ")))?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn capture_qemu_version(root: &Path) -> String {
    let candidates = [
        root.join("deps/toolchains/qemu/bin/qemu-system-aarch64"),
        root.join("deps/toolchains/qemu-sp804/bin/qemu-system-aarch64"),
        PathBuf::from("qemu-system-aarch64"),
    ];
    for c in candidates {
        if let Ok(out) = Command::new(&c).arg("--version").output()
            && out.status.success()
        {
            let s = String::from_utf8_lossy(&out.stdout);
            if let Some(line) = s.lines().next() {
                return line.trim().to_string();
            }
        }
    }
    "qemu-system-aarch64 (version unknown)".into()
}

fn render_markdown(report: &BenchReport) -> String {
    let mut s = String::new();
    s.push_str("# lerux microbench results (Phase 49)\n\n");
    s.push_str(&format!("Generated (unix): {}\n\n", report.generated_unix));
    s.push_str("## Host\n\n");
    s.push_str(&format!("- **uname:** `{}`\n", report.host.uname));
    s.push_str(&format!("- **QEMU:** `{}`\n", report.host.qemu_version));
    s.push_str(&format!("- **Note:** {}\n\n", report.host.note));
    s.push_str("## Results\n\n");
    s.push_str("| Board | Metric | Value | Unit | n | total_ns |\n");
    s.push_str("|-------|--------|------:|------|--:|--------:|\n");
    for r in &report.results {
        s.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} |\n",
            r.board, r.metric, r.value, r.unit, r.n, r.total_ns
        ));
    }
    s.push_str("\n## Markers\n\n");
    for r in &report.results {
        s.push_str(&format!("- start: `{}`\n", r.raw_start));
        s.push_str(&format!("- done: `{}`\n", r.raw_done));
    }
    s.push_str(
        "\n## Repro\n\n```bash\njust bench\n# or: cargo run -q -p lerux-cli -- bench\n```\n",
    );
    s.push_str(
        "\nHost wall-clock between guest markers on QEMU TCG — compare relatively on the same machine.\n",
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_echo() {
        let span = WallSpan {
            kind: "echo".into(),
            n: 1000,
            elapsed: Duration::from_millis(50),
            start_line: "start".into(),
            done_line: "done".into(),
        };
        let r = derive_metrics("b", "", Some(span)).unwrap();
        assert_eq!(r[0].metric, "echo_rtt");
        assert_eq!(r[0].unit, "ns");
        assert!(r[0].value > 0);
    }

    #[test]
    fn kind_detect() {
        assert_eq!(
            bench_kind_start("INFO lerux-bench: echo start n=1000").as_deref(),
            Some("echo")
        );
        assert_eq!(
            bench_kind_done("lerux-bench: blk_read done n=500").as_deref(),
            Some("blk_read")
        );
    }
}
