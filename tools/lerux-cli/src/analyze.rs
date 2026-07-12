//! Phase 57: post-process serial captures for faults, hangs, and service errors.

use std::{
    fs,
    io::{self, Read},
};

use anyhow::{bail, Context, Result};

/// Analyze a serial log file (or stdin if `path` is `-`) and print a diagnose summary.
pub fn analyze_log(path: &str) -> Result<()> {
    let text = if path == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).context("read stdin")?;
        buf
    } else {
        fs::read_to_string(path).with_context(|| format!("read {path}"))?
    };
    let report = analyze_text(&text);
    print_report(&report);
    if report.faults > 0 || report.watchdog_fail || report.errors > 0 {
        bail!("diagnose found problems (see summary above)");
    }
    Ok(())
}

#[derive(Debug, Default)]
struct Report {
    lines: usize,
    faults: usize,
    watchdog_ok: bool,
    watchdog_fail: bool,
    init_ok: bool,
    ready: bool,
    errors: usize,
    warns: usize,
    service_errors: Vec<String>,
    fault_lines: Vec<String>,
    last_interesting: Vec<String>,
}

fn analyze_text(text: &str) -> Report {
    let mut r = Report {
        lines: text.lines().count(),
        ..Default::default()
    };
    for line in text.lines() {
        let l = line.trim();
        if l.contains("lerux-debug: fault")
            || l.contains("VmFault")
            || l.contains("CapFault")
            || l.contains("crash dump")
        {
            r.faults += 1;
            push_cap(&mut r.fault_lines, l, 12);
        }
        if l.contains("lerux-supervisor: watchdog ok") {
            r.watchdog_ok = true;
        }
        if l.contains("lerux-supervisor: watchdog fail") {
            r.watchdog_fail = true;
        }
        if l.contains("lerux-supervisor: init ok") {
            r.init_ok = true;
        }
        if l.contains("lerux-supervisor: ready") || l.contains("lerux-shell: ready") {
            r.ready = true;
        }
        if l.contains(" ERROR ")
            || l.contains("log::error")
            || l.contains(": error")
            || l.starts_with("E[")
        {
            // Heuristic: structured E[tag] or traditional ERROR
            if l.contains("ERROR") || l.starts_with("E[") || l.contains(" probe error") {
                r.errors += 1;
                push_cap(&mut r.service_errors, l, 16);
            }
        }
        if l.contains(" WARN ") || l.starts_with("W[") || l.contains(" degraded") {
            r.warns += 1;
        }
        if l.contains("lerux-") || l.contains("MON|") || l.contains("fault") {
            push_cap(&mut r.last_interesting, l, 20);
        }
    }
    r
}

fn push_cap(v: &mut Vec<String>, line: &str, cap: usize) {
    if v.len() < cap {
        v.push(line.to_string());
    }
}

fn print_report(r: &Report) {
    println!("==> lerux diagnose (Phase 57)");
    println!("lines:           {}", r.lines);
    println!("init ok:         {}", r.init_ok);
    println!("ready:           {}", r.ready);
    println!("watchdog ok:     {}", r.watchdog_ok);
    println!("watchdog fail:   {}", r.watchdog_fail);
    println!("fault markers:   {}", r.faults);
    println!("error markers:   {}", r.errors);
    println!("warn markers:    {}", r.warns);
    if !r.fault_lines.is_empty() {
        println!("\n-- faults --");
        for l in &r.fault_lines {
            println!("{l}");
        }
    }
    if !r.service_errors.is_empty() {
        println!("\n-- errors --");
        for l in &r.service_errors {
            println!("{l}");
        }
    }
    if !r.last_interesting.is_empty() {
        println!("\n-- recent interesting --");
        for l in r
            .last_interesting
            .iter()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            println!("{l}");
        }
    }
    if r.faults == 0 && !r.watchdog_fail && r.errors == 0 {
        println!("\n==> no crash/hang markers found");
    }
}
