//! Package manifests (`support/packages/`) and pins (`support/package-pins.toml`).
//!
//! A package is one PD crate + an `lerux-interface-types` version + an optional
//! profile fragment. Installing means merging the fragment into a profile and
//! rebuilding `loader.img` — Microkit does not load ELFs at runtime.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{build, channels::ChannelSpec};

/// Default profile for `lerux package install`.
pub const DEFAULT_INSTALL_PROFILE: &str = "dev-workstation";

/// One package under `support/packages/<name>.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Package {
    /// PD crate name (Cargo package / ELF basename).
    pub pd: String,
    /// Semver of `lerux-interface-types` this PD was built against.
    pub interface_types: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Optional profile fragment (PD + channel deltas for composition docs).
    #[serde(default)]
    pub fragment: Option<PackageFragment>,
}

/// Deltas documenting how to wire a package into a system profile.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PackageFragment {
    #[serde(default)]
    pub pds: Vec<String>,
    /// Structured channel edges (`[[fragment.channel]]`).
    #[serde(default)]
    pub channel: Vec<ChannelSpec>,
    /// Other PD crates this package expects to be present.
    #[serde(default)]
    pub requires: Vec<String>,
}

pub type Packages = BTreeMap<String, Package>;

/// Committed CI pin registry for published PD ELF artifacts.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PackagePins {
    #[serde(default)]
    pub packages: BTreeMap<String, PackagePinEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PackagePinEntry {
    pub pd: String,
    pub interface_types: String,
    #[serde(default)]
    pub artifacts: BTreeMap<String, ArtifactPin>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArtifactPin {
    pub elf_sha256: String,
    pub target: String,
    #[serde(default)]
    pub built_at_ref: Option<String>,
}

pub fn load_packages(root: &Path) -> Result<Packages> {
    let dir = root.join("support/packages");
    if !dir.exists() {
        return Ok(BTreeMap::new());
    }

    let mut packages = BTreeMap::new();
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
        let package: Package =
            toml::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
        packages.insert(name, package);
    }
    Ok(packages)
}

/// Protection-domain names from package fragments, in system-description spelling.
pub fn optional_component_names(packages: &Packages) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for package in packages.values() {
        let Some(fragment) = &package.fragment else {
            continue;
        };
        for pd in &fragment.pds {
            names.insert(crate::channels::to_sdf_pd_name(pd));
        }
    }
    names
}

pub fn get_package<'a>(packages: &'a Packages, name: &str) -> Result<&'a Package> {
    packages
        .get(name)
        .with_context(|| format!("unknown package {name:?} (run `lerux package list`)"))
}

pub fn list_packages(packages: &Packages) {
    if packages.is_empty() {
        println!("(no packages found in support/packages/)");
        return;
    }
    for (name, p) in packages {
        let desc = p.description.as_deref().unwrap_or("");
        println!("{name:24} pd={} iface={}  {desc}", p.pd, p.interface_types);
    }
}

pub fn show_package(name: &str, package: &Package) {
    println!("package: {name}");
    println!("  pd: {}", package.pd);
    println!("  interface_types: {}", package.interface_types);
    if let Some(desc) = &package.description {
        println!("  description: {desc}");
    }
    if let Some(frag) = &package.fragment {
        if !frag.pds.is_empty() {
            println!("  fragment.pds: {}", frag.pds.join(", "));
        }
        if !frag.requires.is_empty() {
            println!("  fragment.requires: {}", frag.requires.join(", "));
        }
        if !frag.channel.is_empty() {
            println!("  fragment.channels:");
            for ch in &frag.channel {
                println!("    - {ch}");
            }
        }
    }
}

pub fn interface_types_version(root: &Path) -> Result<String> {
    let path = root.join("userspace/crates/lerux-interface-types/Cargo.toml");
    let contents = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let value: toml::Value =
        toml::from_str(&contents).with_context(|| format!("parse {}", path.display()))?;
    value
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .context("lerux-interface-types Cargo.toml missing package.version")
}

