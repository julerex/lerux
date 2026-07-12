//! Phase 47/52: load smoke expects from `support/smoke-expects.toml`.

use std::{collections::HashMap, path::Path};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::test::{ScriptStep, SmokeTest};

#[derive(Debug, Deserialize)]
struct FileRoot {
    #[serde(default)]
    defaults: Defaults,
    #[serde(default)]
    boards: HashMap<String, BoardSpec>,
}

#[derive(Debug, Default, Deserialize)]
struct Defaults {
    #[serde(default = "default_timeout")]
    timeout_secs: u64,
    #[serde(default)]
    unordered: bool,
    #[serde(default = "default_script_timeout")]
    script_timeout_secs: u64,
}

fn default_timeout() -> u64 {
    60
}

fn default_script_timeout() -> u64 {
    30
}

#[derive(Debug, Deserialize)]
struct BoardSpec {
    expects: Vec<String>,
    #[serde(default)]
    unordered: Option<bool>,
    #[serde(default)]
    timeout_secs: Option<u64>,
    /// Phase 52: after boot expects, write `send` and wait for `expect` (hw-serial).
    #[serde(default)]
    script: Vec<ScriptStepToml>,
    #[serde(default)]
    script_timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
struct ScriptStepToml {
    send: String,
    expect: String,
}

/// Path to the expects file under the repo root.
pub fn expects_path(root: &Path) -> std::path::PathBuf {
    root.join("support/smoke-expects.toml")
}

/// Build a [`SmokeTest`] for `board` from TOML data (+ default curls).
pub fn smoke_test_for_board(root: &Path, board: &str) -> Result<SmokeTest> {
    let path = expects_path(root);
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read smoke expects {}", path.display()))?;
    let file: FileRoot = toml::from_str(&text).context("parse support/smoke-expects.toml")?;

    let Some(spec) = file.boards.get(board) else {
        bail!(
            "board {board:?} missing from support/smoke-expects.toml; add a [boards.{board}] section"
        );
    };
    if spec.expects.is_empty() {
        bail!("board {board:?} has empty expects in smoke-expects.toml");
    }

    let script: Vec<ScriptStep> = spec
        .script
        .iter()
        .map(|s| ScriptStep {
            send: s.send.clone(),
            expect: s.expect.clone(),
        })
        .collect();

    Ok(SmokeTest {
        expects: spec.expects.clone(),
        curls: crate::test::default_curls(board),
        unordered: spec.unordered.unwrap_or(file.defaults.unordered),
        timeout_secs: spec
            .timeout_secs
            .unwrap_or(file.defaults.timeout_secs)
            .max(1),
        script,
        script_timeout_secs: spec
            .script_timeout_secs
            .unwrap_or(file.defaults.script_timeout_secs)
            .max(1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    #[test]
    fn loads_rpi_workstation() {
        let root = repo_root();
        let t = smoke_test_for_board(&root, "rpi4b_4gb_workstation").unwrap();
        assert!(t.unordered);
        assert_eq!(t.timeout_secs, 120);
        assert!(t.expects.iter().any(|e| e.contains("lerux-shell")));
        assert!(
            t.expects.iter().any(|e| e.contains("first-boot seed ok")),
            "Phase 52 first-boot seed expect missing"
        );
        assert!(
            !t.script.is_empty(),
            "Phase 52 scripted REPL steps expected on rpi4 workstation"
        );
        assert!(t.script.iter().any(|s| s.expect.contains("boot.log")));
    }

    #[test]
    fn loads_debug_board() {
        let root = repo_root();
        let t = smoke_test_for_board(&root, "qemu_virt_aarch64_debug").unwrap();
        assert!(!t.unordered);
        assert!(t.expects.iter().any(|e| e.contains("VmFault")));
    }

    #[test]
    fn missing_board_errors() {
        let root = repo_root();
        let err = smoke_test_for_board(&root, "no_such_board_xyz").unwrap_err();
        assert!(err.to_string().contains("missing"));
    }
}
