//! Phase 60 Track D: host-side QoS / channel priority abuse checks.
//!
//! Validates Microkit PPC priority rules and workstation service-class bands
//! against the composed SDF (template priorities + profile channels). MCS
//! budgets remain out of scope (ADR-006).

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use anyhow::{bail, Context, Result};
use regex::Regex;

use crate::profile::{effective_trust_class, Profile, Profiles, TrustClass};

/// One protection domain priority from the SDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdPriority {
    pub name: String,
    pub priority: u32,
}

/// PPC edge: caller has `pp="true"` and must have strictly lower priority than callee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PpcEdge {
    pub caller: String,
    pub callee: String,
}

/// Result of validating one composed system description.
#[derive(Debug, Clone, Default)]
pub struct QosReport {
    pub pds: Vec<PdPriority>,
    pub ppc_edges: Vec<PpcEdge>,
    pub errors: Vec<String>,
    pub notes: Vec<String>,
}

impl QosReport {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Extract `protection_domain name="…" priority="…"` entries.
pub fn parse_pd_priorities(sdf: &str) -> Result<Vec<PdPriority>> {
    let re = Regex::new(r#"<protection_domain\s+[^>]*name="([^"]+)"[^>]*priority="(\d+)"[^>]*>"#)
        .context("compile pd priority regex")?;
    // Attributes may appear in either order.
    let re_alt =
        Regex::new(r#"<protection_domain\s+[^>]*priority="(\d+)"[^>]*name="([^"]+)"[^>]*>"#)
            .context("compile pd priority regex alt")?;

    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for cap in re.captures_iter(sdf) {
        let name = cap[1].to_string();
        if seen.insert(name.clone()) {
            out.push(PdPriority {
                name,
                priority: cap[2].parse().context("parse priority")?,
            });
        }
    }
    for cap in re_alt.captures_iter(sdf) {
        let name = cap[2].to_string();
        if seen.insert(name.clone()) {
            out.push(PdPriority {
                name,
                priority: cap[1].parse().context("parse priority")?,
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Extract PPC edges from `<channel>` blocks (caller = end with `pp="true"`).
pub fn parse_ppc_edges(sdf: &str) -> Result<Vec<PpcEdge>> {
    let channel_re = Regex::new(r"(?s)<channel>(.*?)</channel>").context("compile channel re")?;
    // Match any self-closing end tag; detect pp from the full tag text.
    let end_re = Regex::new(r#"<end\s+([^>]*?)/\s*>"#).context("compile end re")?;
    let pd_re = Regex::new(r#"pd="([^"]+)""#).context("compile pd re")?;

    let mut edges = Vec::new();
    for ch in channel_re.captures_iter(sdf) {
        let body = &ch[1];
        let mut ends: Vec<(String, bool)> = Vec::new();
        for end in end_re.captures_iter(body) {
            let tag = &end[1];
            let Some(pd_cap) = pd_re.captures(tag) else {
                continue;
            };
            let pd = pd_cap[1].to_string();
            let pp = tag.contains("pp=\"true\"");
            ends.push((pd, pp));
        }

        if ends.len() != 2 {
            continue;
        }
        let callers: Vec<_> = ends.iter().filter(|(_, pp)| *pp).map(|(p, _)| p).collect();
        let callees: Vec<_> = ends.iter().filter(|(_, pp)| !*pp).map(|(p, _)| p).collect();
        if callers.len() == 1 && callees.len() == 1 {
            edges.push(PpcEdge {
                caller: callers[0].clone(),
                callee: callees[0].clone(),
            });
        }
    }
    edges.sort_by(|a, b| (&a.caller, &a.callee).cmp(&(&b.caller, &b.callee)));
    edges.dedup();
    Ok(edges)
}

/// Validate PPC priorities and optional workstation service-class bands.
pub fn validate_qos_sdf(sdf: &str, enforce_workstation_bands: bool) -> Result<QosReport> {
    let mut report = QosReport {
        pds: parse_pd_priorities(sdf)?,
        ppc_edges: parse_ppc_edges(sdf)?,
        ..Default::default()
    };

    let prio: BTreeMap<String, u32> = report
        .pds
        .iter()
        .map(|p| (p.name.clone(), p.priority))
        .collect();

    if prio.is_empty() {
        report
            .notes
            .push("no protection_domain priorities found in SDF".into());
        return Ok(report);
    }

    for edge in &report.ppc_edges {
        let Some(&caller_p) = prio.get(&edge.caller) else {
            report.errors.push(format!(
                "PPC caller {} not found among protection domains",
                edge.caller
            ));
            continue;
        };
        let Some(&callee_p) = prio.get(&edge.callee) else {
            report.errors.push(format!(
                "PPC callee {} not found among protection domains",
                edge.callee
            ));
            continue;
        };
        if caller_p >= callee_p {
            report.errors.push(format!(
                "PPC priority abuse: {} (prio {caller_p}) must be < {} (prio {callee_p})",
                edge.caller, edge.callee
            ));
        }
    }

    if enforce_workstation_bands {
        check_workstation_bands(&prio, &mut report);
    }

    Ok(report)
}

/// ADR-006 / docs/qos.md service-class floors for workstation-shaped systems.
fn check_workstation_bands(prio: &BTreeMap<String, u32>, report: &mut QosReport) {
    let require_min = |report: &mut QosReport, name: &str, min: u32| {
        if let Some(&p) = prio.get(name)
            && p < min
        {
            report.errors.push(format!(
                "service-class band: {name} priority {p} < minimum {min} (docs/qos.md)"
            ));
        }
    };
    let require_exact = |report: &mut QosReport, name: &str, exact: u32| {
        if let Some(&p) = prio.get(name)
            && p != exact
        {
            report.errors.push(format!(
                "service-class band: {name} priority {p} expected {exact} (docs/qos.md)"
            ));
        }
    };

    // Platform (drivers): typically 6–10
    for name in [
        "serial_driver",
        "serial_virt",
        "virtio_blk_driver",
        "virtio_net_driver",
        "virtio_pci_driver",
        "genet_driver",
        "emmc2_driver",
        "pl031_driver",
        "sp804_driver",
        "goldfish_rtc_driver",
        "rdtime_timer_driver",
        "cmos_rtc_driver",
        "tsc_timer_driver",
    ] {
        require_min(report, name, 6);
    }

    // Services
    require_min(report, "log_server", 5);
    require_min(report, "fs_server", 4);
    require_min(report, "net_server", 4);

    // Control / bulk / interactive
    require_min(report, "config_server", 3);
    require_min(report, "supervisor", 2);
    for name in ["edit", "chat_client", "http_file_browser", "backup"] {
        require_min(report, name, 2);
    }
    // Shell must stay lowest among PPC clients (priority 1 on workstation).
    require_exact(report, "shell", 1);

    if let Some(&shell_p) = prio.get("shell") {
        report.notes.push(format!(
            "shell priority={shell_p}; {} PPC edges checked",
            report.ppc_edges.len()
        ));
    }
}

fn should_enforce_bands(profile: &Profile) -> bool {
    matches!(
        effective_trust_class(profile),
        TrustClass::Admin | TrustClass::AdminCore
    )
}

/// Run QoS checks for one profile (renders composed SDF).
pub fn check_profile_qos(root: &Path, name: &str, profile: &Profile) -> Result<QosReport> {
    let sdf = crate::system::render_profile_system(root, name, profile, None)
        .with_context(|| format!("render SDF for profile {name}"))?;
    let enforce = should_enforce_bands(profile);
    let report = validate_qos_sdf(&sdf, enforce)?;
    Ok(report)
}

/// Print and fail if any profile fails QoS checks.
pub fn check_profiles_qos(root: &Path, profiles: &Profiles, name: Option<&str>) -> Result<()> {
    match name {
        Some(n) => {
            let p = crate::profile::get_profile(profiles, n)?;
            let report = check_profile_qos(root, n, p)?;
            print_report(n, &report);
            if !report.ok() {
                bail!(
                    "profile {n}: QoS check failed ({} errors)",
                    report.errors.len()
                );
            }
            println!("profile {n}: qos ok");
        }
        None => {
            let mut failed = 0usize;
            let mut checked = 0usize;
            for (n, p) in profiles {
                // Skip pure bases with no default board (can't render alone).
                if p.default_board.is_none() {
                    continue;
                }
                checked += 1;
                match check_profile_qos(root, n, p) {
                    Ok(report) => {
                        print_report(n, &report);
                        if report.ok() {
                            println!("profile {n}: qos ok");
                        } else {
                            eprintln!("profile {n}: qos FAILED ({} errors)", report.errors.len());
                            failed += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!("profile {n}: qos check error: {e:#}");
                        failed += 1;
                    }
                }
            }
            if checked == 0 {
                bail!("no profiles with default_board to check");
            }
            if failed > 0 {
                bail!("{failed}/{checked} profile(s) failed QoS checks");
            }
            println!("==> qos: {checked} profile(s) ok");
        }
    }
    Ok(())
}

fn print_report(name: &str, report: &QosReport) {
    if !report.notes.is_empty() {
        for n in &report.notes {
            println!("  note [{name}]: {n}");
        }
    }
    for e in &report.errors {
        eprintln!("  error [{name}]: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_SDF: &str = r#"
    <protection_domain name="serial_driver" priority="10" stack_size="0x10_000">
    </protection_domain>
    <protection_domain name="fs_server" priority="4" stack_size="0x10_000">
    </protection_domain>
    <protection_domain name="shell" priority="1" stack_size="0x10_000">
    </protection_domain>
    <channel>
        <end pd="fs_server" id="3" />
        <end pd="shell" id="1" pp="true" />
    </channel>
    <channel>
        <end pd="serial_driver" id="1" />
        <end pd="shell" id="0" pp="true" />
    </channel>
"#;

    const BAD_PPC_SDF: &str = r#"
    <protection_domain name="server" priority="1" stack_size="0x10_000">
    </protection_domain>
    <protection_domain name="client" priority="5" stack_size="0x10_000">
    </protection_domain>
    <channel>
        <end pd="server" id="1" />
        <end pd="client" id="0" pp="true" />
    </channel>
"#;

    #[test]
    fn good_workstation_fragment_passes() {
        let report = validate_qos_sdf(GOOD_SDF, true).unwrap();
        assert!(report.ok(), "{:?}", report.errors);
        assert_eq!(report.ppc_edges.len(), 2);
    }

    #[test]
    fn inverted_ppc_fails() {
        let report = validate_qos_sdf(BAD_PPC_SDF, false).unwrap();
        assert!(!report.ok());
        assert!(report
            .errors
            .iter()
            .any(|e| e.contains("PPC priority abuse")));
    }

    #[test]
    fn shell_band_enforced() {
        let sdf = r#"
        <protection_domain name="shell" priority="5" stack_size="0x10_000">
        </protection_domain>
        <protection_domain name="fs_server" priority="4" stack_size="0x10_000">
        </protection_domain>
        "#;
        let report = validate_qos_sdf(sdf, true).unwrap();
        assert!(!report.ok());
        assert!(report.errors.iter().any(|e| e.contains("shell")));
    }
}