pub fn load_pins(root: &Path) -> Result<PackagePins> {
    let path = root.join("support/package-pins.toml");
    if !path.exists() {
        return Ok(PackagePins::default());
    }
    let contents = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&contents).with_context(|| format!("parse {}", path.display()))
}

pub fn save_pins(root: &Path, pins: &PackagePins) -> Result<()> {
    let path = root.join("support/package-pins.toml");
    let body = toml::to_string_pretty(pins).context("serialize package-pins")?;
    let header = "# CI-published PD ELF pins (Phase 40).\n\
# Updated by `lerux package pin`. Artifacts are rebuild inputs for profiles;\n\
# Microkit still assembles loader.img from source builds in the usual path.\n\n";
    fs::write(&path, format!("{header}{body}")).with_context(|| format!("write {}", path.display()))
}

fn elf_path(root: &Path, board: &str, build_dir: &str, pd: &str) -> PathBuf {
    root.join(build_dir).join(board).join(format!("{pd}.elf"))
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(format!("{digest:x}"))
}

/// Build one package PD for a board (wraps `build_pd`).
pub fn build_package(
    root: &Path,
    packages: &Packages,
    name: &str,
    board: &str,
    build_dir: &str,
    config: &str,
) -> Result<PathBuf> {
    let package = get_package(packages, name)?;
    let iface = interface_types_version(root)?;
    if package.interface_types != iface {
        bail!(
            "package {name} pins interface_types={} but tree has {iface}",
            package.interface_types
        );
    }
    build::build_pd(root, board, build_dir, config, &package.pd)?;
    let path = elf_path(root, board, build_dir, &package.pd);
    if !path.exists() {
        bail!("expected ELF missing at {}", path.display());
    }
    println!(
        "package {name}: built {} for board {board} ({})",
        package.pd,
        path.display()
    );
    Ok(path)
}

/// Record a pin for a built package ELF.
pub fn pin_package(
    root: &Path,
    packages: &Packages,
    name: &str,
    board: &str,
    build_dir: &str,
    git_ref: Option<&str>,
) -> Result<()> {
    let package = get_package(packages, name)?;
    let path = elf_path(root, board, build_dir, &package.pd);
    if !path.exists() {
        bail!(
            "ELF not found at {} — run `lerux package build {name} --board {board}` first",
            path.display()
        );
    }
    let boards = crate::board::load_boards(root)?;
    let board_cfg = crate::board::get_board(&boards, board)?;
    let hash = sha256_file(&path)?;

    let mut pins = load_pins(root)?;
    let entry = pins.packages.entry(name.to_string()).or_default();
    entry.pd = package.pd.clone();
    entry.interface_types = package.interface_types.clone();
    entry.artifacts.insert(
        board.to_string(),
        ArtifactPin {
            elf_sha256: hash.clone(),
            target: board_cfg.target.clone(),
            built_at_ref: git_ref.map(str::to_string),
        },
    );
    save_pins(root, &pins)?;
    println!("package {name}: pinned {board} sha256={hash}");
    Ok(())
}

pub fn diff_package_pins(root: &Path, name: &str, board: &str, build_dir: &str) -> Result<()> {
    let packages = load_packages(root)?;
    let package = get_package(&packages, name)?;
    let pins = load_pins(root)?;
    let path = elf_path(root, board, build_dir, &package.pd);
    if !path.exists() {
        bail!("ELF not found at {}", path.display());
    }
    let current = sha256_file(&path)?;
    match pins.packages.get(name).and_then(|e| e.artifacts.get(board)) {
        Some(pin) if pin.elf_sha256 == current => {
            println!("package {name} / {board}: pin matches ({current})");
        }
        Some(pin) => {
            println!("package {name} / {board}: DRIFT");
            println!("  pinned:  {}", pin.elf_sha256);
            println!("  current: {current}");
        }
        None => {
            println!("package {name} / {board}: no pin (current={current})");
        }
    }
    let tree_iface = interface_types_version(root)?;
    if package.interface_types != tree_iface {
        println!(
            "  interface_types: package={} tree={} (rebuild required if major drift)",
            package.interface_types, tree_iface
        );
    }
    Ok(())
}

