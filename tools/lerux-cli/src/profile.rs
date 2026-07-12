use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::channels::{ensure_channels_valid, to_sdf_pd_name, ChannelSpec, ChannelValidation};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Profile {
    pub template: String,
    pub pds: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_board: Option<String>,
    /// Structured channel manifest (`[[channel]]` tables). Source of truth for
    /// IPC topology: `render_system` splices these into the board template when
    /// `default_board` matches (Phase 41).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channel: Vec<ChannelSpec>,
}

pub type Profiles = BTreeMap<String, Profile>;

pub fn load_profiles(root: &Path) -> Result<Profiles> {
    let dir = root.join("support/profiles");
    if !dir.exists() {
        return Ok(BTreeMap::new());
    }

    let mut profiles = BTreeMap::new();
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
        let profile: Profile =
            toml::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
        ensure_channels_valid(&profile.channel, &profile.pds, &format!("profile {name}"))
            .with_context(|| format!("validate channels in {}", path.display()))?;
        profiles.insert(name, profile);
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
        println!("{name:20} [{n:2} ch] {desc}");
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
    println!("  pds: {}", profile.pds.join(", "));
    println!("  channels ({}):", profile.channel.len());
    for ch in &profile.channel {
        println!("    - {ch}");
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
