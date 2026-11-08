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
        .with_context(|| format!("failed to run {}", program.as_ref().to_string_lossy()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "command {} {:?} failed ({})\nstdout: {}\nstderr: {}",
            program.as_ref().to_string_lossy(),
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
        .with_context(|| format!("failed to run {}", program.as_ref().to_string_lossy()))?;
    if !status.success() {
        bail!(
            "command {} exited with {}",
            program.as_ref().to_string_lossy(),
            status
        );
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
    match save_url(url, None, dest) {
        Ok(()) => Ok(()),
        Err(err) => fallback_github_release(url, dest, err),
    }
}

fn fallback_github_release(url: &str, dest: &Path, err: anyhow::Error) -> Result<()> {
    match crate::github_release::recover(url, dest) {
        crate::github_release::Recover::Done => Ok(()),
        crate::github_release::Recover::NotRelease => Err(err),
        crate::github_release::Recover::Failed(fallback) => {
            Err(fallback.context(format!("direct download failed: {err:#}")))
        }
    }
}

pub(crate) fn save_url(url: &str, accept: Option<&str>, dest: &Path) -> Result<()> {
    copy_body(fetch_with_retry(url, accept)?, dest)
}

pub(crate) fn fetch_with_retry(
    url: &str,
    accept: Option<&str>,
) -> Result<ureq::http::Response<ureq::Body>> {
    let mut delay_secs = 1u64;
    let mut last = None;
    for attempt in 1u8..=5 {
        match fetch(url, accept) {
            Ok(response) => return Ok(response),
            Err(err) if transient(&err) && attempt < 5 => {
                eprintln!("==> GET {url} failed ({err}); retrying");
                std::thread::sleep(std::time::Duration::from_secs(delay_secs));
                delay_secs = delay_secs.saturating_mul(2);
                last = Some(err);
            }
            Err(err) => return Err(err).context(format!("GET {url}")),
        }
    }
    match last {
        Some(err) => Err(err).context(format!("GET {url}")),
        None => bail!("GET {url}"),
    }
}

fn fetch(url: &str, accept: Option<&str>) -> Result<ureq::http::Response<ureq::Body>, ureq::Error> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(20)))
        .timeout_recv_response(Some(std::time::Duration::from_secs(30)))
        .timeout_recv_body(Some(std::time::Duration::from_mins(20)))
        .build()
        .into();
    let mut request = agent.get(url).header("User-Agent", "lerux");
    if let Some(accept) = accept {
        request = request.header("Accept", accept);
    }
    if let Some(header) = authorization(url) {
        request = request.header("Authorization", &header);
    }
    request.call()
}

fn authorization(url: &str) -> Option<String> {
    bearer(url, github_token().as_deref())
}

fn github_token() -> Option<String> {
    ["GITHUB_TOKEN", "GH_TOKEN"].into_iter().find_map(|key| {
        std::env::var(key)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    })
}

fn bearer(url: &str, token: Option<&str>) -> Option<String> {
    let token = token?.trim();
    if token.is_empty() || !github_host(url) {
        return None;
    }
    Some(format!("Bearer {token}"))
}

fn github_host(url: &str) -> bool {
    url.starts_with("https://github.com/") || url.starts_with("https://api.github.com/")
}

fn copy_body(response: ureq::http::Response<ureq::Body>, dest: &Path) -> Result<()> {
    let mut reader = response.into_body().into_reader();
    let mut file =
        std::fs::File::create(dest).with_context(|| format!("create {}", dest.display()))?;
    std::io::copy(&mut reader, &mut file).context("download body")?;
    Ok(())
}

fn transient(err: &ureq::Error) -> bool {
    match err {
        ureq::Error::StatusCode(code) => matches!(code, 408 | 429 | 500 | 502 | 503 | 504),
        ureq::Error::Io(_)
        | ureq::Error::Timeout(_)
        | ureq::Error::ConnectionFailed
        | ureq::Error::HostNotFound => true,
        // `ureq::Error` is `#[non_exhaustive]`.
        _ => false,
    }
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

#[cfg(test)]
mod tests {
    use super::bearer;

    #[test]
    fn bearer_is_sent_only_to_github() {
        assert_eq!(
            bearer(
                "https://github.com/casey/just/releases/download/1.59.0/just.tar.gz",
                Some("tok")
            )
            .as_deref(),
            Some("Bearer tok")
        );
        assert_eq!(
            bearer(
                "https://api.github.com/repos/casey/just/releases/tags/1.59.0",
                Some(" tok ")
            )
            .as_deref(),
            Some("Bearer tok")
        );
        assert!(bearer("https://developer.arm.com/file.tar.xz", Some("tok")).is_none());
        assert!(bearer(
            "https://release-assets.githubusercontent.com/blob",
            Some("tok")
        )
        .is_none());
        assert!(bearer(
            "https://github.com/casey/just/releases/download/1.59.0/just.tar.gz",
            Some("  ")
        )
        .is_none());
        assert!(bearer(
            "https://github.com/casey/just/releases/download/1.59.0/just.tar.gz",
            None
        )
        .is_none());
    }
}
