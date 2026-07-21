use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::channels::{ensure_channels_valid, to_sdf_pd_name, ChannelSpec, ChannelValidation};

/// Phase 60 capability tier for a system profile (operator-facing surface).
///
/// Declared in profile TOML as `trust_class = "admin"` etc. When omitted, host
/// tooling infers a class from the PD set (`infer_trust_class`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustClass {
    /// Full shell + bulk apps (edit/chat/http-fs/backup). Highest interactive risk.
    Admin,
    /// Shell + services, no bulk app PDs (`dev-workstation`).
    AdminCore,
    /// Fixed-function appliance (HTTP/echo), no interactive admin shell.
    Appliance,
    /// Serial hello / bring-up only.
    Minimal,
    /// Isolation / debug layouts (fault parent, crash child).
    Debug,
}

impl TrustClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::AdminCore => "admin-core",
            Self::Appliance => "appliance",
            Self::Minimal => "minimal",
            Self::Debug => "debug",
        }
    }
}

impl core::fmt::Display for TrustClass {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Profile {
    pub template: String,
    pub pds: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_board: Option<String>,
    /// Phase 60: declared risk tier (`admin`, `admin-core`, `appliance`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust_class: Option<TrustClass>,
    /// Structured channel manifest (`[[channel]]` tables). Source of truth for
    /// IPC topology: `render_system` splices these into the board template when
    /// `default_board` matches (Phase 41).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channel: Vec<ChannelSpec>,
}

/// On-disk profile TOML before `extends` composition.
#[derive(Debug, Clone, Deserialize)]
struct ProfileFile {
    #[serde(default)]
    extends: Option<String>,
    #[serde(flatten)]
    body: ProfileBody,
}

/// Fields that may be inherited from a base profile when `extends` is set.
#[derive(Debug, Clone, Default, Deserialize)]
struct ProfileBody {
    #[serde(default)]
    template: Option<String>,
    #[serde(default)]
    pds: Vec<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    default_board: Option<String>,
    #[serde(default)]
    trust_class: Option<TrustClass>,
    #[serde(default)]
    channel: Vec<ChannelSpec>,
}

fn compose_profile(name: &str, file: ProfileFile, bases: &Profiles) -> Result<Profile> {
    let base = if let Some(ref base_name) = file.extends {
        Some(
            bases
                .get(base_name)
                .with_context(|| format!("profile {name} extends unknown profile {base_name:?}"))?,
        )
    } else {
        None
    };

    let template = file
        .body
        .template
        .or_else(|| base.map(|b| b.template.clone()))
        .with_context(|| format!("profile {name} missing template"))?;

    let mut pds = base.map(|b| b.pds.clone()).unwrap_or_default();
    for pd in file.body.pds {
        if !pds.contains(&pd) {
            pds.push(pd);
        }
    }
    anyhow::ensure!(
        !pds.is_empty(),
        "profile {name} has no pds (set pds or extends)"
    );

    let description = file
        .body
        .description
        .or_else(|| base.and_then(|b| b.description.clone()));
    let default_board = file
        .body
        .default_board
        .or_else(|| base.and_then(|b| b.default_board.clone()));
    let trust_class = file
        .body
        .trust_class
        .or_else(|| base.and_then(|b| b.trust_class));

    let base_channels = base.map(|b| b.channel.as_slice()).unwrap_or(&[]);
    let channel = merge_channels(base_channels, &file.body.channel);

    Ok(Profile {
        template,
        pds,
        description,
        default_board,
        trust_class,
        channel,
    })
}

/// Overlay channels win on name; base channels not overridden keep base order.
fn merge_channels(base: &[ChannelSpec], overlay: &[ChannelSpec]) -> Vec<ChannelSpec> {
    let overlay_names: std::collections::BTreeSet<_> = overlay.iter().map(|c| &c.name).collect();
    let mut out = overlay.to_vec();
    for ch in base {
        if !overlay_names.contains(&ch.name) {
            out.push(ch.clone());
        }
    }
    out
}

pub type Profiles = BTreeMap<String, Profile>;

pub fn load_profiles(root: &Path) -> Result<Profiles> {
    let dir = root.join("support/profiles");
    if !dir.exists() {
        return Ok(BTreeMap::new());
    }

    let mut raw: BTreeMap<String, ProfileFile> = BTreeMap::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("read dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let contents =
            fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let file: ProfileFile =
            toml::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
        raw.insert(name, file);
    }

    // Compose `extends` in dependency order (bases before overlays).
    let mut profiles = BTreeMap::new();
    let mut pending: Vec<String> = raw.keys().cloned().collect();
    while !pending.is_empty() {
        let before = pending.len();
        let mut still_pending = Vec::new();
        for name in pending {
            let file = raw
                .get(&name)
                .with_context(|| format!("profile {name} missing from raw map"))?;
            if let Some(ref base_name) = file.extends
                && !profiles.contains_key(base_name)
            {
                still_pending.push(name);
                continue;
            }
            let profile = compose_profile(&name, file.clone(), &profiles)
                .with_context(|| format!("compose profile {name}"))?;
            ensure_channels_valid(&profile.channel, &profile.pds, &format!("profile {name}"))
                .with_context(|| format!("validate channels in profile {name}"))?;
            profiles.insert(name, profile);
        }
        pending = still_pending;
        if pending.len() == before {
            anyhow::bail!(
                "unresolved profile extends (cycle or missing base?): {}",
                pending.join(", ")
            );
        }
    }
    Ok(profiles)
}

pub fn get_profile<'a>(profiles: &'a Profiles, name: &str) -> Result<&'a Profile> {
    profiles
        .get(name)
        .with_context(|| format!("unknown profile {name:?} (run `lerux profile list`)"))
}

