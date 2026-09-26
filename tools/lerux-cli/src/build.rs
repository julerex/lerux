use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};

use crate::{
    board::{crate_has_board_feature, get_board, load_boards},
    build_sdk::sdk_path,
    libclang::apply_libclang_env,
    process::{ensure_dir, path_str, run_inherit},
    system::{board_build_dir, generate_system, shared_target_dir, system_file},
};

pub fn system(root: &Path, board: &str, build_dir: &str) -> Result<()> {
    let out = system_file(root, board, build_dir);
    generate_system(root, board, &out)
}

pub fn build(root: &Path, board: &str, build_dir: &str, config: &str) -> Result<()> {
    system(root, board, build_dir)?;
    let boards = load_boards(root)?;
    let board_cfg = get_board(&boards, board)?;
    for crate_name in &board_cfg.pds {
        build_pd(root, board, build_dir, config, crate_name)?;
    }
    Ok(())
}

pub fn build_pd(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
    crate_name: &str,
) -> Result<()> {
    let boards = load_boards(root)?;
    let board_cfg = get_board(&boards, board)?;
    let sdk = sdk_path(root)?;
    let target_spec = root
        .join("support/targets")
        .join(format!("{}.json", board_cfg.target));
    let board_build = board_build_dir(root, board, build_dir);
    let target_dir = shared_target_dir(root, build_dir);

    apply_libclang_env(root);
    ensure_dir(&board_build)?;
    ensure_dir(&target_dir)?;

    let mut cmd = Command::new("cargo");
    cmd.current_dir(root);
    cmd.arg("build").arg("--release").arg("-p").arg(crate_name);
    if crate_has_board_feature(root, crate_name, board) {
        cmd.arg("--features").arg(format!("board-{board}"));
    }
    cmd.args([
        "--target-dir",
        &path_str(&target_dir),
        "--target",
        &path_str(&target_spec),
        "-Z",
        "json-target-spec",
        "-Z",
        "build-std=core,alloc,compiler_builtins",
        "-Z",
        "build-std-features=compiler-builtins-mem",
    ]);

    let include = format!(
        "{}/board/{}/{}/include",
        sdk, board_cfg.microkit_board, config
    );
    cmd.env("SEL4_INCLUDE_DIRS", include);
    cmd.env("RUST_TARGET_PATH", root.join("support/targets"));
    cmd.env("RUSTC_BOOTSTRAP", "1");

    let status = cmd.status().context("cargo build pd")?;
    if !status.success() {
        bail!("cargo build -p {crate_name} failed");
    }

    let elf_src = target_dir
        .join(&board_cfg.target)
        .join("release")
        .join(format!("{crate_name}.elf"));
    let elf_dst = board_build.join(format!("{crate_name}.elf"));
    std::fs::copy(&elf_src, &elf_dst)
        .with_context(|| format!("copy {} to {}", elf_src.display(), elf_dst.display()))?;
    Ok(())
}

pub fn image(root: &Path, board: &str, build_dir: &str, config: &str) -> Result<()> {
    build(root, board, build_dir, config)?;
    let boards = load_boards(root)?;
    let board_cfg = get_board(&boards, board)?;
    let sdk = sdk_path(root)?;
    let board_build = board_build_dir(root, board, build_dir);
    let system = system_file(root, board, build_dir);
    let microkit = format!("{}/bin/microkit", sdk);

    let loader = board_build.join("loader.img");
    run_inherit(
        &microkit,
        &[
            &path_str(&system),
            "--search-path",
            &path_str(&board_build),
            "--board",
            &board_cfg.microkit_board,
            "--config",
            config,
            "-r",
            &path_str(&board_build.join("report.txt")),
            "-o",
            &path_str(&loader),
        ],
    )?;
    // Phase 60 Track C: host-side integrity sidecar next to loader.img.
    crate::image_digest::write_sidecar(&loader)?;
    if board_cfg.arch == "x86_64" {
        stage_x86_kernel(&sdk, &board_cfg.microkit_board, config, &board_build)?;
    }
    Ok(())
}

/// Copy the SDK 32-bit kernel next to `loader.img` so `lerux deploy` can put
/// both Multiboot 2 files on USB/ESP media.
pub fn stage_x86_kernel(
    sdk: &str,
    microkit_board: &str,
    config: &str,
    dest_dir: &Path,
) -> Result<PathBuf> {
    let src = crate::qemu::sel4_32_elf(sdk, microkit_board, config);
    if !src.is_file() {
        bail!(
            "missing {}; run MICROKIT_BOARDS={} lerux build-sdk",
            src.display(),
            microkit_board
        );
    }
    fs::create_dir_all(dest_dir).with_context(|| format!("create {}", dest_dir.display()))?;
    let dest = dest_dir.join("sel4_32.elf");
    fs::copy(&src, &dest)
        .with_context(|| format!("copy {} → {}", src.display(), dest.display()))?;
    crate::image_digest::write_sidecar(&dest)?;
    Ok(dest)
}

