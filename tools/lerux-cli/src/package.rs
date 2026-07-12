//! Package manifests (`support/packages/`) and pins (`support/package-pins.toml`).
//!
//! A package is one PD crate + an `lerux-interface-types` version + an optional
//! profile fragment. Installing means merging the fragment into a profile and
//! rebuilding `loader.img` — Microkit does not load ELFs at runtime.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{build, channels::ChannelSpec};

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
            target: board_cfg.target_triple.clone(),
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
    Ok(())
}