pub fn list_profiles(profiles: &Profiles) {
    if profiles.is_empty() {
        println!("(no profiles found in support/profiles/)");
        return;
    }
    for (name, p) in profiles {
        let desc = p.description.as_deref().unwrap_or("");
        let n = p.channel.len();
        let tier = effective_trust_class(p);
        println!("{name:20} [{n:2} ch] [{tier:10}] {desc}");
    }
}

pub fn show_profile(name: &str, profile: &Profile) {
    println!("profile: {name}");
    println!("  template: {}", profile.template);
    if let Some(board) = &profile.default_board {
        println!("  default_board: {board}");
    }
    if let Some(desc) = &profile.description {
        println!("  description: {desc}");
    }
    let tier = effective_trust_class(profile);
    let src = if profile.trust_class.is_some() {
        "declared"
    } else {
        "inferred"
    };
    println!("  trust_class: {tier} ({src})");
    println!("  pds: {}", profile.pds.join(", "));
    println!("  channels ({}):", profile.channel.len());
    for ch in &profile.channel {
        println!("    - {ch}");
    }
}

/// Infer a trust class from the PD set when the profile omits `trust_class`.
pub fn infer_trust_class(pds: &[String]) -> TrustClass {
    let has = |name: &str| pds.iter().any(|p| p == name);
    if has("debug-handler") || has("crash-demo") {
        return TrustClass::Debug;
    }
    let bulk = ["edit", "chat-client", "http-file-browser", "backup"];
    let has_bulk = bulk.iter().any(|p| has(p));
    if has("shell") && has_bulk {
        return TrustClass::Admin;
    }
    if has("shell") {
        return TrustClass::AdminCore;
    }
    if has("http-server") || has("echo-server") || has("echo-client") {
        return TrustClass::Appliance;
    }
    TrustClass::Minimal
}

pub fn effective_trust_class(profile: &Profile) -> TrustClass {
    profile
        .trust_class
        .unwrap_or_else(|| infer_trust_class(&profile.pds))
}

