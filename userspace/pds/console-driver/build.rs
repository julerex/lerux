//! The Swift panel banner uses `git describe --always --dirty`.
//!
//! There is no release tag and no GitHub release. The Cargo package version
//! is the scaffold `0.1.0` on every crate, and the ISO builder has no version
//! field. A missing git command falls back to that package version.

use std::{env, path::PathBuf, process::Command};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let root = manifest.join("../../..");
    let describe = git_describe(&root)
        .unwrap_or_else(|| env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string()));
    println!("cargo:rustc-env=LERUX_GIT_DESCRIBE={describe}");
    println!("cargo:rerun-if-changed={}/.git/HEAD", root.display());
}

fn git_describe(root: &std::path::Path) -> Option<String> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["describe", "--always", "--dirty"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim().strip_prefix('v').unwrap_or(text.trim());
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_graphic()) {
        return None;
    }
    Some(text.to_string())
}
