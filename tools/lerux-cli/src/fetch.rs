use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::process::{download, ensure_dir, run_checked};

#[derive(Debug, Deserialize)]
struct Versions {
    repos: Repos,
    lionsos: RepoPin,
    #[serde(rename = "redox-drivers")]
    redox_drivers: RepoPin,
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
    let contents =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
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
    clone_or_checkout(
        &workspace,
        "lionsos",
        &versions.lionsos.remote,
        &versions.lionsos.tag,
    )?;
    // The queue and filesystem headers live in the LionsOS tree and its
    // dep/sddf submodule. The other submodules are C components and language
    // runtimes; this checkout does not build them.
    let lionsos = workspace.join("lionsos");
    run_checked(
        "git",
        &[
            "-C",
            &lionsos.to_string_lossy(),
            "submodule",
            "update",
            "--init",
            "dep/sddf",
        ],
    )
    .context("initialise LionsOS dep/sddf")?;
    // The Redox tree is a hardware reference. It is not compiled, and it has
    // no submodule this checkout initialises.
    clone_or_checkout_pin(
        &workspace,
        "redox-drivers",
        &versions.redox_drivers.remote,
        &versions.redox_drivers.tag,
    )?;

    eprintln!("==> Dependencies ready under {}", workspace.display());
    Ok(())
}

fn clone_or_checkout_pin(workspace: &Path, name: &str, url: &str, pin: &str) -> Result<()> {
    if is_full_commit(pin) {
        clone_or_checkout_commit(workspace, name, url, pin)
    } else {
        clone_or_checkout(workspace, name, url, pin)
    }
}

