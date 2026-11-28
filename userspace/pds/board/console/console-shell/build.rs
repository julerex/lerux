//! The issue line uses `git describe --always --dirty`.
//!
//! Each `just make-lerux-iso-*` recipe passes that string in `LERUX_GIT_DESCRIBE`. Cargo
//! reruns this script when the variable changes, including an unstaged edit
//! that appends `-dirty`.

use std::{env, path::PathBuf, process::Command};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let root = manifest.join("../../../../..");
    println!("cargo:rerun-if-env-changed=LERUX_GIT_DESCRIBE");
    let describe = env_describe().unwrap_or_else(|| {
        git_describe(&root).unwrap_or_else(|| {
            env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string())
        })
    });
    println!("cargo:rustc-env=LERUX_GIT_DESCRIBE={describe}");
}

fn env_describe() -> Option<String> {
    accept_describe(&env::var("LERUX_GIT_DESCRIBE").ok()?)
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
    accept_describe(&String::from_utf8(output.stdout).ok()?)
}

fn accept_describe(text: &str) -> Option<String> {
    let text = text.trim().strip_prefix('v').unwrap_or(text.trim());
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_graphic()) {
        return None;
    }
    Some(text.to_string())
}
