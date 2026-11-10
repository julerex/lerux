//! Same steps as `BtStage.tla`.
//!
//! The painted bytes come from `fail_line`, `version_line`, `bt_line`, and
//! `bt_bring_up`. Which of those runs is read from `swift-bt`. A branch that
//! moves has to change this file and the TLA module together. This program
//! does not execute xHCI.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use lerux_swift::{
    bt_bring_up, bt_line, fail_line, further, version_line, BtReport, PortStop, XferFail,
};

const WITNESS_COUNT: u8 = 3;
const WITNESS_RANK: u8 = 4;
const RESET_OK: &[u8] = b"bt: reset ok";
const FIRMWARE_ERR: &[u8] = b"bt: firmware err";
const ACK: [u8; 7] = [0x04, 0x0e, 0x04, 0x01, 0x03, 0x0c, 0x00];
const BAD_EVENT: [u8; 7] = [0x04, 0x0e, 0x04, 0x01, 0x03, 0x0c, 0x01];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Phase {
    Boot,
    Ports,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Kind {
    None,
    Absent,
    Ver,
    Fw,
    Port,
    Id,
    Event,
    Ok,
    Halt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct World {
    phase: Phase,
    firmware: bool,
    live: bool,
    foreign: bool,
    connected: u8,
    rank: u8,
    peak: u8,
    kind: Kind,
    event_ack: bool,
}

struct Facts {
    only_forward: bool,
    reset_ok_needs_firmware: bool,
    reset_ok_needs_event: bool,
    port_is_not_reset_ok: bool,
    empty_is_firmware_err: bool,
    ver_paints_word: bool,
    id_paints_device: bool,
    stage_labels: bool,
    paints_version: bool,
    paints_firmware: bool,
    paints_bring_up: bool,
    paints_fail: bool,
    witness_count: u8,
    witness_rank: u8,
    ver_lo: u16,
    ver_hi: u16,
}

struct PaintFlags {
    version: bool,
    firmware: bool,
    bring_up: bool,
    fail: bool,
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("probe") => match read_facts(&repo_root()) {
            Ok(facts) => {
                print_probe(&facts);
                ExitCode::SUCCESS
            }
            Err(error) => {
                println!("error={error}");
                ExitCode::from(2)
            }
        },
        Some("check") => check(),
        _ => {
            eprintln!("usage: bt-stage probe|check");
            ExitCode::from(2)
        }
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn print_probe(facts: &Facts) {
    println!("only_forward={}", facts.only_forward);
    println!("reset_ok_needs_firmware={}", facts.reset_ok_needs_firmware);
    println!("reset_ok_needs_event={}", facts.reset_ok_needs_event);
    println!("port_is_not_reset_ok={}", facts.port_is_not_reset_ok);
    println!("empty_is_firmware_err={}", facts.empty_is_firmware_err);
    println!("ver_paints_word={}", facts.ver_paints_word);
    println!("id_paints_device={}", facts.id_paints_device);
    println!("stage_labels={}", facts.stage_labels);
    println!("paints_version={}", facts.paints_version);
    println!("paints_firmware={}", facts.paints_firmware);
    println!("paints_bring_up={}", facts.paints_bring_up);
    println!("paints_fail={}", facts.paints_fail);
    println!("witness_count={}", facts.witness_count);
    println!("witness_rank={}", facts.witness_rank);
    println!("ver_lo={}", facts.ver_lo);
    println!("ver_hi={}", facts.ver_hi);
}

fn check() -> ExitCode {
    let facts = match read_facts(&repo_root()) {
        Ok(facts) => facts,
        Err(error) => {
            println!("error={error}");
            return ExitCode::from(2);
        }
    };
    if !facts.paints_version
        || !facts.paints_firmware
        || !facts.paints_bring_up
        || !facts.paints_fail
    {
        println!(
            "paint=version:{} firmware:{} bring_up:{} fail:{}",
            facts.paints_version, facts.paints_firmware, facts.paints_bring_up, facts.paints_fail
        );
        println!("verdict=paint");
        return ExitCode::from(1);
    }
    let problems = line_problems();
    if !problems.is_empty() {
        for problem in problems {
            println!("line={problem}");
        }
        println!("verdict=line");
        return ExitCode::from(1);
    }
    match explore() {
        Explore::Ok {
            states,
            witness_trace,
            reset_trace,
        } => {
            println!("states={states}");
            println!("witness_trace={witness_trace}");
            println!("witness_line=bt: port 3 desc");
            println!("reset_trace={reset_trace}");
            println!("reset_line=bt: reset ok");
            println!("verdict=ok");
            ExitCode::SUCCESS
        }
        Explore::Line { trace, msg } => {
            println!("trace={trace}");
            println!("line={msg}");
            println!("verdict=line");
            ExitCode::from(1)
        }
        Explore::Model { msg } => {
            println!("error={msg}");
            ExitCode::from(2)
        }
    }
}

fn read_facts(root: &Path) -> Result<Facts, String> {
    let main = read_fn(
        &root.join("userspace/pds/swift-bt/src/main.rs"),
        "bring_up_text",
    )?;
    let live = read_fn(
        &root.join("userspace/pds/swift-bt/src/main.rs"),
        "controllers_live",
    )?;
    let reset = read_fn(
        &root.join("userspace/pds/swift-bt/src/main.rs"),
        "reset_either",
    )?;
    let radio = read_fn(
        &root.join("userspace/pds/swift-bt/src/xhci.rs"),
        "find_radio",
    )?;
    if !uses_further(&reset) {
        return Err("reset_either does not call further".to_string());
    }
    stage_order(&radio)?;
    let (ver_lo, ver_hi) = parse_range(&live)?;
    let paint = paint_flags(&main);
    Ok(Facts {
        only_forward: only_forward(),
        reset_ok_needs_firmware: reset_ok_needs_firmware(),
        reset_ok_needs_event: reset_ok_needs_event(),
        port_is_not_reset_ok: port_is_not_reset_ok(),
        empty_is_firmware_err: empty_is_firmware_err(),
        ver_paints_word: ver_paints_word(),
        id_paints_device: id_paints_device(),
        stage_labels: stage_labels(),
        paints_version: paint.version,
        paints_firmware: paint.firmware,
        paints_bring_up: paint.bring_up,
        paints_fail: paint.fail,
        witness_count: WITNESS_COUNT,
        witness_rank: WITNESS_RANK,
        ver_lo,
        ver_hi,
    })
}

fn line_problems() -> Vec<String> {
    let mut out = Vec::new();
    if !only_forward() {
        out.push("a later failure replaced a further stage".to_string());
    }
    if !stage_labels() {
        out.push(stage_mismatch());
    }
    if !ver_paints_word() {
        out.push(version_mismatch());
    }
    if !id_paints_device() {
        out.push(fail_mismatch(
            &XferFail::Id(0x0e8d, 0x7961, 0xe0),
            b"bt: id 0e8d:7961 e0",
        ));
    }
    if !empty_is_firmware_err() {
        out.push("an empty firmware blob is not bt: firmware err".to_string());
    }
    if !reset_ok_needs_firmware() {
        out.push("bt: reset ok was painted without firmware".to_string());
    }
    if !reset_ok_needs_event() {
        out.push("bt: reset ok was painted without a command-complete event".to_string());
    }
    if !port_is_not_reset_ok() {
        out.push("a port line is bt: reset ok".to_string());
    }
    if !other_lines() {
        out.push(format!(
            "halt is '{}', event is '{}'",
            show(&got_fail(&XferFail::Halt).unwrap_or_default()),
            show(&got_fail(&XferFail::Event).unwrap_or_default())
        ));
    }
    out
}

fn stage_cases() -> Vec<(XferFail, &'static [u8])> {
    vec![
        (XferFail::Port(3, PortStop::Reset), b"bt: port 3 reset"),
        (XferFail::Port(3, PortStop::Slot), b"bt: port 3 slot"),
        (XferFail::Port(3, PortStop::Addr), b"bt: port 3 addr"),
        (XferFail::Port(3, PortStop::Desc), b"bt: port 3 desc"),
        (XferFail::Port(1, PortStop::Desc), b"bt: port 1 desc"),
        (XferFail::Port(12, PortStop::Addr), b"bt: port 12 addr"),
        (XferFail::Port(0, PortStop::Reset), b"bt: port 0"),
    ]
}

fn stage_labels() -> bool {
    stage_cases()
        .into_iter()
        .all(|(fail, expect)| is_fail(&fail, expect))
}

fn stage_mismatch() -> String {
    stage_cases()
        .into_iter()
        .find_map(|(fail, expect)| {
            if is_fail(&fail, expect) {
                None
            } else {
                Some(fail_mismatch(&fail, expect))
            }
        })
        .unwrap_or_else(|| "port line mismatch".to_string())
}

fn ver_paints_word() -> bool {
    is_ver(0x0200, b"bt: ver 0200")
        && is_ver(0x00ff, b"bt: ver 00ff")
        && is_ver(0x0121, b"bt: ver 0121")
}

fn version_mismatch() -> String {
    for (word, expect) in [
        (0x0200u16, &b"bt: ver 0200"[..]),
        (0x00ff, &b"bt: ver 00ff"[..]),
        (0x0121, &b"bt: ver 0121"[..]),
    ] {
        if !is_ver(word, expect) {
            return match got_ver(word) {
                Ok(got) => format!(
                    "formatter wrote '{}', milestone line is '{}'",
                    show(&got),
                    show(expect)
                ),
                Err(msg) => format!("{msg}: {}", show(expect)),
            };
        }
    }
    "version line mismatch".to_string()
}

fn id_paints_device() -> bool {
    is_fail(&XferFail::Id(0x0e8d, 0x7961, 0xe0), b"bt: id 0e8d:7961 e0")
}

fn empty_is_firmware_err() -> bool {
    is_report(&BtReport::FirmwareErr, FIRMWARE_ERR)
        && is_report(&bt_bring_up(true, false, Some(&ACK)), FIRMWARE_ERR)
}

fn reset_ok_needs_firmware() -> bool {
    bt_line(&bt_bring_up(true, false, Some(&ACK))) != RESET_OK
        && bt_line(&bt_bring_up(false, true, Some(&ACK))) != RESET_OK
}

fn reset_ok_needs_event() -> bool {
    is_report(&bt_bring_up(true, true, Some(&ACK)), RESET_OK)
        && bt_line(&bt_bring_up(true, true, None)) != RESET_OK
        && bt_line(&bt_bring_up(true, true, Some(&BAD_EVENT))) != RESET_OK
}

fn port_is_not_reset_ok() -> bool {
    stage_cases()
        .into_iter()
        .chain([
            (XferFail::Halt, b"bt: halt".as_slice()),
            (XferFail::Event, b"bt: event".as_slice()),
            (XferFail::Id(0x0e8d, 0x7961, 0xe0), b"bt: id 0e8d:7961 e0"),
        ])
        .all(|(fail, _)| got_fail(&fail).ok().as_deref() != Some(RESET_OK))
        && is_report(&bt_bring_up(true, true, Some(&ACK)), RESET_OK)
}

fn other_lines() -> bool {
    is_fail(&XferFail::Halt, b"bt: halt") && is_fail(&XferFail::Event, b"bt: event")
}

fn only_forward() -> bool {
    let mut stage = PortStop::Reset;
    stage = stage.later(PortStop::Slot);
    if stage != PortStop::Slot {
        return false;
    }
    stage = stage.later(PortStop::Addr);
    if stage != PortStop::Addr {
        return false;
    }
    stage = stage.later(PortStop::Desc);
    if stage != PortStop::Desc {
        return false;
    }
    if stage.later(PortStop::Reset) != PortStop::Desc
        || stage.later(PortStop::Slot) != PortStop::Desc
        || stage.later(PortStop::Addr) != PortStop::Desc
    {
        return false;
    }
    further(
        XferFail::Port(3, PortStop::Desc),
        XferFail::Port(1, PortStop::Reset),
    ) == XferFail::Port(3, PortStop::Desc)
        && further(
            XferFail::Port(3, PortStop::Reset),
            XferFail::Port(1, PortStop::Addr),
        ) == XferFail::Port(1, PortStop::Addr)
        && further(
            XferFail::Port(3, PortStop::Desc),
            XferFail::Port(3, PortStop::Reset),
        ) == XferFail::Port(3, PortStop::Desc)
        && further(XferFail::Halt, XferFail::Port(3, PortStop::Desc))
            == XferFail::Port(3, PortStop::Desc)
}

fn written(buf_len: usize, fill: &impl Fn(&mut [u8]) -> usize) -> Vec<u8> {
    let mut buf = vec![0u8; buf_len];
    let n = fill(&mut buf);
    buf.truncate(n);
    buf
}

fn panel(fill: impl Fn(&mut [u8]) -> usize) -> Result<Vec<u8>, &'static str> {
    let wide = written(64, &fill);
    let narrow = written(24, &fill);
    if wide.is_empty() || wide != narrow {
        return Err("line does not fit the 24-byte panel buffer");
    }
    Ok(wide)
}

fn got_fail(fail: &XferFail) -> Result<Vec<u8>, &'static str> {
    panel(|buf| fail_line(fail, buf))
}

fn got_ver(word: u16) -> Result<Vec<u8>, &'static str> {
    panel(|buf| version_line(word, buf))
}