/// Trust domain of a single PD (matches docs/security.md map).
pub fn pd_trust_domain(pd: &str) -> &'static str {
    match pd {
        "serial-driver"
        | "virtio-blk-driver"
        | "virtio-net-driver"
        | "virtio-pci-driver"
        | "genet-driver"
        | "emmc2-driver"
        | "pl031-driver"
        | "sp804-driver"
        | "goldfish-rtc-driver"
        | "rdtime-timer-driver"
        | "cmos-rtc-driver"
        | "tsc-timer-driver" => "platform",
        "fs-server" | "net-server" | "serial-virt" | "config-server" | "log-server"
        | "blk-server" => "service",
        "supervisor" => "control",
        "debug-handler" => "debug",
        "shell" | "edit" | "chat-client" | "http-file-browser" | "backup" | "fetch-client"
        | "crash-demo" | "hello" | "echo-client" | "echo-server" | "http-server" | "fs-client"
        | "net-client" | "blk-client" => "untrusted",
        _ => "unknown",
    }
}

fn profile_has_channel_between(profile: &Profile, a: &str, b: &str) -> bool {
    let a = to_sdf_pd_name(a);
    let b = to_sdf_pd_name(b);
    profile.channel.iter().any(|ch| {
        let pds: Vec<_> = ch.ends.iter().map(|e| to_sdf_pd_name(&e.pd)).collect();
        pds.contains(&a) && pds.contains(&b)
    })
}

/// Print a Phase 60 capability audit for one profile (or all if `name` is None).
pub fn audit_profiles(profiles: &Profiles, name: Option<&str>) -> Result<()> {
    match name {
        Some(n) => {
            let p = get_profile(profiles, n)?;
            audit_one(n, p);
        }
        None => {
            for (n, p) in profiles {
                audit_one(n, p);
                println!();
            }
        }
    }
    Ok(())
}

fn audit_one(name: &str, profile: &Profile) {
    let tier = effective_trust_class(profile);
    let src = if profile.trust_class.is_some() {
        "declared"
    } else {
        "inferred"
    };
    println!("==> profile {name}");
    println!("    trust_class: {tier} ({src})");
    if let Some(desc) = &profile.description {
        println!("    description: {desc}");
    }
    if let Some(board) = &profile.default_board {
        println!("    default_board: {board}");
    }

    println!("    PDs by trust domain:");
    let mut by_domain: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for pd in &profile.pds {
        by_domain
            .entry(pd_trust_domain(pd))
            .or_default()
            .push(pd.as_str());
    }
    for (domain, pds) in by_domain {
        println!("      {domain:10} {}", pds.join(", "));
    }

    println!("    High-risk edges (capability surface):");
    let mut notes: Vec<String> = Vec::new();
    if profile.pds.iter().any(|p| p == "shell") {
        if profile_has_channel_between(profile, "shell", "supervisor") {
            notes.push("shell ↔ supervisor (reboot / status IPC)".into());
        }
        if profile_has_channel_between(profile, "shell", "config-server") {
            notes.push(
                "shell ↔ config-server (policy R/W; secret.* write denied — supervisor only)"
                    .into(),
            );
        }
        if profile_has_channel_between(profile, "shell", "fs-server") {
            notes.push("shell ↔ fs-server (full FS RPC)".into());
        }
        if profile_has_channel_between(profile, "shell", "net-server") {
            notes.push("shell ↔ net-server (full net RPC)".into());
        }
        for app in ["edit", "chat-client", "backup", "http-file-browser"] {
            if profile.pds.iter().any(|p| p == app)
                && profile_has_channel_between(profile, "shell", app)
            {
                notes.push(format!("shell ↔ {app} (launch / control)"));
            }
        }
    }
    for app in ["edit", "http-file-browser", "backup"] {
        if profile.pds.iter().any(|p| p == app)
            && profile_has_channel_between(profile, app, "fs-server")
        {
            notes.push(format!("{app} ↔ fs-server (untrusted app FS access)"));
        }
    }
    for app in ["chat-client", "http-file-browser", "fetch-client"] {
        if profile.pds.iter().any(|p| p == app)
            && profile_has_channel_between(profile, app, "net-server")
        {
            notes.push(format!("{app} ↔ net-server (untrusted app net access)"));
        }
    }
    if notes.is_empty() {
        println!("      (none flagged — appliance/minimal layout)");
    } else {
        for n in notes {
            println!("      - {n}");
        }
    }

    match tier {
        TrustClass::Admin => println!(
            "    note: admin surface — full REPL + bulk apps; use only on trusted consoles"
        ),
        TrustClass::AdminCore => println!(
            "    note: admin-core — shell/services without bulk apps; install apps via package CLI"
        ),
        TrustClass::Appliance => {
            println!("    note: appliance — no interactive shell; fixed PD set")
        }
        TrustClass::Minimal => println!("    note: minimal bring-up layout"),
        TrustClass::Debug => {
            println!("    note: debug/isolation only — not a production workstation default")
        }
    }
}

