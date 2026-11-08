//! GitHub `releases/download` sometimes returns HTTP 504 while the API asset
//! redirect still serves the same bytes. Callers try the release URL first.

use std::{fmt::Write, path::Path};

use anyhow::{bail, Context, Result};

use crate::process::save_url;

pub(crate) enum Recover {
    NotRelease,
    Done,
    Failed(anyhow::Error),
}

pub(crate) fn recover(url: &str, dest: &Path) -> Recover {
    let Some(asset) = asset_from_url(url) else {
        return Recover::NotRelease;
    };
    eprintln!("==> {url} failed; fetching the asset through the GitHub API");
    match download_asset(&asset, dest) {
        Ok(()) => Recover::Done,
        Err(err) => Recover::Failed(err),
    }
}

struct ReleaseAsset {
    owner: String,
    repo: String,
    tag: String,
    name: String,
}

fn asset_from_url(url: &str) -> Option<ReleaseAsset> {
    let rest = url.strip_prefix("https://github.com/")?;
    let (owner, rest) = rest.split_once('/')?;
    let (repo, rest) = rest.split_once("/releases/download/")?;
    let (tag, name) = rest.rsplit_once('/')?;
    if owner.is_empty() || repo.is_empty() || tag.is_empty() || name.is_empty() {
        return None;
    }
    Some(ReleaseAsset {
        owner: owner.to_owned(),
        repo: repo.to_owned(),
        tag: tag.to_owned(),
        name: name.to_owned(),
    })
}

fn release_api(asset: &ReleaseAsset) -> String {
    format!(
        "https://api.github.com/repos/{}/{}/releases/tags/{}",
        asset.owner,
        asset.repo,
        path_encode(&asset.tag)
    )
}

fn path_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            _ => {
                let _ = write!(out, "%{byte:02X}");
            }
        }
    }
    out
}

fn download_asset(asset: &ReleaseAsset, dest: &Path) -> Result<()> {
    let body = read_release(&release_api(asset))?;
    let asset_url = asset_api_url(&body, &asset.name)?;
    save_url(&asset_url, Some("application/octet-stream"), dest)
}

fn read_release(url: &str) -> Result<String> {
    let response = crate::process::fetch_with_retry(url, Some("application/vnd.github+json"))?;
    response
        .into_body()
        .read_to_string()
        .with_context(|| format!("read {url}"))
}

fn asset_api_url(body: &str, name: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(body).context("parse release JSON")?;
    let assets = value
        .get("assets")
        .and_then(serde_json::Value::as_array)
        .context("release assets")?;
    for asset in assets {
        if asset.get("name").and_then(serde_json::Value::as_str) == Some(name) {
            return asset
                .get("url")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .context("asset url");
        }
    }
    bail!("release has no asset named {name}")
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::{asset_api_url, asset_from_url, release_api};

    #[test]
    fn splits_a_release_url() {
        let url = "https://github.com/casey/just/releases/download/1.59.0/\
            just-1.59.0-x86_64-unknown-linux-musl.tar.gz";
        let asset = asset_from_url(url).expect("release url");
        assert_eq!(asset.owner, "casey");
        assert_eq!(asset.repo, "just");
        assert_eq!(asset.tag, "1.59.0");
        assert_eq!(asset.name, "just-1.59.0-x86_64-unknown-linux-musl.tar.gz");
        assert_eq!(
            release_api(&asset),
            "https://api.github.com/repos/casey/just/releases/tags/1.59.0"
        );
    }

    #[test]
    fn encodes_a_slash_in_the_tag() {
        let url = "https://github.com/verus-lang/verus/releases/download/\
            release/0.2026.10.18.7d82994/verus.zip";
        let asset = asset_from_url(url).expect("release url");
        assert_eq!(asset.tag, "release/0.2026.10.18.7d82994");
        assert_eq!(asset.name, "verus.zip");
        assert_eq!(
            release_api(&asset),
            "https://api.github.com/repos/verus-lang/verus/releases/tags/release%2F0.2026.10.18.7d82994"
        );
    }

    #[test]
    fn ignores_urls_that_are_not_release_assets() {
        assert!(asset_from_url("https://developer.arm.com/download/file.tar.xz").is_none());
        assert!(
            asset_from_url("https://github.com/casey/just/archive/refs/tags/1.59.0.tar.gz")
                .is_none()
        );
    }

    #[test]
    fn reads_the_asset_api_url_for_the_named_file() {
        let body = r#"{"assets":[
            {"name":"other","url":"https://api.github.com/repos/o/r/releases/assets/1"},
            {"name":"just.tar.gz","url":"https://api.github.com/repos/o/r/releases/assets/2",
             "uploader":{"url":"https://api.github.com/users/a"}}
        ]}"#;
        let url = asset_api_url(body, "just.tar.gz").expect("asset");
        assert_eq!(url, "https://api.github.com/repos/o/r/releases/assets/2");
        assert!(asset_api_url(body, "missing").is_err());
    }

    #[test]
    fn api_downloads_the_pinned_just_archive() {
        let url = "https://github.com/casey/just/releases/download/1.59.0/\
            just-1.59.0-x86_64-unknown-linux-musl.tar.gz";
        let asset = asset_from_url(url).expect("release url");
        let tmp = tempfile::NamedTempFile::new().expect("temp");
        super::download_asset(&asset, tmp.path()).expect("api download");
        let bytes = std::fs::read(tmp.path()).expect("read");
        let digest = Sha256::digest(&bytes);
        assert_eq!(
            format!("{digest:x}"),
            "409044617b6f06517c1035e55ca38c8c856ff8b650e41bc2aa1ef82d5903a23e"
        );
    }
}