// ── Phase 55: search / install / remove / upgrade ──────────────────────────

/// Substring search over package name, pd, and description.
pub fn search_packages(packages: &Packages, query: &str) {
    let q = query.to_ascii_lowercase();
    let mut hits = 0usize;
    for (name, p) in packages {
        let hay = format!(
            "{} {} {} {}",
            name,
            p.pd,
            p.interface_types,
            p.description.as_deref().unwrap_or("")
        )
        .to_ascii_lowercase();
        if hay.contains(&q) {
            let desc = p.description.as_deref().unwrap_or("");
            println!("{name:24} pd={} iface={}  {desc}", p.pd, p.interface_types);
            hits += 1;
        }
    }
    if hits == 0 {
        println!("(no packages match {query:?})");
    }
}

fn channel_name_key(ch: &ChannelSpec) -> Option<String> {
    ch.name
        .as_ref()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
}

fn profile_has_pd(profile: &crate::profile::Profile, pd: &str) -> bool {
    let want = crate::channels::to_sdf_pd_name(pd);
    profile
        .pds
        .iter()
        .any(|p| crate::channels::to_sdf_pd_name(p) == want)
}

/// Merge `name` into `profile_name` and return the description `install_package` writes.
///
/// Writes the profile file only when the merge added a protection domain or a channel,
/// and only after render succeeds. Creates the parent directory, then always writes
/// `system_file` with the returned string. Does not call `link_image`.
///
/// An extending profile returns
/// `refusing to rewrite profile "{profile_name}" which extends another profile`
/// before render and before any write.
pub fn merge_and_render(
    root: &Path,
    packages: &Packages,
    name: &str,
    profile_name: &str,
    board: Option<&str>,
    build_dir: &str,
) -> Result<String> {
    if crate::profile::profile_path(root, profile_name).is_file()
        && crate::profile::profile_source_extends(root, profile_name)?
    {
        bail!("refusing to rewrite profile \"{profile_name}\" which extends another profile");
    }

    let package = get_package(packages, name)?;
    let Some(frag) = &package.fragment else {
        bail!(
            "package {name} has no [fragment] — add pds/channels to support/packages/{name}.toml"
        );
    };

    let profiles = crate::profile::load_profiles(root)?;
    let mut profile = crate::profile::get_profile(&profiles, profile_name)?.clone();

    for req in &frag.requires {
        let in_frag = frag
            .pds
            .iter()
            .any(|p| crate::channels::to_sdf_pd_name(p) == crate::channels::to_sdf_pd_name(req));
        if !in_frag && !profile_has_pd(&profile, req) {
            bail!(
                "package {name} requires {req:?} which is not in profile {profile_name:?} \
                 (install prerequisites first or choose another profile)"
            );
        }
    }

    let mut added_pds = Vec::new();
    for pd in &frag.pds {
        if !profile_has_pd(&profile, pd) {
            profile.pds.push(pd.clone());
            added_pds.push(pd.clone());
        }
    }

    let existing_names: BTreeMap<String, ()> = profile
        .channel
        .iter()
        .filter_map(channel_name_key)
        .map(|n| (n, ()))
        .collect();

    let mut added_ch = 0usize;
    for ch in &frag.channel {
        if let Some(n) = channel_name_key(ch)
            && existing_names.contains_key(&n)
        {
            println!("  skip channel {n} (already in profile)");
            continue;
        }
        profile.channel.push(ch.clone());
        added_ch += 1;
    }

    let board_name = crate::profile::resolve_board_for_profile(&profiles, profile_name, board)?;
    let rendered =
        crate::system::render_profile_system(root, profile_name, &profile, Some(&board_name))?;

    if !added_pds.is_empty() || added_ch > 0 {
        let path = crate::profile::save_profile(root, profile_name, &profile)?;
        println!(
            "package {name}: installed into profile {profile_name} (+pds {added_pds:?} +{added_ch} channels)"
        );
        println!("  wrote {}", path.display());
    } else {
        println!("package {name}: already installed in profile {profile_name}");
    }

    let system_path = crate::system::system_file(root, &board_name, build_dir);
    if let Some(parent) = system_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    fs::write(&system_path, &rendered)
        .with_context(|| format!("write {}", system_path.display()))?;
    Ok(rendered)
}