pub fn validate_profile(name: &str, profile: &Profile) -> Result<ChannelValidation> {
    let report = crate::channels::validate_channels(&profile.channel, &profile.pds);
    if report.ok() {
        println!(
            "profile {name}: ok ({} channels, {} pds)",
            profile.channel.len(),
            profile.pds.len()
        );
    } else {
        report.clone().into_result(&format!("profile {name}"))?;
    }
    Ok(report)
}

pub fn validate_all_profiles(profiles: &Profiles) -> Result<()> {
    let mut failed = 0usize;
    for (name, p) in profiles {
        match validate_profile(name, p) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{e:#}");
                failed += 1;
            }
        }
    }
    if failed > 0 {
        anyhow::bail!("{failed} profile(s) failed channel validation");
    }
    Ok(())
}

pub fn diff_profiles(a_name: &str, a: &Profile, b_name: &str, b: &Profile) {
    println!("diff {a_name} vs {b_name}");
    println!("template: {} | {}", a.template, b.template);
    println!(
        "default_board: {} | {}",
        a.default_board.as_deref().unwrap_or("-"),
        b.default_board.as_deref().unwrap_or("-")
    );

    println!("\nPDs in {a_name} only:");
    for pd in &a.pds {
        if !b.pds.contains(pd) {
            println!("  + {pd}");
        }
    }
    println!("PDs in {b_name} only:");
    for pd in &b.pds {
        if !a.pds.contains(pd) {
            println!("  - {pd}");
        }
    }
    println!(
        "Common PDs: {}",
        a.pds.iter().filter(|p| b.pds.contains(p)).count()
    );

    let a_lines: BTreeMap<String, &ChannelSpec> = a
        .channel
        .iter()
        .map(|c| (canonical_channel_key(c), c))
        .collect();
    let b_lines: BTreeMap<String, &ChannelSpec> = b
        .channel
        .iter()
        .map(|c| (canonical_channel_key(c), c))
        .collect();

    println!("\nChannels in {a_name} only:");
    for key in a_lines.keys() {
        if !b_lines.contains_key(key) {
            println!("  + {}", a_lines[key]);
        }
    }
    println!("Channels in {b_name} only:");
    for key in b_lines.keys() {
        if !a_lines.contains_key(key) {
            println!("  - {}", b_lines[key]);
        }
    }
    let common = a_lines
        .keys()
        .filter(|k| b_lines.contains_key(k.as_str()))
        .count();
    println!("Common channels: {common}");
}

/// Diff profile TOML topology and the composed Microkit SDF for each profile.
pub fn diff_profiles_with_sdf(
    root: &Path,
    a_name: &str,
    a: &Profile,
    b_name: &str,
    b: &Profile,
) -> Result<()> {
    diff_profiles(a_name, a, b_name, b);
    println!();
    let sdf_a = crate::system::render_profile_system(root, a_name, a, None)
        .with_context(|| format!("render SDF for profile {a_name}"))?;
    let sdf_b = crate::system::render_profile_system(root, b_name, b, None)
        .with_context(|| format!("render SDF for profile {b_name}"))?;
    print!(
        "{}",
        crate::system::sdf_diff_summary(a_name, &sdf_a, b_name, &sdf_b)
    );
    Ok(())
}

