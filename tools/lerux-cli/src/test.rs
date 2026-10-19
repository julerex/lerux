use std::{
    collections::VecDeque,
    fs::OpenOptions,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
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
    /// After boot expects, optional write/expect pairs on the guest serial.
    pub script: Vec<ScriptStep>,
    pub script_timeout_secs: u64,
    /// QEMU `send-key` qcodes, injected after boot expects when the board opens QMP.
    pub qmp_keys: Vec<String>,
    /// Substring expected on the serial log after [`Self::qmp_keys`].
    pub qmp_expect: Option<String>,
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
            qmp_keys: Vec::new(),
            qmp_expect: None,
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
    cmd: Command,
    test: &SmokeTest,
    save_log: Option<&std::path::Path>,
) -> Result<()> {
    run_smoke_inner(cmd, test, save_log, None)
}

/// Same as [`run_smoke_with_capture`], then inject PS/2 keys over QMP.
pub fn run_smoke_with_qmp(
    cmd: Command,
    test: &SmokeTest,
    save_log: Option<&Path>,
    qmp_socket: &Path,
) -> Result<()> {
    run_smoke_inner(cmd, test, save_log, Some(qmp_socket))
}

fn run_smoke_inner(
    mut cmd: Command,
    test: &SmokeTest,
    save_log: Option<&Path>,
    qmp_socket: Option<&Path>,
) -> Result<()> {
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    // An empty script keeps the inherited stdin. Closing the pipe early makes QEMU see end of file.
    if !test.script.is_empty() {
        cmd.stdin(Stdio::piped());
    }
    let mut child = cmd.spawn()?;
    let mut script_in = if test.script.is_empty() {
        None
    } else {
        Some(child.stdin.take().context("child stdin pipe")?)
    };

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
        if !test.qmp_keys.is_empty() {
            let socket = qmp_socket.context("qmp_keys set but this run has no QMP socket")?;
            let expect = test
                .qmp_expect
                .as_deref()
                .context("qmp_keys set without qmp_expect")?;
            println!("==> injecting {} PS/2 key(s) via QMP…", test.qmp_keys.len());
            send_qmp_keys(socket, &test.qmp_keys)?;
            expect_ordered(
                &output,
                &[expect.to_string()],
                test.script_timeout_secs.max(30),
            )?;
            println!("==> PS/2 line matched");
        }
        if !test.script.is_empty() {
            let stdin = script_in.as_mut().context("script stdin")?;
            run_script_steps(
                &output,
                &test.script,
                test.script_timeout_secs,
                qmp_socket,
                |bytes| {
                    stdin.write_all(bytes)?;
                    stdin.flush()?;
                    Ok(())
                },
            )?;
        }
        println!("\n==> smoke test passed");
        Ok(())
    })();

    let _ = child.kill();
    drop(script_in);
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