/// Merge the fragment, write the system description, and link `board.pds`.
///
/// `link_image` runs on a second install too. There is no early return when the
/// package is already present.
pub fn install_package(
    root: &Path,
    packages: &Packages,
    name: &str,
    profile_name: &str,
    board: Option<&str>,
    build_dir: &str,
    config: &str,
) -> Result<()> {
    merge_and_render(root, packages, name, profile_name, board, build_dir)?;
    let profiles = crate::profile::load_profiles(root)?;
    let board_name = crate::profile::resolve_board_for_profile(&profiles, profile_name, board)?;
    let boards = crate::board::load_boards(root)?;
    let board_cfg = crate::board::get_board(&boards, &board_name)?;
    build::link_image(root, &board_name, build_dir, config, &board_cfg.pds)?;
    println!("package {name}: built loader.img for board {board_name}");
    Ok(())
}

/// Remove package fragment PDs and named channels from a profile.
pub fn remove_package(
    root: &Path,
    packages: &Packages,
    name: &str,
    profile_name: &str,
) -> Result<()> {
    let package = get_package(packages, name)?;
    let Some(frag) = &package.fragment else {
        bail!("package {name} has no [fragment] to remove");
    };

    let profiles = crate::profile::load_profiles(root)?;
    let mut profile = crate::profile::get_profile(&profiles, profile_name)?.clone();

    let remove_pds: BTreeMap<String, ()> = frag
        .pds
        .iter()
        .map(|p| (crate::channels::to_sdf_pd_name(p), ()))
        .collect();
    let before_pd = profile.pds.len();
    profile
        .pds
        .retain(|p| !remove_pds.contains_key(&crate::channels::to_sdf_pd_name(p)));
    let removed_pd = before_pd - profile.pds.len();

    let remove_ch: BTreeMap<String, ()> = frag
        .channel
        .iter()
        .filter_map(channel_name_key)
        .map(|n| (n, ()))
        .collect();
    let before_ch = profile.channel.len();
    profile.channel.retain(|ch| match channel_name_key(ch) {
        Some(n) => !remove_ch.contains_key(&n),
        None => true,
    });
    let removed_ch = before_ch - profile.channel.len();

    if removed_pd == 0 && removed_ch == 0 {
        println!("package {name}: not present in profile {profile_name}");
        return Ok(());
    }

    let path = crate::profile::save_profile(root, profile_name, &profile)?;
    println!(
        "package {name}: removed from profile {profile_name} (-{removed_pd} pds -{removed_ch} channels)"
    );
    println!("  wrote {}", path.display());
    Ok(())
}

