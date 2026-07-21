//! Phase 60 Track C: host-side `loader.img` integrity via SHA-256 sidecars.
//!
//! After `lerux image`, a `loader.img.sha256` file is written next to the image
//! in `sha256sum`-compatible form. `lerux deploy` verifies the digest before
//! copy (default). Hardware measured boot / asymmetric signing remain future work.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

/// Path of the sidecar for a loader image (`loader.img` → `loader.img.sha256`).
pub fn sidecar_path(loader: &Path) -> PathBuf {
    let mut s = loader.as_os_str().to_os_string();
    s.push(".sha256");
    PathBuf::from(s)
}

/// Resolve `build/<board>/loader.img` under the repo root.
pub fn loader_path(root: &Path, board: &str, build_dir: &str) -> PathBuf {
    root.join(build_dir).join(board).join("loader.img")
}

/// SHA-256 hex digest of a file (lowercase).
pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(format!("{digest:x}"))
}

/// Parse a sidecar body: first whitespace-separated field on the first
/// non-empty, non-`#` line must be a 64-char hex digest (sha256sum format).
pub fn parse_sidecar(text: &str) -> Result<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let hash = line
            .split_whitespace()
            .next()
            .context("empty digest line in sidecar")?;
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            bail!("invalid sha256 in sidecar (want 64 hex chars): {hash:?}");
        }
        return Ok(hash.to_ascii_lowercase());
    }
    bail!("sidecar has no digest line");
}

/// Write `loader.img.sha256` next to `loader` (`hex  filename\\n`).
pub fn write_sidecar(loader: &Path) -> Result<String> {
    if !loader.is_file() {
        bail!("missing image {}", loader.display());
    }
    let hash = sha256_file(loader)?;
    let name = loader
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("loader.img");
    let body = format!("{hash}  {name}\n");
    let side = sidecar_path(loader);
    fs::write(&side, &body).with_context(|| format!("write {}", side.display()))?;
    println!("==> wrote {} (sha256={hash})", side.display());
    Ok(hash)
}

/// Verify `loader` bytes match its `.sha256` sidecar.
pub fn verify_sidecar(loader: &Path) -> Result<String> {
    if !loader.is_file() {
        bail!("missing image {}", loader.display());
    }
    let side = sidecar_path(loader);
    if !side.is_file() {
        bail!(
            "missing integrity sidecar {} — run `lerux image` (writes digest) or `lerux digest`",
            side.display()
        );
    }
    let text = fs::read_to_string(&side).with_context(|| format!("read {}", side.display()))?;
    let expected = parse_sidecar(&text)?;
    let actual = sha256_file(loader)?;
    if actual != expected {
        bail!(
            "integrity check failed for {}:\n  expected {expected}\n  actual   {actual}\n\
             rebuild with `lerux image` or re-run `lerux digest` if the image was rebuilt",
            loader.display()
        );
    }
    println!("==> verified {} (sha256={actual})", loader.display());
    Ok(actual)
}

/// Resolve loader path from optional explicit path or board layout.
pub fn resolve_loader(
    root: &Path,
    board: &str,
    build_dir: &str,
    path: Option<&Path>,
) -> Result<PathBuf> {
    let loader = match path {
        Some(p) => {
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                root.join(p)
            }
        }
        None => loader_path(root, board, build_dir),
    };
    Ok(loader)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn sidecar_path_appends_sha256() {
        let p = Path::new("/tmp/build/board/loader.img");
        assert_eq!(
            sidecar_path(p),
            PathBuf::from("/tmp/build/board/loader.img.sha256")
        );
    }

    #[test]
    fn write_and_verify_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let loader = tmp.path().join("loader.img");
        {
            let mut f = fs::File::create(&loader).unwrap();
            f.write_all(b"fake-loader-bytes").unwrap();
        }
        let hash = write_sidecar(&loader).unwrap();
        assert_eq!(hash.len(), 64);
        let side = sidecar_path(&loader);
        assert!(side.is_file());
        let body = fs::read_to_string(&side).unwrap();
        assert!(body.starts_with(&hash));
        assert!(body.contains("loader.img"));
        assert_eq!(verify_sidecar(&loader).unwrap(), hash);
    }

    #[test]
    fn verify_detects_tamper() {
        let tmp = tempfile::tempdir().unwrap();
        let loader = tmp.path().join("loader.img");
        fs::write(&loader, b"v1").unwrap();
        write_sidecar(&loader).unwrap();
        fs::write(&loader, b"v2-tampered").unwrap();
        let err = verify_sidecar(&loader).unwrap_err().to_string();
        assert!(err.contains("integrity check failed"), "{err}");
    }

    #[test]
    fn parse_sidecar_sha256sum_and_bare() {
        assert!(parse_sidecar("abc\n")
            .unwrap_err()
            .to_string()
            .contains("invalid"));
        let hex = "a".repeat(64);
        assert_eq!(parse_sidecar(&format!("{hex}  loader.img\n")).unwrap(), hex);
        assert_eq!(parse_sidecar(&format!("# comment\n{hex}\n")).unwrap(), hex);
        assert_eq!(
            parse_sidecar(&format!("  {hex}  *loader.img\n")).unwrap(),
            hex
        );
    }
}