/// Path to a profile TOML under `support/profiles/`.
pub fn profile_path(root: &Path, name: &str) -> PathBuf {
    root.join("support/profiles").join(format!("{name}.toml"))
}

/// Rewrite a profile file (Phase 55 install/remove). Preserves no free comments.
pub fn save_profile(root: &Path, name: &str, profile: &Profile) -> Result<PathBuf> {
    ensure_channels_valid(&profile.channel, &profile.pds, &format!("profile {name}"))?;
    let path = profile_path(root, name);
    let body = toml::to_string_pretty(profile).context("serialize profile")?;
    let header = format!(
        "# Profile `{name}` — managed by `lerux package install|remove` (Phase 55).\n\
         # Template + default_board are authoritative; channels are the IPC source of truth.\n\n"
    );
    fs::write(&path, format!("{header}{body}"))
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

/// Order-independent key for channel equality (ends sorted by pd:id).
fn canonical_channel_key(ch: &ChannelSpec) -> String {
    let mut ends: Vec<String> = ch
        .ends
        .iter()
        .map(|e| {
            format!(
                "{}:{}{}",
                to_sdf_pd_name(&e.pd),
                e.id,
                if e.pp { "pp" } else { "" }
            )
        })
        .collect();
    ends.sort();
    ends.join("|")
}

/// Resolve a board name for a profile. Prefers explicit board, then profile.default_board.
pub fn resolve_board_for_profile(
    profiles: &Profiles,
    name: &str,
    explicit_board: Option<&str>,
) -> Result<String> {
    if let Some(b) = explicit_board {
        return Ok(b.to_string());
    }
    let p = get_profile(profiles, name)?;
    p.default_board.clone().ok_or_else(|| {
        anyhow::anyhow!("profile {name} has no default_board and none was --board supplied")
    })
}

/// Find the profile whose `default_board` matches `board_name` (if any).
///
/// Used by system generation to splice structured channels into the board template.
pub fn find_profile_for_board<'a>(
    profiles: &'a Profiles,
    board_name: &str,
) -> Option<(&'a str, &'a Profile)> {
    // Multiple recipes may share a default_board (e.g. workstation vs dev-workstation).
    // Prefer the fullest PD set so board-level `lerux system` matches the product profile.
    profiles
        .iter()
        .filter(|(_, p)| p.default_board.as_deref() == Some(board_name))
        .max_by_key(|(_, p)| p.pds.len())
        .map(|(name, p)| (name.as_str(), p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_trust_class_from_pds() {
        assert_eq!(
            infer_trust_class(&["hello".into(), "serial-driver".into()]),
            TrustClass::Minimal
        );
        assert_eq!(
            infer_trust_class(&[
                "http-server".into(),
                "serial-driver".into(),
                "virtio-net-driver".into()
            ]),
            TrustClass::Appliance
        );
        assert_eq!(
            infer_trust_class(&["shell".into(), "fs-server".into(), "supervisor".into()]),
            TrustClass::AdminCore
        );
        assert_eq!(
            infer_trust_class(&[
                "shell".into(),
                "edit".into(),
                "fs-server".into(),
                "supervisor".into()
            ]),
            TrustClass::Admin
        );
        assert_eq!(
            infer_trust_class(&["debug-handler".into(), "crash-demo".into()]),
            TrustClass::Debug
        );
    }

    #[test]
    fn pd_trust_domain_map() {
        assert_eq!(pd_trust_domain("serial-driver"), "platform");
        assert_eq!(pd_trust_domain("fs-server"), "service");
        assert_eq!(pd_trust_domain("supervisor"), "control");
        assert_eq!(pd_trust_domain("shell"), "untrusted");
        assert_eq!(pd_trust_domain("debug-handler"), "debug");
    }
}