fn is_full_commit(pin: &str) -> bool {
    pin.len() == 40 && pin.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn clone_or_checkout_commit(workspace: &Path, name: &str, url: &str, commit: &str) -> Result<()> {
    let dest = workspace.join(name);
    prepare_commit_repo(&dest, name, url, commit)?;
    fetch_pinned_commit(&dest, name, commit)?;
    let head = head_commit(&dest)?;
    if head != commit {
        bail!("{name}: checked out {head}, pin is {commit}");
    }
    Ok(())
}

fn prepare_commit_repo(dest: &Path, name: &str, url: &str, commit: &str) -> Result<()> {
    if dest.join(".git").is_dir() {
        eprintln!("==> {name}: already cloned, checking out {commit}");
        return Ok(());
    }
    if dest.exists() {
        eprintln!(
            "==> {name}: removing existing non-repository path at {}",
            dest.display()
        );
        std::fs::remove_dir_all(dest)?;
    }
    eprintln!("==> {name}: cloning {commit}");
    let dest_arg = dest.to_string_lossy();
    run_checked("git", &["init", "-q", dest_arg.as_ref()])?;
    run_checked(
        "git",
        &["-C", dest_arg.as_ref(), "remote", "add", "origin", url],
    )?;
    Ok(())
}

fn fetch_pinned_commit(dest: &Path, name: &str, commit: &str) -> Result<()> {
    let dest_arg = dest.to_string_lossy();
    let _ = run_checked(
        "git",
        &[
            "config",
            "--global",
            "--add",
            "safe.directory",
            dest_arg.as_ref(),
        ],
    );
    // `git clone --branch` rejects a commit. GitLab serves one reachable
    // commit at depth 1, and this checkout does not initialise submodules.
    run_checked(
        "git",
        &[
            "-C",
            dest_arg.as_ref(),
            "fetch",
            "--depth",
            "1",
            "--no-recurse-submodules",
            "origin",
            commit,
        ],
    )
    .with_context(|| format!("fetch {name} {commit}"))?;
    run_checked(
        "git",
        &[
            "-C",
            dest_arg.as_ref(),
            "checkout",
            "--detach",
            "FETCH_HEAD",
        ],
    )?;
    Ok(())
}

fn head_commit(dest: &Path) -> Result<String> {
    let output = run_checked("git", &["-C", &dest.to_string_lossy(), "rev-parse", "HEAD"])
        .context("read checkout commit")?;
    let text = String::from_utf8(output.stdout).context("commit id is not utf-8")?;
    Ok(text.trim().to_string())
}

fn clone_or_checkout(workspace: &Path, name: &str, url: &str, tag: &str) -> Result<()> {
    let dest = workspace.join(name);
    if dest.join(".git").is_dir() {
        return checkout_existing(&dest, name, tag);
    }
    if dest.exists() {
        eprintln!(
            "==> {name}: removing existing non-repository path at {}",
            dest.display()
        );
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

fn checkout_existing(dest: &Path, name: &str, tag: &str) -> Result<()> {
    let _ = run_checked(
        "git",
        &[
            "config",
            "--global",
            "--add",
            "safe.directory",
            &dest.to_string_lossy(),
        ],
    );
    eprintln!("==> {name}: already cloned, checking out {tag}");
    // `--tags` retrieves every tag. Git then fetches a populated submodule
    // commit recorded by that history. Older LionsOS history records an
    // sddf commit the remote no longer has, and the fetch fails before
    // checkout. LionsOS dep/sddf is initialised after this checkout.
    run_checked(
        "git",
        &[
            "-C",
            &dest.to_string_lossy(),
            "fetch",
            "--no-recurse-submodules",
            "--tags",
            "origin",
        ],
    )?;
    run_checked("git", &["-C", &dest.to_string_lossy(), "checkout", tag])?;
    Ok(())
}

pub fn fetch_sdk(root: &Path) -> Result<()> {
    let versions = load_versions(root)?;
    let version = versions.repos.microkit.tag;
    let dest = root.join("deps/microkit-sdk");
    let url = format!(
        "https://github.com/seL4/microkit/releases/download/{version}/microkit-sdk-{version}-linux-x86-64.tar.gz"
    );

    let installed = std::fs::read_to_string(dest.join("VERSION"))
        .ok()
        .map(|text| text.trim().to_string());
    let current = installed.as_deref() == Some(version.as_str()) && dest.join("bin").is_dir();
    if current {
        eprintln!("==> SDK {version} already present at {}", dest.display());
    } else {
        download_sdk(&dest, &url, &version, installed.as_deref())?;
    }

    let sdk_path_file = root.join("deps/.sdk-path");
    crate::process::write_file(&sdk_path_file, &format!("{}\n", dest.display()))?;
    eprintln!("==> Microkit SDK: {}", dest.display());
    Ok(())
}

fn download_sdk(dest: &Path, url: &str, version: &str, installed: Option<&str>) -> Result<()> {
    if dest.exists() {
        eprintln!(
            "==> Replacing Microkit SDK at {} (installed {})",
            dest.display(),
            installed.unwrap_or("unknown")
        );
        std::fs::remove_dir_all(dest)?;
    }
    eprintln!("==> Downloading Microkit SDK {version}");
    ensure_dir(dest)?;
    let tmp = tempfile::NamedTempFile::new().context("sdk tarball")?;
    download(url, tmp.path())?;
    extract_sdk_tarball(tmp.path(), dest)?;
    let downloaded = std::fs::read_to_string(dest.join("VERSION"))
        .context("read SDK VERSION")?
        .trim()
        .to_string();
    if downloaded != version {
        bail!("SDK VERSION is {downloaded}, expected {version}");
    }
    run_checked("chmod", &["-R", "a+X", &dest.to_string_lossy()])?;
    Ok(())
}

fn extract_sdk_tarball(tarball: &Path, dest: &Path) -> Result<()> {
    let archive = tarball.to_string_lossy().into_owned();
    let dest_dir = dest.to_string_lossy().into_owned();
    let status = std::process::Command::new("tar")
        .args(["-xzf", &archive, "-C", &dest_dir, "--strip-components=1"])
        .status()
        .context("extract SDK tarball")?;
    if !status.success() {
        bail!("SDK download failed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::load_versions;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn parses_lionsos_pin() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("lerux-versions-{nanos}"));
        let deps = root.join("deps");
        fs::create_dir_all(&deps).expect("temp deps");
        let fixture: PathBuf = deps.join("versions.toml");
        fs::write(
            &fixture,
            r#"
[repos]
sel4 = { remote = "https://example.com/seL4.git", tag = "15.0.0" }
microkit = { remote = "https://example.com/microkit.git", tag = "2.2.0" }

[lionsos]
tag = "0.4.0"
remote = "https://github.com/au-ts/lionsos.git"

[redox-drivers]
tag = "20ffe4d7f4a85b7cc1f59495d7e6e355fed4cb06"
remote = "https://gitlab.redox-os.org/redox-os/drivers.git"
"#,
        )
        .expect("write fixture");

        let versions = load_versions(&root).expect("parse fixture");
        assert_eq!(versions.lionsos.tag, "0.4.0");
        assert_eq!(
            versions.lionsos.remote,
            "https://github.com/au-ts/lionsos.git"
        );
        assert_eq!(
            versions.redox_drivers.tag,
            "20ffe4d7f4a85b7cc1f59495d7e6e355fed4cb06"
        );
        assert_eq!(
            versions.redox_drivers.remote,
            "https://gitlab.redox-os.org/redox-os/drivers.git"
        );
        assert!(super::is_full_commit(&versions.redox_drivers.tag));
        assert!(!super::is_full_commit(&versions.lionsos.tag));

        fs::remove_dir_all(&root).expect("remove temp");
    }
}
