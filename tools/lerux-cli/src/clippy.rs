//! Cross-target clippy derived from `support/boards.toml`.
//!
//! Every (PD crate, board) pair in the matrix is linted with the same
//! `board-<name>` feature the build uses, so lint coverage cannot drift from
//! the board matrix. Shared userspace crates without board features are
//! linted once per arch.

use std::{
    collections::{BTreeSet, HashSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

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
    "lerux-sddf",
    "lerux-machine",
    "lerux-cothread",
    "lerux-gate",
    "lerux-posix",
    "lerux-driver-protocols",
    "lerux-tls",
    "lerux-html",
    "lerux-input",
    "lerux-desk",
    "lerux-web",
    "lerux-wasm",
    "lerux-rtw8852be",
    "lerux-logging",
    "lerux-virtio-hal",
    "lerux-virtio-pci",
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

        run_clippy_once(root, &target_dir, &mut cmd, entry)?;
    }

    Ok(())
}

fn run_clippy_once(
    root: &Path,
    target_dir: &Path,
    cmd: &mut Command,
    entry: &ClippyEntry,
) -> Result<()> {
    let before = ice_files(root);
    for attempt in 0..2 {
        let output = cmd
            .output()
            .with_context(|| format!("cargo clippy -p {}", entry.crate_name))?;
        let _ = std::io::stdout().write_all(&output.stdout);
        let _ = std::io::stderr().write_all(&output.stderr);
        let fresh = new_ice_files(root, &before);
        if fresh.is_empty() {
            if output.status.success() {
                return Ok(());
            }
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
        for path in &fresh {
            let _ = fs::remove_file(path);
        }
        if attempt == 0 {
            eprintln!(
                "==> compiler ice in {}; clearing cross incremental caches and retrying once",
                entry.crate_name
            );
            clear_cross_incremental(target_dir)?;
            continue;
        }
        let names = fresh
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "cargo clippy -p {} wrote an ice file after one retry: {names}",
            entry.crate_name
        );
    }
    Ok(())
}

fn ice_files(root: &Path) -> HashSet<PathBuf> {
    let mut found = HashSet::new();
    let Ok(entries) = fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("rustc-ice-") && name.ends_with(".txt") {
            found.insert(entry.path());
        }
    }
    found
}

fn new_ice_files(root: &Path, before: &HashSet<PathBuf>) -> Vec<PathBuf> {
    let mut fresh: Vec<PathBuf> = ice_files(root).difference(before).cloned().collect();
    fresh.sort();
    fresh
}

fn clear_cross_incremental(target_dir: &Path) -> Result<()> {
    let Ok(entries) = fs::read_dir(target_dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.contains("sel4-microkit") {
            continue;
        }
        let incremental = entry.path().join("debug").join("incremental");
        if incremental.is_dir() {
            fs::remove_dir_all(&incremental)
                .with_context(|| format!("remove incremental cache {}", incremental.display()))?;
        }
    }
    Ok(())
}
