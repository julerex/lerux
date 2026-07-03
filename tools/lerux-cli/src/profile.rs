use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    pub template: String,
    pub pds: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_board: Option<String>,
    /// High-level channel manifest for the profile (documentation / diffing).
    #[serde(default)]
    pub channels: Vec<String>,
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
        println!("{name:20} {desc}");
    }
}

pub fn diff_profiles(a_name: &str, a: &Profile, b_name: &str, b: &Profile) {
    println!("diff {a_name} vs {b_name}");
    println!("template: {} | {}", a.template, b.template);

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

    // channels
    println!("\nChannels in {a_name} only:");
    for ch in &a.channels {
        if !b.channels.contains(ch) {
            println!("  + {ch}");
        }
    }
    println!("Channels in {b_name} only:");
    for ch in &b.channels {
        if !a.channels.contains(ch) {
            println!("  - {ch}");
        }
    }
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