/// Rebuild + re-pin one package; print SHA256 / interface_types delta vs old pin.
pub fn upgrade_package(
    root: &Path,
    packages: &Packages,
    name: &str,
    board: &str,
    build_dir: &str,
    config: &str,
    git_ref: Option<&str>,
) -> Result<()> {
    let package = get_package(packages, name)?;
    let pins_before = load_pins(root)?;
    let old = pins_before
        .packages
        .get(name)
        .and_then(|e| e.artifacts.get(board))
        .cloned();
    let old_iface = pins_before
        .packages
        .get(name)
        .map(|e| e.interface_types.clone());

    let path = build_package(root, packages, name, board, build_dir, config)?;
    let new_hash = sha256_file(&path)?;
    pin_package(root, packages, name, board, build_dir, git_ref)?;

    println!("package {name}: upgrade summary for {board}");
    match old {
        Some(prev) if prev.elf_sha256 == new_hash => {
            println!("  elf_sha256: unchanged ({new_hash})");
        }
        Some(prev) => {
            println!("  elf_sha256:");
            println!("    - {}", prev.elf_sha256);
            println!("    + {new_hash}");
        }
        None => println!("  elf_sha256: (new pin) {new_hash}"),
    }
    let tree_iface = interface_types_version(root)?;
    match old_iface {
        Some(prev) if prev == package.interface_types => {
            println!("  interface_types: {prev} (tree={tree_iface})");
        }
        Some(prev) => {
            println!(
                "  interface_types: {prev} → {} (tree={tree_iface})",
                package.interface_types
            );
            if prev != tree_iface {
                println!(
                    "  warning: interface_types drift may break postcard IPC until all PDs rebuild"
                );
            }
        }
        None => println!(
            "  interface_types: {} (tree={tree_iface})",
            package.interface_types
        ),
    }
    Ok(())
}

