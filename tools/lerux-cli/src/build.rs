use std::{path::Path, process::Command};

use anyhow::{bail, Context, Result};

use crate::{
    board::{get_board, load_boards},
    build_sdk::sdk_path,
    libclang::apply_libclang_env,
    process::{ensure_dir, path_str, run_inherit},
    system::{board_build_dir, generate_system, shared_target_dir, system_file},
};

const BOARD_FEATURE_CRATES: &[&str] = &[
    "hello",
    "echo-server",
    "echo-client",
    "http-server",
    "supervisor",
    "blk-server",
    "blk-client",
    "net-server",
    "net-client",
    "fetch-client",
    "fs-server",
    "fs-client",
    "shell",
    "edit",
    "chat-client",
    "http-file-browser",
    "backup",
    "log-server",
    "config-server",
    "serial-driver",
    "serial-virt",
    "debug-handler",
    "crash-demo",
    "virtio-blk-driver",
    "virtio-net-driver",
    "virtio-pci-driver",
    "genet-driver",
    "emmc2-driver",
];

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
        .join(format!("{}.json", board_cfg.target_triple));
    let board_build = board_build_dir(root, board, build_dir);
    let target_dir = shared_target_dir(root, build_dir);

    apply_libclang_env(root);
    ensure_dir(&board_build)?;
    ensure_dir(&target_dir)?;

    let mut cmd = Command::new("cargo");
    cmd.current_dir(root);
    cmd.arg("build").arg("--release").arg("-p").arg(crate_name);
    if BOARD_FEATURE_CRATES.contains(&crate_name) {
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
        .join(&board_cfg.target_triple)
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
            &path_str(&board_build.join("loader.img")),
        ],
    )
}

pub fn run(root: &Path, board: &str, build_dir: &str, config: &str) -> Result<()> {
    image(root, board, build_dir, config)?;
    let ctx = crate::qemu::load_qemu_context(root, board, build_dir, config)?;
    if crate::qemu::is_hardware_board(&ctx) {
        println!(
            "==> Hardware board {board:?}: image ready.\n\
             \x20   Deploy: just deploy-rpi4 DEST=/path/to/sd-boot   # or: lerux deploy --dest …\n\
             \x20   Boot smoke: LERUX_HW_SERIAL=/dev/ttyUSB0 BOARD={board} just test-hw\n\
             \x20   Docs: docs/boards.md#rpi4-workstation-install-path-phase-52"
        );
        return Ok(());
    }
    if crate::qemu::is_http_board(board) {
        crate::qemu::cleanup_http_conflicts();
        crate::qemu::print_http_hint(&ctx);
    }
    let helper = crate::qemu::setup_test_helpers(&ctx)?;
    let mut cmd = crate::qemu::qemu_command(&ctx)?;
    let status = cmd.status().context("qemu run")?;
    if let Some(mut child) = helper {
        let _ = child.kill();
    }
    if !status.success() {
        bail!("qemu exited with {}", status);
    }
    Ok(())
}

pub fn test_all(root: &Path, build_dir: &str, config: &str) -> Result<()> {
    let tests_before_disk = [
        "qemu_virt_aarch64",
        "x86_64_generic",
        "qemu_virt_riscv64",
        "qemu_virt_riscv64_echo",
        "qemu_virt_aarch64_virtio",
        "qemu_virt_riscv64_virtio",
        "qemu_virt_aarch64_echo",
        "x86_64_generic_echo",
        "x86_64_generic_virtio",
        "x86_64_generic_http",
        "qemu_virt_riscv64_http",
        "qemu_virt_aarch64_init",
        "qemu_virt_riscv64_init",
        "x86_64_generic_init",
        "qemu_virt_aarch64_debug",
    ];
    let tests_after_disk = [
        "qemu_virt_aarch64_blk",
        "qemu_virt_riscv64_blk",
        "x86_64_generic_blk",
        "qemu_virt_aarch64_composed",
        "qemu_virt_aarch64_blk_composed",
        "qemu_virt_aarch64_http",
        "qemu_virt_aarch64_http_composed",
        "qemu_virt_aarch64_net",
        "qemu_virt_aarch64_fetch",
        "qemu_virt_aarch64_fs",
        "qemu_virt_aarch64_fs_fat",
        "qemu_virt_aarch64_net_composed",
        "qemu_virt_aarch64_ipc_composed",
        "qemu_virt_aarch64_workstation",
        "qemu_virt_riscv64_net",
        "x86_64_generic_net",
    ];

    for board in tests_before_disk {
        image(root, board, build_dir, config)?;
        crate::test::run_board_test(root, board, build_dir, config)?;
    }
    crate::disk_img::disk_img(root)?;
    for board in tests_after_disk {
        image(root, board, build_dir, config)?;
        crate::test::run_board_test(root, board, build_dir, config)?;
    }
    Ok(())
}
