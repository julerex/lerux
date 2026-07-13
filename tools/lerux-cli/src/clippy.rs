//! Cross-target clippy derived from `support/boards.toml`.
//!
//! Every (PD crate, board) pair in the matrix is linted with the same
//! `board-<name>` feature the build uses, so lint coverage cannot drift from
//! the board matrix. Shared userspace crates without board features are
//! linted once per arch.

use std::{collections::BTreeSet, path::Path, process::Command};

use anyhow::{bail, Context, Result};

use crate::{
    board::{crate_has_board_feature, load_boards, Boards},
    build_sdk::sdk_path,
    libclang::apply_libclang_env,
    process::{ensure_dir, path_str},
    system::shared_target_dir,
};

/// Arch pass order and the SDK board whose include dir backs each pass.
const ARCH_PASSES: &[(&str, &str)] = &[
    ("aarch64", "qemu_virt_aarch64"),
    ("riscv64", "qemu_virt_riscv64"),
    ("x86_64", "x86_64_generic"),
];

/// Shared userspace library crates linted per-arch (no board features).
const SHARED_CRATES: &[&str] = &[
    "lerux-serial-queue",
    "lerux-service-async",
    "lerux-fat",
    "lerux-driver-protocols",
];

struct ClippyEntry {
    crate_name: String,
    feature: Option<String>,
}

fn derive_entries(root: &Path, boards: &Boards, arch: &str) -> Vec<ClippyEntry> {
    let mut seen: BTreeSet<(String, Option<String>)> = BTreeSet::new();
    let mut entries = Vec::new();
    for (board_name, board) in boards.iter().filter(|(_, b)| b.arch == arch) {
        for pd in &board.pds {
            let feature = crate_has_board_feature(root, pd, board_name)
                .then(|| format!("board-{board_name}"));
            if seen.insert((pd.clone(), feature.clone())) {
                entries.push(ClippyEntry {
                    crate_name: pd.clone(),
                    feature,
                });
            }
        }
    }
    if arch == "aarch64" {
        for name in SHARED_CRATES {
            entries.push(ClippyEntry {
                crate_name: (*name).to_string(),
                feature: None,
            });
        }
    }
    entries
}

pub fn clippy_workspace(root: &Path, build_dir: &str, config: &str) -> Result<()> {
    let sdk = sdk_path(root)?;
    apply_libclang_env(root);
    let boards = load_boards(root)?;

    for (arch, sdk_board) in ARCH_PASSES {
        let target = boards
            .values()
            .find(|b| b.arch == *arch)
            .map(|b| b.target.clone())
            .with_context(|| format!("no boards with arch {arch}"))?;
        let entries = derive_entries(root, &boards, arch);
        eprintln!("==> clippy ({arch}, {} crate combos)", entries.len());
        clippy_arch(root, build_dir, config, &sdk, sdk_board, &target, &entries)?;
    }

    Ok(())
}

fn clippy_arch(
    root: &Path,
    build_dir: &str,
    config: &str,
    sdk: &str,
    sdk_board: &str,
    target: &str,
    entries: &[ClippyEntry],
) -> Result<()> {
    let target_spec = root.join("support/targets").join(format!("{target}.json"));
    let target_dir = shared_target_dir(root, build_dir);
    ensure_dir(&target_dir)?;

    let include = format!("{sdk}/board/{sdk_board}/{config}/include");

    for entry in entries {
        let mut cmd = Command::new("cargo");
        cmd.current_dir(root);
        cmd.arg("clippy")
            .arg("-p")
            .arg(&entry.crate_name)
            .arg("--target-dir")
            .arg(path_str(&target_dir))
            .arg("--target")
            .arg(path_str(&target_spec))
            .args([
                "-Z",
                "json-target-spec",
                "-Z",
                "build-std=core,alloc,compiler_builtins",
                "-Z",
                "build-std-features=compiler-builtins-mem",
            ]);

        if let Some(feature) = &entry.feature {
            cmd.arg("--features").arg(feature);
        }

        cmd.args(["--", "-D", "warnings"]);

        cmd.env("SEL4_INCLUDE_DIRS", &include);
        cmd.env("RUST_TARGET_PATH", root.join("support/targets"));
        cmd.env("RUSTC_BOOTSTRAP", "1");

        let status = cmd
            .status()
            .with_context(|| format!("cargo clippy -p {}", entry.crate_name))?;
        if !status.success() {
            bail!(
                "cargo clippy -p {}{} failed",
                entry.crate_name,
                entry
                    .feature
                    .as_deref()
                    .map(|f| format!(" --features {f}"))
                    .unwrap_or_default()
            );
        }
    }

    Ok(())
}
