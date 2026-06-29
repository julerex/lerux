use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::process::{ensure_dir, run_checked};

#[derive(Debug, Deserialize)]
struct Versions {
    repos: Repos,
}

#[derive(Debug, Deserialize)]
struct Repos {
    sel4: RepoPin,
    microkit: RepoPin,
}

#[derive(Debug, Deserialize)]
struct RepoPin {
    remote: String,
    tag: String,
}

fn load_versions(root: &Path) -> Result<Versions> {
    let path = root.join("deps/versions.toml");
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&contents).with_context(|| format!("parse {}", path.display()))
}

pub fn fetch(root: &Path) -> Result<()> {
    let versions = load_versions(root)?;
    let workspace = root.join("deps/workspace");
    ensure_dir(&workspace)?;

    clone_or_checkout(
        &workspace,
        "seL4",
        &versions.repos.sel4.remote,
        &versions.repos.sel4.tag,
    )?;
    clone_or_checkout(
        &workspace,
        "microkit",
        &versions.repos.microkit.remote,
        &versions.repos.microkit.tag,
    )?;

    eprintln!("==> Dependencies ready under {}", workspace.display());
    Ok(())
}

fn clone_or_checkout(workspace: &Path, name: &str, url: &str, tag: &str) -> Result<()> {
    let dest = workspace.join(name);
    if dest.join(".git").is_dir() {
        let _ = run_checked("git", &["config", "--global", "--add", "safe.directory", &dest.to_string_lossy()]);
        eprintln!("==> {name}: already cloned, checking out {tag}");
        run_checked("git", &["-C", &dest.to_string_lossy(), "fetch", "--tags", "origin"])?;
        run_checked("git", &["-C", &dest.to_string_lossy(), "checkout", tag])?;
        return Ok(());
    }

    if dest.exists() {
        eprintln!("==> {name}: removing existing non-repository path at {}", dest.display());
        std::fs::remove_dir_all(&dest)?;
    }

    eprintln!("==> {name}: cloning {tag}");
    run_checked(
        "git",
        &[
            "clone",
            "--branch",
            tag,
            "--depth",
            "1",
            url,
            &dest.to_string_lossy(),
        ],
    )?;
    Ok(())
}

pub fn fetch_sdk(root: &Path) -> Result<()> {
    let version = "2.2.0";
    let dest = root.join("deps/microkit-sdk");
    let url = format!(
        "https://github.com/seL4/microkit/releases/download/{version}/microkit-sdk-{version}-linux-x86-64.tar.gz"
    );

    if dest.join("bin").is_dir() {
        eprintln!("==> SDK already present at {}", dest.display());
    } else {
        eprintln!("==> Downloading Microkit SDK {version}");
        ensure_dir(&dest)?;
        let status = std::process::Command::new("bash")
            .arg("-c")
            .arg(format!(
                "curl -fsSL '{url}' | tar -xzf - -C '{}' --strip-components=1",
                dest.display()
            ))
            .status()
            .context("extract SDK tarball")?;
        if !status.success() {
            bail!("SDK download failed");
        }
        run_checked("chmod", &["-R", "a+X", &dest.to_string_lossy()])?;
    }

    let sdk_path_file = root.join("deps/.sdk-path");
    crate::process::write_file(&sdk_path_file, &format!("{}\n", dest.display()))?;
    eprintln!("==> Microkit SDK: {}", dest.display());
    Ok(())
}