/// QEMU qcodes are a closed token set. Reject anything that would break the JSON.
fn qmp_key_ok(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn send_qmp_keys(socket: &Path, keys: &[String]) -> Result<()> {
    use std::os::unix::net::UnixStream;

    let started = Instant::now();
    let mut stream = loop {
        match UnixStream::connect(socket) {
            Ok(stream) => break stream,
            Err(err) => {
                if started.elapsed() >= Duration::from_secs(5) {
                    return Err(err).with_context(|| format!("connect QMP {}", socket.display()));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };
    // The shell logs the prompt before its handler is running. Give init time
    // to return so the keyboard notification is not dropped on the floor.
    std::thread::sleep(Duration::from_millis(200));

    let cloned = stream.try_clone().context("clone QMP socket")?;
    cloned
        .set_read_timeout(Some(Duration::from_secs(2)))
        .context("qmp read timeout")?;
    let mut reader = BufReader::new(cloned);
    let mut line = String::new();
    reader.read_line(&mut line).context("QMP greeting")?;
    qmp_exec(
        &mut stream,
        &mut reader,
        &mut line,
        "{\"execute\":\"qmp_capabilities\"}",
    )?;
    for key in keys {
        if !qmp_key_ok(key) {
            bail!("refusing QMP key {key:?}");
        }
        let cmd = format!(
            "{{\"execute\":\"send-key\",\"arguments\":{{\"keys\":[{{\"type\":\"qcode\",\"data\":\"{key}\"}}]}}}}"
        );
        qmp_exec(&mut stream, &mut reader, &mut line, &cmd)
            .with_context(|| format!("QMP send-key {key}"))?;
        std::thread::sleep(Duration::from_millis(40));
    }
    Ok(())
}

fn wait_for_mark(
    output: &std::sync::Arc<std::sync::Mutex<String>>,
    mark: usize,
    needle: &str,
    deadline: Instant,
) -> Result<()> {
    loop {
        let found = output
            .lock()
            .map(|capture| capture.len() > mark && capture_contains(&capture[mark..], needle))
            .unwrap_or(false);
        if found {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("timed out waiting for {needle:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn qmp_system_reset(socket: &Path) -> Result<()> {
    use std::os::unix::net::UnixStream;

    let started = Instant::now();
    let mut stream = loop {
        match UnixStream::connect(socket) {
            Ok(stream) => break stream,
            Err(err) => {
                if started.elapsed() >= Duration::from_secs(5) {
                    return Err(err).with_context(|| format!("connect QMP {}", socket.display()));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };
    let cloned = stream.try_clone().context("clone QMP socket")?;
    cloned
        .set_read_timeout(Some(Duration::from_secs(2)))
        .context("qmp read timeout")?;
    let mut reader = BufReader::new(cloned);
    let mut line = String::new();
    reader.read_line(&mut line).context("QMP greeting")?;
    qmp_exec(
        &mut stream,
        &mut reader,
        &mut line,
        "{\"execute\":\"qmp_capabilities\"}",
    )?;
    qmp_exec(
        &mut stream,
        &mut reader,
        &mut line,
        "{\"execute\":\"system_reset\"}",
    )?;
    Ok(())
}

fn qmp_exec(
    stream: &mut impl Write,
    reader: &mut impl BufRead,
    line: &mut String,
    cmd: &str,
) -> Result<()> {
    stream.write_all(cmd.as_bytes()).context("write QMP")?;
    stream.write_all(b"\n").context("write QMP newline")?;
    stream.flush().context("flush QMP")?;
    line.clear();
    reader.read_line(line).context("read QMP reply")?;
    if line.contains("\"error\"") {
        bail!("QMP error: {line}");
    }
    Ok(())
}

fn ordered_hit(raw: &str, pattern: &str, cursor: usize) -> Option<usize> {
    let raw_end = raw
        .get(cursor..)
        .and_then(|suffix| suffix.find(pattern).map(|at| cursor + at + pattern.len()));
    let traced = collapse_traced(raw);
    let collapsed_end = earliest_collapsed_end(&traced.text, &traced.src, pattern, cursor);
    match (raw_end, collapsed_end) {
        (Some(raw_end), Some(collapsed_end)) => Some(raw_end.min(collapsed_end)),
        (Some(end), None) | (None, Some(end)) => Some(end),
        (None, None) => None,
    }
}

fn earliest_collapsed_end(
    text: &str,
    src: &[Option<usize>],
    pattern: &str,
    cursor: usize,
) -> Option<usize> {
    debug_assert_eq!(text.len(), src.len());
    let mut from = 0;
    let mut best: Option<usize> = None;
    while from < text.len() {
        let Some(at) = text[from..].find(pattern) else {
            break;
        };
        let abs = from + at;
        let end = abs + pattern.len();
        if let Some(raw_end) = real_raw_end(&src[abs..end])
            && raw_end > cursor
        {
            best = Some(best.map_or(raw_end, |have| have.min(raw_end)));
        }
        let Some(ch) = text[abs..].chars().next() else {
            break;
        };
        from = abs + ch.len_utf8();
    }
    best
}

fn real_raw_end(src: &[Option<usize>]) -> Option<usize> {
    src.iter().copied().flatten().max().map(|at| at + 1)
}

fn expect_ordered(
    output: &std::sync::Arc<std::sync::Mutex<String>>,
    patterns: &[String],
    per: u64,
) -> Result<()> {
    let mut cursor = 0;
    for pattern in patterns {
        let deadline = Instant::now() + Duration::from_secs(per);
        loop {
            let hit = output
                .lock()
                .ok()
                .and_then(|buf| ordered_hit(&buf, pattern, cursor));
            if let Some(end) = hit {
                cursor = end;
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
        run_script_steps(
            &output,
            &test.script,
            test.script_timeout_secs,
            None,
            |bytes| {
                writer
                    .write_all(bytes)
                    .with_context(|| "write serial step")?;
                writer.flush().context("flush serial")?;
                Ok(())
            },
        )?;
    }

    // Detach: reader may block on serial; we don't join forever.
    drop(reader_thread);
    println!("\n==> hardware serial smoke passed");
    Ok(())
}

/// Write each script step and wait until the capture after the send contains `expect`.
fn run_script_steps(
    output: &std::sync::Arc<std::sync::Mutex<String>>,
    script: &[ScriptStep],
    timeout_secs: u64,
    qmp_socket: Option<&Path>,
    mut send: impl FnMut(&[u8]) -> Result<()>,
) -> Result<()> {
    println!("==> running {} scripted serial step(s)…", script.len());
    for (i, step) in script.iter().enumerate() {
        let mark = output.lock().map(|s| s.len()).unwrap_or(0);
        print!(
            "    [{}] send {:?} expect {:?}… ",
            i + 1,
            step.send.trim_end_matches(['\r', '\n']),
            step.expect
        );
        let _ = std::io::Write::flush(&mut std::io::stdout());
        send(step.send.as_bytes()).with_context(|| format!("write serial step {}", i + 1))?;
        let step_deadline = Instant::now() + Duration::from_secs(timeout_secs);
        if step.send.starts_with("reboot") {
            let socket = qmp_socket.context("a reboot step needs a QMP socket")?;
            wait_for_mark(output, mark, "reboot", step_deadline).with_context(|| {
                format!(
                    "script step {} timed out waiting for the guest reboot line",
                    i + 1
                )
            })?;
            qmp_system_reset(socket).context("QMP system_reset")?;
        }
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
    Ok(())
}

const SEL4_LOG_LEVELS: [&str; 5] = ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"];

fn starts_with_sel4_log_level(s: &str) -> bool {
    SEL4_LOG_LEVELS.iter().any(|l| s.starts_with(*l))
}

/// sel4-logging line prefix: `{level:<5} [{target}] `.
///
/// `target` is `[A-Za-z0-9_:]+` so a split prefix like
/// `INFO  [virtio_drivers::device::net::dev_INFO  [virtio_drivers::device::blk]`
/// is not treated as one prefix (the inner `[blk]` `]` would otherwise match).
///
/// A trailing space after `]` is the usual terminator. Concurrent
/// `debug_putchar` may splice the next prefix immediately (`]INFO`), or the
/// capture may end mid-prefix; those still count so the outer line stays in
/// the unfinished FIFO.
fn sel4_log_prefix_len(s: &str) -> Option<usize> {
    let level = SEL4_LOG_LEVELS.iter().find(|l| s.starts_with(*l))?;
    let after_level = &s[level.len()..];
    let spaces = after_level.bytes().take_while(|&b| b == b' ').count();
    if spaces == 0 {
        return None;
    }
    let after_spaces = &after_level[spaces..];
    let rest = after_spaces.strip_prefix('[')?;
    let close = rest.find(']')?;
    let target = &rest[..close];
    if target.is_empty()
        || !target
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
    {
        return None;
    }
    let after_brack = &rest[close + 1..];
    let without_space = level.len() + spaces + 1 + close + 1;
    if after_brack.starts_with(' ') {
        Some(without_space + 1)
    } else if after_brack.is_empty() || starts_with_sel4_log_level(after_brack) {
        Some(without_space)
    } else {
        None
    }
}

fn is_prefix_only(s: &str) -> bool {
    sel4_log_prefix_len(s).is_some_and(|n| n == s.len())
}

struct Traced {
    text: String,
    src: Vec<Option<usize>>,
}

impl Traced {
    fn new() -> Self {
        Self {
            text: String::new(),
            src: Vec::new(),
        }
    }

    fn push_raw(&mut self, bytes: &str, raw_at: usize) {
        self.text.push_str(bytes);
        self.src
            .extend((0..bytes.len()).map(|offset| Some(raw_at + offset)));
    }

    fn push_copied(&mut self, other: &Traced, range: std::ops::Range<usize>) {
        self.text.push_str(&other.text[range.clone()]);
        self.src.extend_from_slice(&other.src[range]);
    }

    fn push_char(&mut self, ch: char, raw_at: usize) {
        let before = self.text.len();
        self.text.push(ch);
        let width = self.text.len() - before;
        self.src
            .extend((0..width).map(|offset| Some(raw_at + offset)));
    }

    fn push_inserted(&mut self, ch: char) {
        self.text.push(ch);
        self.src.resize(self.text.len(), None);
    }

    fn append(&mut self, other: &Traced) {
        self.text.push_str(&other.text);
        self.src.extend_from_slice(&other.src);
    }
}

/// When an outer prefix was preempted before any payload, the first newline
/// often belongs to that outer line (`serial driver: PL011`) even though the
/// inner line is still current (`lerux-debug:`). Split the inner payload at
/// the first space so the tail completes the oldest prefix-only line.
fn steal_preempted_outer_payload(
    current: &Traced,
    oldest: Option<&Traced>,
) -> Option<(Traced, Traced)> {
    let oldest = oldest?;
    if !is_prefix_only(&oldest.text) {
        return None;
    }
    let plen = sel4_log_prefix_len(&current.text)?;
    let payload = &current.text[plen..];
    let sp = payload.find(' ')?;
    if sp == 0 {
        return None;
    }
    let tail_at = plen + sp + 1;
    if tail_at >= current.text.len() {
        return None;
    }
    let mut inner = Traced::new();
    inner.push_copied(current, 0..plen + sp);
    let mut tail = Traced::new();
    tail.push_copied(current, tail_at..current.text.len());
    Some((inner, tail))
}

/// Text of the next physical line, not including its newline.
fn rest_of_line(s: &str) -> &str {
    match s.find('\n') {
        Some(n) => &s[..n],
        None => s,
    }
}

fn trailing_alnum_len(s: &str) -> usize {
    s.bytes()
        .rev()
        .take_while(|b| b.is_ascii_alphanumeric())
        .count()
}

fn leading_alnum_len(s: &str) -> usize {
    s.bytes().take_while(|b| b.is_ascii_alphanumeric()).count()
}

fn mid_word_join(left: &str, right: &str) -> bool {
    matches!(
        (left.as_bytes().last(), right.as_bytes().first()),
        (Some(l), Some(r)) if l.is_ascii_alphanumeric() && r.is_ascii_alphanumeric()
    )
}

/// Outer line resumed inside the inner line and hit its newline before the
/// inner line finished. `current` then holds `inner head + outer tail`, and
/// `after` (the next physical line) is the inner tail.
///
/// Splits inside one alphanumeric run tie on healed length (`bloc|irtio`
/// versus `blocir|tio`). Emit every max-scoring split so the real cut, which
/// rebuilds `blocks` and `virtio-net`, is one of them.
///
/// The inner tail may begin on a space instead of continuing that run
/// (`512|tio` with remainder ` bytes`). The cut is still inside `512tio`, so
/// the same max-score set includes it.
fn resumed_outer_lines(
    oldest: &Traced,
    current: &Traced,
    after: &str,
    after_at: usize,
    outer_newline_at: usize,
) -> Vec<Traced> {
    let Some(old_pre) = sel4_log_prefix_len(&oldest.text) else {
        return Vec::new();
    };
    let Some(cur_pre) = sel4_log_prefix_len(&current.text) else {
        return Vec::new();
    };
    if sel4_log_prefix_len(after).is_some() {
        return Vec::new();
    }
    let old_pay = &oldest.text[old_pre..];
    let cur_pay = &current.text[cur_pre..];
    if old_pay.is_empty() || cur_pay.is_empty() || after.is_empty() {
        return Vec::new();
    }
    let mut best = 0usize;
    let mut splits = Vec::new();
    for s in 1..cur_pay.len() {
        if !cur_pay.is_char_boundary(s) {
            continue;
        }
        let b1 = &cur_pay[..s];
        let a2 = &cur_pay[s..];
        // The cut sits inside one alphanumeric run. The inner tail either
        // continues that run or starts at whitespace (`512` then ` bytes`).
        if !mid_word_join(old_pay, a2) || !mid_word_join(b1, a2) {
            continue;
        }
        if !mid_word_join(b1, after) && !after.starts_with(char::is_whitespace) {
            continue;
        }
        let score = trailing_alnum_len(old_pay)
            + leading_alnum_len(a2)
            + trailing_alnum_len(b1)
            + leading_alnum_len(after);
        if score > best {
            best = score;
            splits.clear();
            splits.push(s);
        } else if score == best {
            splits.push(s);
        }
    }
    let mut lines = Vec::with_capacity(splits.len().saturating_mul(2));
    for s in splits {
        let mut outer = Traced::new();
        outer.append(oldest);
        outer.push_copied(current, cur_pre + s..current.text.len());
        outer.push_char('\n', outer_newline_at);
        let mut inner = Traced::new();
        inner.push_copied(current, 0..cur_pre + s);
        inner.push_raw(after, after_at);
        inner.push_inserted('\n');
        lines.push(outer);
        lines.push(inner);
    }
    lines
}

/// Rebuild sel4-logging lines after concurrent `debug_putchar` spliced one
/// message inside another.
///
/// Incomplete lines are a FIFO: a new `LEVEL [target] ` starts a line; a
/// newline completes the current line; leftover text that is not a prefix
/// continues the oldest unfinished line. That recovers both simple splices
/// (`virtio-` / `net: MAC`) and nested ones (`lerux-edit: ` / `lerux-` /
/// complete backup line / later `ready` then `chat: ready`).
///
/// If the oldest unfinished line is still prefix-only, the first newline on
/// the inner line is treated as the outer resuming (`]INFO` then
/// `lerux-debug: serial driver: PL011` / ` ready`).
///
/// If the outer line reaches its newline while the inner line is still open
/// (`v` / `bloc` / `irtio-net: MAC` / `ks x 512 bytes`, or `vir` / `512` /
/// `tio-net: MAC` / ` bytes`), every max-scoring mid-word split of that
/// shape is kept as well.
fn collapse_interleaved_sel4_logs(haystack: &str) -> String {
    collapse_traced(haystack).text
}

fn collapse_traced(haystack: &str) -> Traced {
    let mut completed = Traced::new();
    let mut incomplete: VecDeque<Traced> = VecDeque::new();
    let mut current: Option<Traced> = None;
    let mut i = 0;
    while i < haystack.len() {
        if let Some(plen) = sel4_log_prefix_len(&haystack[i..]) {
            if let Some(cur) = current.take() {
                incomplete.push_back(cur);
            }
            let mut started = Traced::new();
            started.push_raw(&haystack[i..i + plen], i);
            current = Some(started);
            i += plen;
            continue;
        }
        let Some(ch) = haystack[i..].chars().next() else {
            break;
        };
        let ch_at = i;
        i += ch.len_utf8();
        if let Some(mut cur) = current.take() {
            if ch == '\n' {
                if let Some(oldest) = incomplete.back() {
                    let after = rest_of_line(&haystack[i..]);
                    for line in resumed_outer_lines(oldest, &cur, after, i, ch_at) {
                        completed.append(&line);
                    }
                }
                if let Some((inner, outer_tail)) =
                    steal_preempted_outer_payload(&cur, incomplete.front())
                {
                    if let Some(front) = incomplete.front_mut() {
                        if !front.text.ends_with(' ') && !outer_tail.text.starts_with(' ') {
                            front.push_inserted(' ');
                        }
                        front.append(&outer_tail);
                        front.push_char('\n', ch_at);
                    }
                    if let Some(done) = incomplete.pop_front() {
                        completed.append(&done);
                    }
                    current = Some(inner);
                } else {
                    completed.append(&cur);
                    completed.push_char('\n', ch_at);
                }
            } else {
                cur.push_char(ch, ch_at);
                current = Some(cur);
            }
        } else if incomplete.front().is_some() {
            if let Some(front) = incomplete.front_mut() {
                front.push_char(ch, ch_at);
            }
            if ch == '\n'
                && let Some(done) = incomplete.pop_front()
            {
                completed.append(&done);
            }
        } else {
            completed.push_char(ch, ch_at);
        }
    }
    if let Some(cur) = current {
        incomplete.push_back(cur);
    }
    for line in incomplete {
        completed.append(&line);
    }
    debug_assert_eq!(completed.text.len(), completed.src.len());
    completed
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
        let iso_line = if ctx.board.arch == "x86_64" {
            "   ISO: just iso\n"
        } else {
            ""
        };
        println!(
            "==> Hardware board {board:?}: image built successfully.\n\
             {iso_line}\
             \x20   No QEMU profile.\n\
             \x20   Deploy: lerux deploy --board {board} --dest /abs/path/to/boot\n\
             \x20   Golden path: LERUX_HW_SERIAL=/dev/ttyUSB0 BOARD={board} just test-hw\n\
             \x20   Install path: docs/boards.md"
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

    let helpers = crate::qemu::setup_test_helpers(&ctx)?;
    let cmd = crate::qemu::qemu_command(&ctx)?;
    let test = crate::smoke_expects::smoke_test_for_board(root, board)?;

    // Phase 57: always capture serial under build/smoke-logs/ for diagnose.
    let log_path = root
        .join(build_dir)
        .join("smoke-logs")
        .join(format!("{board}.serial.log"));
    let script_reboots = test
        .script
        .iter()
        .any(|step| step.send.starts_with("reboot"));
    let result = if test.qmp_keys.is_empty() && !script_reboots {
        run_smoke_with_capture(cmd, &test, Some(&log_path))
    } else {
        let qemu = ctx
            .board
            .qemu()
            .context("this smoke needs a QEMU board with QMP")?;
        if !qemu.qmp {
            bail!("board {board:?} smoke needs QMP but qemu.qmp is false");
        }
        let socket = crate::qemu::qmp_socket(&ctx);
        run_smoke_with_qmp(cmd, &test, Some(&log_path), &socket)
    };
    for mut child in helpers {
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

    /// 2026-09-25 ipc-composed CI: the outer MAC line reaches its newline
    /// before blk-server finishes, so `v` / `irtio-net` and `bloc` / `ks`
    /// are not in FIFO order.
    const IPC_COMPOSED_CI_SNIP_2026_09_25: &str = "\
INFO  [net_server] vINFO  [blk_server] virtio-blk: 8192 blocirtio-net: MAC 52:54:00:12:34:56
ks x 512 bytes
";

    #[test]
    fn qmp_keys_reject_json_breakers() {
        assert!(qmp_key_ok("ret"));
        assert!(qmp_key_ok("spc"));
        assert!(!qmp_key_ok(""));
        assert!(!qmp_key_ok("a\"b"));
        assert!(!qmp_key_ok("a b"));
    }

    /// 2026-10-18 ipc-composed CI: the outer MAC line reaches its newline
    /// inside the block line, and the block remainder starts at a space
    /// (`vir` / `512` / `tio-net: MAC` / ` bytes`). Carriage return is what
    /// the serial capture stored.
    const IPC_COMPOSED_CI_SNIP_2026_10_18: &str = "\
INFO  [virtio_net_driver] virtio-net: unified-dma (no client_dma map)\r
INFO  [virtio_driversINFO  [virtio_drivers::device::blk] found a block device of size 4096KB\r
::device::net::dev_raw] negotiated_features Features(MAC | STATUS | RING_INDIRECT_DESC | RING_EVENT_IDX)\r
INFO  [virtio_blk_driver] virtio-blk driver ready\r
INFO  [virtio_net_driver] virtio-net driver ready\r
INFO  [net_server] virINFO  [blk_server] virtio-blk: 8192 blocks x 512tio-net: MAC 52:54:00:12:34:56\r
 bytes\r
INFO  [blk_server] lerux-blk: ready\r
INFO  [net_server] lerux-net: ready\r
";

    /// 2026-08-20 ipc-composed CI: earlier split prefix must not steal the
    /// `net: MAC` leftover from `virtio-`.
    const IPC_COMPOSED_CI_SNIP_2026_08_20: &str = "\
INFO  [virtio_net_driver] virtio-net: unified-dma (no client_dma map)
INFO  [virtio_drivers::device::net::dev_INFO  [virtio_drivers::device::blk] found a block device of size 4096KB
raw] negotiated_features Features(MAC | STATUS | RING_INDIRECT_DESC | RING_EVENT_IDX)
INFO  [virtio_blk_driver] virtio-blk driver ready
INFO  [virtio_net_driver] virtio-net driver ready
INFO  [net_server] virtio-INFO  [blk_server] virtio-blk: 8192 blocks x 512 bytes
net: MAC 52:54:00:12:34:56
INFO  [blk_server] lerux-blk: ready
INFO  [net_server] lerux-net: ready
";

    /// 2026-08-20 workstation CI: three PDs splice, remnants arrive after
    /// unrelated complete lines.
    const WORKSTATION_CI_SNIP: &str = "\
INFO  [edit] lerux-edit: INFO  [chat_client] lerux-INFO  [backup] lerux-backup: ready
INFO  [http_file_browser] lerux-http-fs: listening
INFO  [http_file_browser] lerux-http-fs: ready (v2 mime/put)
ready
chat: ready rooms=lobby
";

    /// 2026-08-20 isolation CI: `INFO  [serial_driver]` splices onto
    /// `debug_handler` with no space after `]`, and serial's payload plus
    /// newline arrive while the debug line is still open.
    const ISOLATION_CI_SNIP: &str = "\
INFO  [serial_driver]INFO  [debug_handler] lerux-debug: serial driver: PL011
 ready (parent fault handler)
INFO  [crash_demo] crash-demo: startiINFO  [fs_client] lerux-isolation: waiting for untrung
INFO  [crash_demo] crash-demo: about to fault
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
    fn capture_contains_recovers_mac_when_outer_newline_arrives_first() {
        assert!(
            !IPC_COMPOSED_CI_SNIP_2026_09_25.contains("virtio-net: MAC"),
            "fixture must reproduce the raw split"
        );
        assert!(
            !IPC_COMPOSED_CI_SNIP_2026_09_25.contains("8192 blocks"),
            "fixture must reproduce the raw split"
        );
        assert!(capture_contains(
            IPC_COMPOSED_CI_SNIP_2026_09_25,
            "virtio-net: MAC"
        ));
        assert!(capture_contains(
            IPC_COMPOSED_CI_SNIP_2026_09_25,
            "virtio-blk: 8192 blocks"
        ));
    }

    #[test]
    fn capture_contains_recovers_mac_when_inner_tail_starts_at_a_space() {
        assert!(
            !IPC_COMPOSED_CI_SNIP_2026_10_18.contains("virtio-net: MAC"),
            "fixture must reproduce the raw split"
        );
        assert!(capture_contains(
            IPC_COMPOSED_CI_SNIP_2026_10_18,
            "virtio-net: MAC"
        ));
        assert!(capture_contains(
            IPC_COMPOSED_CI_SNIP_2026_10_18,
            "virtio-blk: 8192 blocks"
        ));
        assert!(capture_contains(
            IPC_COMPOSED_CI_SNIP_2026_10_18,
            "lerux-net: ready"
        ));
    }

    #[test]
    fn capture_contains_recovers_mac_when_earlier_prefix_also_splits() {
        assert!(
            !IPC_COMPOSED_CI_SNIP_2026_08_20.contains("virtio-net: MAC"),
            "fixture must reproduce the raw split"
        );
        assert!(capture_contains(
            IPC_COMPOSED_CI_SNIP_2026_08_20,
            "virtio-net: MAC"
        ));
        assert!(capture_contains(
            IPC_COMPOSED_CI_SNIP_2026_08_20,
            "lerux-net: ready"
        ));
    }

    #[test]
    fn capture_contains_recovers_nested_edit_and_chat_ready() {
        assert!(
            !WORKSTATION_CI_SNIP.contains("lerux-edit: ready"),
            "fixture must reproduce the raw split"
        );
        assert!(
            !WORKSTATION_CI_SNIP.contains("lerux-chat: ready"),
            "fixture must reproduce the raw split"
        );
        assert!(capture_contains(WORKSTATION_CI_SNIP, "lerux-edit: ready"));
        assert!(capture_contains(WORKSTATION_CI_SNIP, "lerux-chat: ready"));
        assert!(capture_contains(WORKSTATION_CI_SNIP, "lerux-backup: ready"));
        assert!(capture_contains(WORKSTATION_CI_SNIP, "v2 mime"));
    }

    #[test]
    fn capture_contains_recovers_debug_ready_when_serial_prefix_splices() {
        assert!(
            !ISOLATION_CI_SNIP.contains("lerux-debug: ready"),
            "fixture must reproduce the raw split"
        );
        assert!(capture_contains(ISOLATION_CI_SNIP, "lerux-debug: ready"));
        assert!(capture_contains(
            ISOLATION_CI_SNIP,
            "crash-demo: about to fault"
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

    #[test]
    fn reversed_program_lines_fail() {
        let output = std::sync::Arc::new(std::sync::Mutex::new(
            "lerux-wasm: ran\nprogram: rejected\n".to_string(),
        ));
        let patterns = vec![
            "program: rejected".to_string(),
            "lerux-wasm: ran".to_string(),
        ];
        let result = expect_ordered(&output, &patterns, 0);
        assert!(result.is_err(), "reversed lines must not match");
    }

    #[test]
    fn in_order_program_lines_pass() {
        let output = std::sync::Arc::new(std::sync::Mutex::new(
            "program: rejected\nlerux-wasm: ran\n".to_string(),
        ));
        let patterns = vec![
            "program: rejected".to_string(),
            "lerux-wasm: ran".to_string(),
        ];
        expect_ordered(&output, &patterns, 0).expect("in-order lines must match");
    }

    #[test]
    fn spliced_token_after_earlier_line_passes() {
        let capture = format!("early-line\n{IPC_COMPOSED_CI_SNIP}");
        assert!(
            !capture.contains("virtio-net: MAC"),
            "fixture must reproduce the raw split"
        );
        let output = std::sync::Arc::new(std::sync::Mutex::new(capture));
        let patterns = vec!["early-line".to_string(), "virtio-net: MAC".to_string()];
        expect_ordered(&output, &patterns, 0).expect("spliced token after an earlier line");
    }

    #[test]
    fn spliced_token_ending_before_cursor_fails() {
        let capture = format!("{IPC_COMPOSED_CI_SNIP}later-marker\n");
        assert!(
            !capture.contains("virtio-net: MAC"),
            "fixture must reproduce the raw split"
        );
        let output = std::sync::Arc::new(std::sync::Mutex::new(capture));
        let patterns = vec!["later-marker".to_string(), "virtio-net: MAC".to_string()];
        let result = expect_ordered(&output, &patterns, 0);
        assert!(
            result.is_err(),
            "spliced token before the cursor must not match"
        );
    }

    #[test]
    fn unordered_expects_pass_in_either_order() {
        let patterns = vec!["alpha-token".to_string(), "beta-token".to_string()];
        for capture in ["alpha-token\nbeta-token\n", "beta-token\nalpha-token\n"] {
            let output = std::sync::Arc::new(std::sync::Mutex::new(capture.to_string()));
            let deadline = Instant::now() + Duration::from_secs(2);
            expect_unordered(&output, &patterns, deadline)
                .expect("unordered list accepts this order");
        }
    }

    #[test]
    fn board_expects_match_guest_serial_order() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cases = [
            (
                "qemu_virt_aarch64_blk",
                "virtio-blk: 8192\nlerux-blk: ready\nlerux-blk: MBR sig\nlerux-blk: write round-trip ok\n",
            ),
            (
                "qemu_virt_aarch64_blk_composed",
                "virtio-blk: 8192\nlerux-blk: ready\nlerux-supervisor: RTC\nlerux-supervisor: timer ok\nlerux-supervisor: init ok\nlerux-blk: MBR sig\nlerux-blk: write round-trip ok\n",
            ),
            (
                "qemu_virt_aarch64_net",
                "virtio-net: MAC\nlerux-net: ready\nlerux-net: TX ok\nlerux-net: IPC ok\nlerux-net: apply static\n",
            ),
            (
                "qemu_virt_aarch64_net_composed",
                "virtio-net: MAC\nlerux-net: ready\nlerux-supervisor: RTC\nlerux-supervisor: timer ok\nlerux-supervisor: init ok\nlerux-net: TX ok\nlerux-net: IPC ok\nlerux-net: apply static\n",
            ),
            (
                "qemu_virt_aarch64_ipc_composed",
                "virtio-net: MAC\nvirtio-blk: 8192\nlerux-blk: ready\nlerux-net: ready\nlerux-supervisor: RTC\nlerux-supervisor: timer ok\nlerux-supervisor: init ok\nlerux-blk: MBR sig\nlerux-blk: write round-trip ok\nlerux-net: TX ok\nlerux-net: IPC ok\n",
            ),
            (
                "qemu_virt_aarch64_fs",
                "virtio-blk: 8192\nlerux-fs: ready\nlerux-fs: v3 20k ok\nlerux-fs: v3 long path ok\nlerux-fs: round-trip ok\n",
            ),
            (
                "qemu_virt_aarch64_agent",
                "virtio-net: MAC\nlerux-fs: ready\nlerux-net: ready\nlerux-agent: chrome ok\nlerux-agent: workspace ok\nlerux-tls: handshake ok\nlerux-agent: tools ok\n",
            ),
            (
                "qemu_virt_aarch64_isolation",
                "lerux-fs: ready\nlerux-debug: ready\ncrash-demo: about to fault\nlerux-debug: crash-demo stopped\nlerux-isolation: notified survivor\nlerux-fs: round-trip ok\nlerux-isolation: fs-server survived untrusted PD crash\n",
            ),
            (
                "qemu_virt_aarch64_browser",
                "virtio-net: MAC\nlerux-net: ready\nlerux-display: ramfb ok\nlerux-tls: handshake ok\nlerux-web: paint ok\nlerux-browser: paint ok\n",
            ),
        ];
        for (board, capture) in cases {
            let spec = crate::smoke_expects::smoke_test_for_board(&root, board)
                .unwrap_or_else(|err| panic!("{board}: {err}"));
            let output = std::sync::Arc::new(std::sync::Mutex::new(capture.to_string()));
            expect_ordered(&output, &spec.expects, 0)
                .unwrap_or_else(|err| panic!("{board}: {err}"));
        }
    }

    #[test]
    fn blk_ready_before_device_line_fails() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let spec = crate::smoke_expects::smoke_test_for_board(&root, "qemu_virt_aarch64_blk")
            .expect("blk expects");
        let output = std::sync::Arc::new(std::sync::Mutex::new(
            "lerux-blk: ready\nvirtio-blk: 8192\nlerux-blk: MBR sig\nlerux-blk: write round-trip ok\n"
                .to_string(),
        ));
        let result = expect_ordered(&output, &spec.expects, 0);
        assert!(
            result.is_err(),
            "a ready line before the device line is not guest order"
        );
    }

    #[test]
    fn script_step_ignores_expect_before_send_mark() {
        let output = std::sync::Arc::new(std::sync::Mutex::new("only-before\n".to_string()));
        let steps = [ScriptStep {
            send: "cmd\r".to_string(),
            expect: "only-before".to_string(),
        }];
        let result = run_script_steps(&output, &steps, 0, None, |_| Ok(()));
        assert!(
            result.is_err(),
            "script step must ignore text before the send mark"
        );
    }
}