pub fn run(root: &Path, board: &str, build_dir: &str, config: &str) -> Result<()> {
    image(root, board, build_dir, config)?;
    let ctx = crate::qemu::load_qemu_context(root, board, build_dir, config)?;
    if crate::qemu::is_hardware_board(&ctx) {
        println!("{}", hardware_ready_message(board, &ctx.board.arch));
        return Ok(());
    }
    if crate::qemu::is_http_board(&ctx.board) {
        crate::qemu::cleanup_http_conflicts();
        crate::qemu::print_http_hint(&ctx);
    }
    let mut ctx = ctx;
    ctx.graphic = true;
    crate::qemu::print_graphic_hint(&ctx);
    let helpers = crate::qemu::setup_test_helpers(&ctx)?;
    let mut cmd = crate::qemu::qemu_command(&ctx)?;
    let status = cmd.status().context("qemu run")?;
    for mut child in helpers {
        let _ = child.kill();
    }
    if !status.success() {
        bail!("qemu exited with {}", status);
    }
    Ok(())
}

/// Run every board with `ci = true` in boards.toml: diskless boards first,
/// then the disk image is created once and the disk boards follow.
pub fn test_all(root: &Path, build_dir: &str, config: &str) -> Result<()> {
    let boards = load_boards(root)?;
    let ci_boards: Vec<(&String, &crate::board::Board)> =
        boards.iter().filter(|(_, b)| b.ci).collect();

    for (board, _) in ci_boards.iter().filter(|(_, b)| !b.needs_disk()) {
        image(root, board, build_dir, config)?;
        crate::test::run_board_test(root, board, build_dir, config)?;
    }
    crate::disk_img::disk_img(root)?;
    for (board, _) in ci_boards.iter().filter(|(_, b)| b.needs_disk()) {
        image(root, board, build_dir, config)?;
        crate::test::run_board_test(root, board, build_dir, config)?;
    }
    Ok(())
}

fn hardware_ready_message(board: &str, arch: &str) -> String {
    let docs = if arch == "x86_64" {
        "docs/boards.md#gigabyte-z97-d3h-install-path"
    } else {
        "docs/boards.md#rpi4-workstation-install-path-phase-52"
    };
    let iso = if arch == "x86_64" {
        format!("\n   ISO: just iso  (build/{board}/lerux.iso)")
    } else {
        String::new()
    };
    format!(
        "==> Hardware board {board:?}: image ready.{iso}\n\
         \x20   Deploy: lerux deploy --board {board} --dest /abs/path/to/boot\n\
         \x20   Boot smoke: LERUX_HW_SERIAL=/dev/ttyUSB0 BOARD={board} just test-hw\n\
         \x20   Docs: {docs}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_x86_kernel_copies_sdk_elf_and_sidecar() {
        let tmp = tempfile::tempdir().unwrap();
        let sdk = tmp.path().join("sdk");
        let elf_dir = sdk.join("board/x86_64_generic/debug/elf");
        fs::create_dir_all(&elf_dir).unwrap();
        fs::write(elf_dir.join("sel4_32.elf"), b"kernel-bytes").unwrap();
        let dest_dir = tmp.path().join("build/pc_z97_d3h");

        let dest =
            stage_x86_kernel(sdk.to_str().unwrap(), "x86_64_generic", "debug", &dest_dir).unwrap();

        assert_eq!(dest, dest_dir.join("sel4_32.elf"));
        assert_eq!(fs::read(&dest).unwrap(), b"kernel-bytes");
        assert!(crate::image_digest::sidecar_path(&dest).is_file());
    }

    #[test]
    fn hardware_ready_message_points_at_z97_docs() {
        let msg = hardware_ready_message("pc_z97_d3h", "x86_64");
        assert!(msg.contains("lerux deploy --board pc_z97_d3h"), "{msg}");
        assert!(msg.contains("just iso"), "{msg}");
        assert!(msg.contains("gigabyte-z97-d3h-install-path"), "{msg}");
        assert!(!msg.contains("deploy-rpi4"), "{msg}");
        let rpi = hardware_ready_message("rpi4b_4gb_workstation", "aarch64");
        assert!(!rpi.contains("just iso"), "{rpi}");
    }
}
