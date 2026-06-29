use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use anyhow::{bail, Context, Result};

pub fn path_str(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub fn binary_parent(path: &Path) -> Result<PathBuf> {
    path.parent()
        .map(Path::to_path_buf)
        .with_context(|| format!("{} has no parent directory", path.display()))
}

pub fn repo_root() -> Result<PathBuf> {
    if let Ok(root) = std::env::var("LERUX_ROOT") {
        return Ok(PathBuf::from(root));
    }

    let mut dir = std::env::current_dir().context("current directory")?;
    loop {
        let cargo = dir.join("Cargo.toml");
        if cargo.is_file() {
            let contents = std::fs::read_to_string(&cargo).unwrap_or_default();
            if contents.contains("lerux-cli") && contents.contains("userspace/pds/hello") {
                return Ok(dir);
            }
        }
        if !dir.pop() {
            break;
        }
    }
    bail!("could not find lerux repository root (set LERUX_ROOT)")
}

pub fn run_checked(program: impl AsRef<OsStr>, args: &[impl AsRef<OsStr>]) -> Result<Output> {
    let output = Command::new(program.as_ref())
        .args(args.iter().map(AsRef::as_ref))
        .output()
        .with_context(|| format!("failed to run {:?}", program.as_ref()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "command {:?} {:?} failed ({})\nstdout: {}\nstderr: {}",
            program.as_ref(),
            args.iter()
                .map(|a| a.as_ref().to_string_lossy())
                .collect::<Vec<_>>(),
            output.status,
            stdout,
            stderr
        );
    }
    Ok(output)
}

pub fn run_inherit(program: impl AsRef<OsStr>, args: &[impl AsRef<OsStr>]) -> Result<()> {
    let status = Command::new(program.as_ref())
        .args(args.iter().map(AsRef::as_ref))
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to run {:?}", program.as_ref()))?;
    if !status.success() {
        bail!("command {:?} exited with {}", program.as_ref(), status);
    }
    Ok(())
}

pub fn command_on_path(name: &str) -> bool {
    which::which(name).is_ok()
}

pub fn ensure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).with_context(|| format!("mkdir {}", path.display()))
}

pub fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    std::fs::write(path, contents).with_context(|| format!("write {}", path.display()))
}

pub fn download(url: &str, dest: &Path) -> Result<()> {
    eprintln!("==> Downloading {url}");
    let response = ureq::get(url)
        .call()
        .with_context(|| format!("GET {url}"))?;
    let mut reader = response.into_body().into_reader();
    let mut file =
        std::fs::File::create(dest).with_context(|| format!("create {}", dest.display()))?;
    std::io::copy(&mut reader, &mut file).context("download body")?;
    Ok(())
}

pub fn apt_deb_url(package: &str, fallback: &str) -> String {
    let output = Command::new("apt-cache")
        .args(["show", package])
        .output()
        .ok();
    if let Some(output) = output
        && output.status.success()
    {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some(path) = line.strip_prefix("Filename: ") {
                return format!("http://archive.ubuntu.com/ubuntu/{path}");
            }
        }
    }
    fallback.to_string()
}