fn got_report(report: &BtReport) -> Result<Vec<u8>, &'static str> {
    let line = bt_line(report);
    if line.is_empty() || line.len() > 24 {
        return Err("line does not fit the 24-byte panel buffer");
    }
    Ok(line.to_vec())
}

fn is_fail(fail: &XferFail, expect: &[u8]) -> bool {
    got_fail(fail).ok().as_deref() == Some(expect)
}

fn is_ver(word: u16, expect: &[u8]) -> bool {
    got_ver(word).ok().as_deref() == Some(expect)
}

fn is_report(report: &BtReport, expect: &[u8]) -> bool {
    got_report(report).ok().as_deref() == Some(expect)
}

fn fail_mismatch(fail: &XferFail, expect: &[u8]) -> String {
    match got_fail(fail) {
        Ok(got) => format!(
            "formatter wrote '{}', milestone line is '{}'",
            show(&got),
            show(expect)
        ),
        Err(msg) => format!("{msg}: {}", show(expect)),
    }
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn paint_flags(body: &str) -> PaintFlags {
    let version = body.find("version_line");
    let empty = body.find("FIRMWARE.is_empty()");
    let firmware_err = body.find("BtReport::FirmwareErr");
    let reset = body.find("reset_either");
    let ok = body.find("Ok(");
    let err = body.find("Err(");
    let paints_version = match (version, empty) {
        (Some(version), Some(empty)) => version < empty && body.contains("version_line(version"),
        _ => false,
    };
    let paints_firmware = match (empty, firmware_err, reset) {
        (Some(empty), Some(firmware_err), Some(reset)) => {
            empty < firmware_err && firmware_err < reset && body[empty..reset].contains("bt_line")
        }
        _ => false,
    };
    let paints_bring_up = match (ok, err) {
        (Some(ok), Some(err)) if ok < err => {
            let arm = &body[ok..err];
            arm.contains("bt_bring_up(true, true, Some(")
                && arm.contains("bt_line")
                && !arm.contains("BtReport::ResetOk")
        }
        _ => false,
    };
    let paints_fail = match err {
        Some(err) => {
            let arm = &body[err..];
            arm.contains("fail_line") && !arm.contains("bt_line") && !arm.contains("ResetOk")
        }
        None => false,
    };
    PaintFlags {
        version: paints_version,
        firmware: paints_firmware,
        bring_up: paints_bring_up,
        fail: paints_fail,
    }
}

fn uses_further(body: &str) -> bool {
    body.contains("further(")
}

fn stage_order(body: &str) -> Result<(), String> {
    let mut at = 0;
    for word in [
        "PortStop::Reset",
        "PortStop::Slot",
        "PortStop::Addr",
        "PortStop::Desc",
    ] {
        let Some(found) = body[at..].find(word) else {
            return Err(format!(
                "find_radio does not name {word} after the earlier stages"
            ));
        };
        at += found + word.len();
    }
    Ok(())
}

fn parse_range(body: &str) -> Result<(u16, u16), String> {
    let at = body
        .find("..=")
        .ok_or("controllers_live has no capability range")?;
    let before = &body[..at];
    let lo_at = before
        .rfind("0x")
        .ok_or("capability range has no low word")?;
    let lo_digits: String = before[lo_at + 2..]
        .chars()
        .take_while(|ch| ch.is_ascii_hexdigit())
        .collect();
    if lo_digits.is_empty() {
        return Err("capability low word is not hex".to_string());
    }
    let lo = u16::from_str_radix(&lo_digits, 16)
        .map_err(|_| "capability low word is not hex".to_string())?;
    let after = &body[at + 3..];
    let hi_at = after
        .find("0x")
        .ok_or("capability range has no high word")?;
    let hi_digits: String = after[hi_at + 2..]
        .chars()
        .take_while(|ch| ch.is_ascii_hexdigit())
        .collect();
    if hi_digits.is_empty() {
        return Err("capability high word is not hex".to_string());
    }
    let hi = u16::from_str_radix(&hi_digits, 16)
        .map_err(|_| "capability high word is not hex".to_string())?;
    Ok((lo, hi))
}

fn read_fn(path: &Path, name: &str) -> Result<String, String> {
    let src = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    fn_body(&src, name).ok_or_else(|| format!("{} has no fn {name}", path.display()))
}

fn fn_body(src: &str, name: &str) -> Option<String> {
    let needle = format!("fn {name}(");
    let start = src.find(&needle)?;
    let rest = &src[start..];
    let brace = rest.find('{')?;
    let mut depth = 0usize;
    for (index, ch) in rest[brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(rest[brace..=brace + index].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

enum Explore {
    Ok {
        states: usize,
        witness_trace: String,
        reset_trace: String,
    },
    Line {
        trace: String,
        msg: String,
    },
    Model {
        msg: String,
    },
}

fn explore() -> Explore {
    let mut seen = HashSet::new();
    let mut prev: HashMap<World, (&'static str, World)> = HashMap::new();
    let mut queue = VecDeque::new();
    for world in initials() {
        if seen.insert(world) {
            queue.push_back(world);
        }
    }
    let mut done = Vec::new();
    let mut stuck = Vec::new();
    while let Some(world) = queue.pop_front() {
        if seen.len() > 10_000 {
            return Explore::Model {
                msg: "more than 10000 states".to_string(),
            };
        }
        let next = successors(&world);
        if next.is_empty() {
            if world.phase == Phase::Done {
                done.push(world);
            } else {
                stuck.push(world);
            }
            continue;
        }
        for (name, next_world) in next {
            if seen.insert(next_world) {
                prev.insert(next_world, (name, world));
                queue.push_back(next_world);
            }
        }
    }
    if let Some(world) = stuck.first().copied() {
        return Explore::Model {
            msg: format!("stuck after {}", trace_of(&prev, world)),
        };
    }
    let mut witness_trace = None;
    let mut reset_trace = None;
    for world in done {
        if let Err(msg) = check_done(&world) {
            return Explore::Line {
                trace: trace_of(&prev, world),
                msg,
            };
        }
        if is_witness(&world) && witness_trace.is_none() {
            witness_trace = Some(trace_of(&prev, world));
        }
        if is_reset(&world) && reset_trace.is_none() {
            reset_trace = Some(trace_of(&prev, world));
        }
    }
    match (witness_trace, reset_trace) {
        (Some(witness_trace), Some(reset_trace)) => Explore::Ok {
            states: seen.len(),
            witness_trace,
            reset_trace,
        },
        (None, _) => Explore::Model {
            msg: "model did not reach bt: port 3 desc".to_string(),
        },
        (_, None) => Explore::Model {
            msg: "model did not reach bt: reset ok".to_string(),
        },
    }
}

fn initials() -> Vec<World> {
    let mut out = Vec::new();
    for firmware in [false, true] {
        out.push(World::boot(true, false, firmware));
        out.push(World::boot(false, false, firmware));
        out.push(World::boot(false, true, firmware));
    }
    out
}

fn is_witness(world: &World) -> bool {
    world.phase == Phase::Done
        && world.kind == Kind::Port
        && world.connected == WITNESS_COUNT
        && world.rank == WITNESS_RANK
        && world.peak == WITNESS_RANK
        && !world.event_ack
        && world.firmware
        && world.live
}

fn is_reset(world: &World) -> bool {
    world.phase == Phase::Done && world.kind == Kind::Ok && world.event_ack && world.firmware
}

fn check_done(world: &World) -> Result<(), String> {
    match world.kind {
        Kind::Port => {
            let actual = actual_port(world.connected, world.rank)?;
            let expected = expect_port(world.connected, world.peak);
            if actual != expected {
                return Err(format!(
                    "painted '{}' for count {} rank {} peak {}, milestone line is '{}'",
                    show(&actual),
                    world.connected,
                    world.rank,
                    world.peak,
                    show(&expected)
                ));
            }
            if actual.as_slice() == RESET_OK {
                return Err("a port line is bt: reset ok".to_string());
            }
            Ok(())
        }
        Kind::Ok => {
            if !world.firmware || !world.live || !world.event_ack {
                return Err(
                    "bt: reset ok without firmware and a command-complete event".to_string()
                );
            }
            let line = got_report(&bt_bring_up(true, true, Some(&ACK)))?;
            if line.as_slice() != RESET_OK {
                return Err(format!("command-complete painted '{}'", show(&line)));
            }
            Ok(())
        }
        Kind::Fw => {
            if world.firmware || !world.live {
                return Err("firmware err was painted for a present blob".to_string());
            }
            let line = got_report(&BtReport::FirmwareErr)?;
            if line.as_slice() != FIRMWARE_ERR {
                return Err(format!("empty blob painted '{}'", show(&line)));
            }
            Ok(())
        }
        Kind::Ver => {
            if world.live || !world.foreign {
                return Err("bt: ver was painted for an in-range controller".to_string());
            }
            let line = got_ver(0x0200)?;
            if line.as_slice() != b"bt: ver 0200" {
                return Err(format!("capability word painted '{}'", show(&line)));
            }
            Ok(())
        }
        Kind::Id => {
            let line = got_fail(&XferFail::Id(0x0e8d, 0x7961, 0xe0))?;
            if line.as_slice() != b"bt: id 0e8d:7961 e0" {
                return Err(format!("addressed device painted '{}'", show(&line)));
            }
            Ok(())
        }
        Kind::Event => {
            let line = got_fail(&XferFail::Event)?;
            if line.as_slice() != b"bt: event" || line.as_slice() == RESET_OK {
                return Err(format!("unfinished reset painted '{}'", show(&line)));
            }
            Ok(())
        }
        Kind::Halt => {
            let line = got_fail(&XferFail::Halt)?;
            if line.as_slice() != b"bt: halt" {
                return Err(format!("halt painted '{}'", show(&line)));
            }
            Ok(())
        }
        Kind::Absent => {
            if world.live || world.foreign {
                return Err("absent was painted for a live or foreign controller".to_string());
            }
            let line = got_report(&BtReport::Absent)?;
            if line.as_slice() == RESET_OK || line.is_empty() {
                return Err(format!("absent painted '{}'", show(&line)));
            }
            Ok(())
        }
        Kind::None => Err("a finished boot painted no line".to_string()),
    }
}

fn expect_port(count: u8, peak: u8) -> Vec<u8> {
    match stage_word(peak) {
        Some(word) => format!("bt: port {count} {word}").into_bytes(),
        None => format!("bt: port {count}").into_bytes(),
    }
}

fn actual_port(count: u8, rank: u8) -> Result<Vec<u8>, String> {
    let stop = if rank == 0 {
        PortStop::Reset
    } else {
        stop_of(rank).ok_or_else(|| format!("rank {rank} is not a stage"))?
    };
    got_fail(&XferFail::Port(count, stop)).map_err(str::to_string)
}

fn stage_word(rank: u8) -> Option<&'static str> {
    match rank {
        1 => Some("reset"),
        2 => Some("slot"),
        3 => Some("addr"),
        4 => Some("desc"),
        _ => None,
    }
}

fn stop_of(rank: u8) -> Option<PortStop> {
    match rank {
        1 => Some(PortStop::Reset),
        2 => Some(PortStop::Slot),
        3 => Some(PortStop::Addr),
        4 => Some(PortStop::Desc),
        _ => None,
    }
}

fn rank_of(stop: PortStop) -> u8 {
    match stop {
        PortStop::Reset => 1,
        PortStop::Slot => 2,
        PortStop::Addr => 3,
        PortStop::Desc => 4,
    }
}

fn trace_of(prev: &HashMap<World, (&'static str, World)>, end: World) -> String {
    let mut steps = Vec::new();
    let mut cur = end;
    while let Some((name, parent)) = prev.get(&cur) {
        steps.push(*name);
        cur = *parent;
    }
    steps.reverse();
    steps.join(" ")
}

fn successors(world: &World) -> Vec<(&'static str, World)> {
    let mut out = Vec::new();
    push(&mut out, "ClassifyDead", classify_dead(world));
    push(&mut out, "ClassifyEmpty", classify_empty(world));
    push(&mut out, "OpenPorts", open_ports(world));
    push(&mut out, "SeeReset", see_stage(world, 1));
    push(&mut out, "SeeSlot", see_stage(world, 2));
    push(&mut out, "SeeAddr", see_stage(world, 3));
    push(&mut out, "SeeDesc", see_stage(world, 4));
    push(&mut out, "SeeId", see_id(world));
    push(&mut out, "SeeEvent", see_event(world));
    push(&mut out, "AckEvent", ack_event(world));
    push(&mut out, "PaintOk", paint_ok(world));
    push(&mut out, "PaintFail", paint_fail(world));
    push(&mut out, "PaintHalt", paint_halt(world));
    push(&mut out, "PaintUnconnected", paint_unconnected(world));
    out
}

fn push(out: &mut Vec<(&'static str, World)>, name: &'static str, next: Option<World>) {
    if let Some(next) = next {
        out.push((name, next));
    }
}

impl World {
    fn boot(live: bool, foreign: bool, firmware: bool) -> Self {
        Self {
            phase: Phase::Boot,
            firmware,
            live,
            foreign,
            connected: 0,
            rank: 0,
            peak: 0,
            kind: Kind::None,
            event_ack: false,
        }
    }
}

fn classify_dead(world: &World) -> Option<World> {
    if world.phase != Phase::Boot || world.live {
        return None;
    }
    let mut next = *world;
    next.phase = Phase::Done;
    next.kind = if world.foreign {
        Kind::Ver
    } else {
        Kind::Absent
    };
    Some(next)
}

fn classify_empty(world: &World) -> Option<World> {
    if world.phase != Phase::Boot || !world.live || world.firmware {
        return None;
    }
    let mut next = *world;
    next.phase = Phase::Done;
    next.kind = Kind::Fw;
    Some(next)
}

fn open_ports(world: &World) -> Option<World> {
    if world.phase != Phase::Boot || !world.live || !world.firmware {
        return None;
    }
    let mut next = *world;
    next.phase = Phase::Ports;
    Some(next)
}

fn see_stage(world: &World, rank: u8) -> Option<World> {
    if world.phase != Phase::Ports || world.connected >= WITNESS_COUNT {
        return None;
    }
    if world.kind != Kind::None && world.kind != Kind::Port {
        return None;
    }
    let incoming = stop_of(rank)?;
    let kept = match stop_of(world.peak) {
        Some(prev) => prev.later(incoming),
        None => incoming,
    };
    let mut next = *world;
    next.connected += 1;
    next.peak = world.peak.max(rank);
    next.rank = rank_of(kept);
    next.kind = Kind::Port;
    Some(next)
}

fn see_id(world: &World) -> Option<World> {
    if world.phase != Phase::Ports || (world.kind != Kind::None && world.kind != Kind::Port) {
        return None;
    }
    let mut next = *world;
    next.kind = Kind::Id;
    Some(next)
}

fn see_event(world: &World) -> Option<World> {
    if world.phase != Phase::Ports {
        return None;
    }
    if world.kind != Kind::None && world.kind != Kind::Port && world.kind != Kind::Id {
        return None;
    }
    let mut next = *world;
    next.kind = Kind::Event;
    Some(next)
}

fn ack_event(world: &World) -> Option<World> {
    if world.phase != Phase::Ports || !world.firmware || world.event_ack {
        return None;
    }
    let mut next = *world;
    next.event_ack = true;
    Some(next)
}

fn paint_ok(world: &World) -> Option<World> {
    if world.phase != Phase::Ports || !world.event_ack || !world.firmware || !world.live {
        return None;
    }
    let mut next = *world;
    next.phase = Phase::Done;
    next.kind = Kind::Ok;
    Some(next)
}

fn paint_fail(world: &World) -> Option<World> {
    if world.phase != Phase::Ports || world.event_ack {
        return None;
    }
    if world.kind != Kind::Port && world.kind != Kind::Id && world.kind != Kind::Event {
        return None;
    }
    let mut next = *world;
    next.phase = Phase::Done;
    Some(next)
}

fn paint_halt(world: &World) -> Option<World> {
    if world.phase != Phase::Ports || world.event_ack || world.kind != Kind::None {
        return None;
    }
    let mut next = *world;
    next.phase = Phase::Done;
    next.kind = Kind::Halt;
    Some(next)
}

fn paint_unconnected(world: &World) -> Option<World> {
    if world.phase != Phase::Ports
        || world.event_ack
        || world.kind != Kind::None
        || world.connected != 0
    {
        return None;
    }
    let mut next = *world;
    next.phase = Phase::Done;
    next.kind = Kind::Port;
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protection_domain_source_parses() {
        assert!(read_facts(&repo_root()).is_ok());
    }

    #[test]
    fn stages_are_named_reset_then_slot_then_addr_then_desc() {
        let body = "PortStop::Reset PortStop::Slot PortStop::Addr PortStop::Desc";
        assert!(stage_order(body).is_ok());
        let backwards = "PortStop::Desc PortStop::Addr PortStop::Slot PortStop::Reset";
        assert!(stage_order(backwards).is_err());
    }

    #[test]
    fn the_capability_range_is_read_from_the_comparison() {
        let body = "if (0x0100..=0x0120).contains(&version) {";
        assert_eq!(parse_range(body).unwrap(), (0x0100, 0x0120));
        let wider = "if (0x0100..=0x0130).contains(&version)";
        assert_eq!(parse_range(wider).unwrap(), (0x0100, 0x0130));
    }

    #[test]
    fn the_success_arm_has_to_call_bring_up() {
        let good = "\
fn bring_up_text() {
    if !live {
        return version_line(version, text);
    }
    if FIRMWARE.is_empty() {
        return bt_line(&BtReport::FirmwareErr);
    }
    reset_either();
    Ok(n) => bt_line(&bt_bring_up(true, true, Some(&event[..n]))),
    Err(fail) => fail_line(&fail, text),
}";
        let flags = paint_flags(good);
        assert!(flags.version && flags.firmware && flags.bring_up && flags.fail);
        let early = good.replace(
            "bt_line(&bt_bring_up(true, true, Some(&event[..n])))",
            "bt_line(&BtReport::ResetOk)",
        );
        assert!(!paint_flags(&early).bring_up);
        assert!(uses_further("fail = further(fail, next);"));
        assert!(!uses_further("fail = next;"));
    }
}