/// Upgrade every package that has a pin for `board`.
pub fn upgrade_all(
    root: &Path,
    packages: &Packages,
    board: &str,
    build_dir: &str,
    config: &str,
    git_ref: Option<&str>,
) -> Result<()> {
    let pins = load_pins(root)?;
    let mut names: Vec<_> = pins
        .packages
        .iter()
        .filter(|(_, e)| e.artifacts.contains_key(board))
        .map(|(n, _)| n.clone())
        .collect();
    names.sort();
    if names.is_empty() {
        println!("(no pinned packages for board {board})");
        return Ok(());
    }
    for name in names {
        if packages.contains_key(&name) {
            upgrade_package(root, packages, &name, board, build_dir, config, git_ref)?;
        } else {
            println!("skip {name}: no support/packages/{name}.toml");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        channels::{ChannelEnd, ChannelSpec},
        profile::Profile,
    };
    use std::fs;

    #[test]
    fn search_matches_description() {
        let mut packages = BTreeMap::new();
        packages.insert(
            "edit".into(),
            Package {
                pd: "edit".into(),
                interface_types: "0.1.0".into(),
                description: Some("Serial TUI text editor".into()),
                fragment: None,
            },
        );
        // smoke: does not panic
        search_packages(&packages, "tui");
        search_packages(&packages, "zzz-nope");
    }

    fn write_pd_template(root: &std::path::Path, template: &str, names: &[&str]) {
        let dir = root.join("userspace/systems/templates");
        fs::create_dir_all(&dir).unwrap();
        let mut body = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<system>\n");
        for name in names {
            body.push_str(&format!(
                "    <protection_domain name=\"{name}\" priority=\"1\">\n        <program_image path=\"{name}.elf\" />\n    </protection_domain>\n"
            ));
        }
        body.push_str("</system>\n");
        fs::write(dir.join(template), body).unwrap();
    }

    fn write_board(root: &std::path::Path, board: &str, template: &str, pds: &str) {
        fs::create_dir_all(root.join("support")).unwrap();
        fs::write(
            root.join("support/boards.toml"),
            format!(
                r#"
[{board}]
arch = "aarch64"
microkit_board = "qemu_virt_aarch64"
target = "aarch64-sel4-microkit"
template = "{template}"
pds = [{pds}]
system_vars = {{}}
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn install_edit_adds_one_protection_domain() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write_pd_template(
            root,
            "workstation.system.template",
            &["shell", "fs_server", "net_server", "edit", "chat_client"],
        );
        write_board(
            root,
            "qemu_virt_aarch64_workstation",
            "workstation.system.template",
            "\"shell\", \"fs-server\", \"edit\", \"chat-client\"",
        );
        fs::create_dir_all(root.join("support/packages")).unwrap();
        fs::create_dir_all(root.join("support/profiles")).unwrap();
        fs::write(
            root.join("support/packages/edit.toml"),
            r#"
pd = "edit"
interface_types = "0.1.0"
description = "Serial TUI text editor (FS load/save via shell proxy)"

[fragment]
pds = ["edit"]
requires = ["shell", "fs-server"]

[[fragment.channel]]
name = "edit_shell"
ends = [
  { pd = "shell", id = 6, pp = true },
  { pd = "edit", id = 0 },
]

[[fragment.channel]]
name = "edit_fs"
ends = [
  { pd = "fs_server", id = 6 },
  { pd = "edit", id = 1, pp = true },
]
"#,
        )
        .unwrap();
        fs::write(
            root.join("support/packages/chat-client.toml"),
            include_str!("../../../support/packages/chat-client.toml"),
        )
        .unwrap();
        fs::write(
            root.join("support/profiles/dev-workstation.toml"),
            r#"
template = "workstation.system.template"
pds = ["shell", "fs-server"]
description = "core"
default_board = "qemu_virt_aarch64_workstation"

[[channel]]
name = "shell_fs"
ends = [
  { pd = "shell", id = 2, pp = true },
  { pd = "fs_server", id = 1 },
]
"#,
        )
        .unwrap();

        let profiles = crate::profile::load_profiles(root).unwrap();
        let profile = profiles.get("dev-workstation").unwrap();
        let before =
            crate::system::render_profile_system(root, "dev-workstation", profile, None).unwrap();
        assert!(!before.contains("protection_domain name=\"edit\""));

        let packages = load_packages(root).unwrap();
        let after =
            merge_and_render(root, &packages, "edit", "dev-workstation", None, "build").unwrap();
        let system_path =
            crate::system::system_file(root, "qemu_virt_aarch64_workstation", "build");
        assert_eq!(after.matches("protection_domain name=\"edit\"").count(), 1);
        assert_eq!(fs::read_to_string(&system_path).unwrap(), after);

        fs::write(&system_path, b"stale").unwrap();
        let again =
            merge_and_render(root, &packages, "edit", "dev-workstation", None, "build").unwrap();
        let written = fs::read_to_string(&system_path).unwrap();
        assert_ne!(written, "stale");
        assert_eq!(
            written.matches("protection_domain name=\"edit\"").count(),
            1
        );
        assert_eq!(again.matches("protection_domain name=\"edit\"").count(), 1);
    }

    #[test]
    fn install_chat_client_adds_one_protection_domain() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write_pd_template(
            root,
            "workstation.system.template",
            &["shell", "fs_server", "net_server", "edit", "chat_client"],
        );
        write_board(
            root,
            "qemu_virt_aarch64_workstation",
            "workstation.system.template",
            "\"shell\", \"fs-server\", \"net-server\", \"chat-client\"",
        );
        fs::create_dir_all(root.join("support/packages")).unwrap();
        fs::create_dir_all(root.join("support/profiles")).unwrap();
        fs::write(
            root.join("support/packages/chat-client.toml"),
            include_str!("../../../support/packages/chat-client.toml"),
        )
        .unwrap();
        fs::write(
            root.join("support/profiles/dev-workstation.toml"),
            r#"
template = "workstation.system.template"
pds = ["shell", "fs-server", "net-server"]
description = "core"
default_board = "qemu_virt_aarch64_workstation"

[[channel]]
name = "shell_fs"
ends = [
  { pd = "shell", id = 2, pp = true },
  { pd = "fs_server", id = 1 },
]
"#,
        )
        .unwrap();

        let packages = load_packages(root).unwrap();
        let after = merge_and_render(
            root,
            &packages,
            "chat-client",
            "dev-workstation",
            None,
            "build",
        )
        .unwrap();
        assert_eq!(
            after
                .matches("protection_domain name=\"chat_client\"")
                .count(),
            1
        );
    }

    #[test]
    fn install_refuses_to_rewrite_extending_profile() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("support/profiles")).unwrap();
        let path = root.join("support/profiles/workstation.toml");
        let bytes_before = b"extends = \"workstation-base\"\ntemplate = \"workstation.system.template\"\npds = [\"shell\"]\n";
        fs::write(&path, bytes_before).unwrap();
        let err = merge_and_render(root, &BTreeMap::new(), "edit", "workstation", None, "build")
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "refusing to rewrite profile \"workstation\" which extends another profile"
        );
        assert_eq!(fs::read(&path).unwrap(), bytes_before);
    }

    #[test]
    fn install_merges_channels() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write_pd_template(
            root,
            "serial-hello.system.template",
            &["shell", "serial_driver", "toy"],
        );
        write_board(
            root,
            "qemu_virt_aarch64",
            "serial-hello.system.template",
            "\"shell\", \"serial-driver\", \"toy\"",
        );
        fs::create_dir_all(root.join("support/packages")).unwrap();
        fs::create_dir_all(root.join("support/profiles")).unwrap();
        fs::write(
            root.join("support/packages/toy.toml"),
            r#"
pd = "toy"
interface_types = "0.1.0"
description = "toy"

[fragment]
pds = ["toy"]
requires = ["shell"]

[[fragment.channel]]
name = "toy_shell"
ends = [
  { pd = "shell", id = 9, pp = true },
  { pd = "toy", id = 0 },
]
"#,
        )
        .unwrap();
        fs::write(
            root.join("support/profiles/sandbox.toml"),
            r#"
template = "serial-hello.system.template"
pds = ["shell", "serial-driver"]
description = "test sandbox"
default_board = "qemu_virt_aarch64"

[[channel]]
name = "shell_serial"
ends = [
  { pd = "serial_driver", id = 1 },
  { pd = "shell", id = 0, pp = true },
]
"#,
        )
        .unwrap();

        let packages = load_packages(root).unwrap();
        merge_and_render(root, &packages, "toy", "sandbox", None, "build").unwrap();
        let profiles = crate::profile::load_profiles(root).unwrap();
        let p = profiles.get("sandbox").unwrap();
        assert!(p.pds.iter().any(|x| x == "toy"));
        assert!(p
            .channel
            .iter()
            .any(|c| c.name.as_deref() == Some("toy_shell")));

        remove_package(root, &packages, "toy", "sandbox").unwrap();
        let profiles = crate::profile::load_profiles(root).unwrap();
        let p = profiles.get("sandbox").unwrap();
        assert!(!p.pds.iter().any(|x| x == "toy"));
        assert!(!p
            .channel
            .iter()
            .any(|c| c.name.as_deref() == Some("toy_shell")));
    }

    #[test]
    fn install_requires_missing_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("support/packages")).unwrap();
        fs::create_dir_all(root.join("support/profiles")).unwrap();
        fs::write(
            root.join("support/packages/toy.toml"),
            r#"
pd = "toy"
interface_types = "0.1.0"
[fragment]
pds = ["toy"]
requires = ["missing-pd"]
"#,
        )
        .unwrap();
        fs::write(
            root.join("support/profiles/sandbox.toml"),
            r#"
template = "serial-hello.system.template"
pds = ["shell"]
"#,
        )
        .unwrap();
        let packages = load_packages(root).unwrap();
        let err = merge_and_render(root, &packages, "toy", "sandbox", None, "build").unwrap_err();
        assert!(err.to_string().contains("requires"));
    }

    #[test]
    fn channel_name_key_works() {
        let ch = ChannelSpec {
            name: Some("edit_shell".into()),
            ends: vec![
                ChannelEnd {
                    pd: "shell".into(),
                    id: 6,
                    pp: true,
                },
                ChannelEnd {
                    pd: "edit".into(),
                    id: 0,
                    pp: false,
                },
            ],
        };
        assert_eq!(channel_name_key(&ch).as_deref(), Some("edit_shell"));
        let _ = Profile {
            template: "t".into(),
            pds: vec![],
            description: None,
            default_board: None,
            trust_class: None,
            channel: vec![],
        };
    }
}
