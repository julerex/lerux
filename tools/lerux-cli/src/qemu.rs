//! QEMU launch derived from `support/boards.toml` (`[board].qemu` table).
//!
//! The board entry is the whole interface: arch picks the base machine,
//! `disk`/`net`/`sp804`/`tcp_echo`/`http_one` pick devices and host helpers.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Result};

use crate::{
    board::{get_board, load_boards, Board, DiskMode, NetMode, QemuConfig},
    build_sdk::sdk_path,
    install::install_sp804_qemu,
    path::host_path,
    process::{command_on_path, path_str},
    system::board_build_dir,
    tcp_echo::{port_is_listening, start_tcp_echo_background},
};

pub struct QemuContext {
    pub root: PathBuf,
    pub board_name: String,
    pub board: Board,
    pub build_dir: String,
    pub config: String,
}

const HOSTFWD: &str = "user,id=netdev0,hostfwd=tcp::18080-:8080";

pub fn qemu_command(ctx: &QemuContext) -> Result<Command> {
    let board_build = board_build_dir(&ctx.root, &ctx.board_name, &ctx.build_dir);
    let loader = board_build.join("loader.img");
    let disk = ctx.root.join("support/disk.img");

    let Some(qemu) = ctx.board.qemu() else {
        bail!(
            "board {:?} is hardware-only (no QEMU profile); run `lerux image --board {}` then deploy loader.img manually (e.g. via U-Boot)",
            ctx.board_name, ctx.board_name
        );
    };

    let mut path = host_path(&ctx.root);
    if qemu.sp804 {
        let sp804 = install_sp804_qemu(&ctx.root)?;
        path = format!("{}:{}", sp804.display(), path);
    }

    if qemu.disk != DiskMode::None {
        ensure_disk(&disk)?;
    }

    let mut cmd = match ctx.board.arch.as_str() {
        "aarch64" => aarch64_command(qemu, &loader, &disk),
        "riscv64" => riscv64_command(qemu, &loader, &disk),
        "x86_64" => x86_command(ctx, qemu, &loader, &disk)?,
        other => bail!("unsupported arch {other}"),
    };

    cmd.env("PATH", path);
    cmd.stdin(std::process::Stdio::inherit());
    Ok(cmd)
}

fn netdev_arg(net: NetMode) -> Option<&'static str> {
    match net {
        NetMode::None => None,
        NetMode::User => Some("user,id=netdev0"),
        NetMode::Hostfwd => Some(HOSTFWD),
    }
}

fn blockdev_arg(disk: DiskMode, disk_path: &Path) -> Option<String> {
    let read_only = match disk {
        DiskMode::None => return None,
        DiskMode::Ro => "on",
        DiskMode::Rw => "off",
    };
    Some(format!(
        "node-name=blkdev0,read-only={read_only},driver=file,filename={}",
        path_str(disk_path)
    ))
}

fn aarch64_command(qemu: &QemuConfig, loader: &Path, disk: &Path) -> Command {
    let mut c = Command::new("qemu-system-aarch64");
    c.args([
        "-machine",
        "virt,virtualization=on",
        "-cpu",
        "cortex-a53",
        "-m",
        "size=2G",
        "-serial",
        "mon:stdio",
        "-nographic",
        "-device",
        &format!("loader,file={},addr=0x70000000,cpu-num=0", path_str(loader)),
    ]);
    // Virtio-mmio slot order matters: net first, blk at +0xc00 in the same page
    // (see virtio-blk-driver VIRTIO_BLK_MMIO_OFFSET).
    if let Some(netdev) = netdev_arg(qemu.net) {
        c.args([
            "-device",
            "virtio-net-device,netdev=netdev0",
            "-netdev",
            netdev,
        ]);
    }
    if let Some(blockdev) = blockdev_arg(qemu.disk, disk) {
        c.args([
            "-device",
            "virtio-blk-device,drive=blkdev0",
            "-blockdev",
            &blockdev,
        ]);
    }
    c
}

fn riscv64_command(qemu: &QemuConfig, loader: &Path, disk: &Path) -> Command {
    let mut c = Command::new("qemu-system-riscv64");
    c.args([
        "-machine",
        "virt",
        "-m",
        "size=2G",
        "-nographic",
        "-serial",
        "mon:stdio",
        "-kernel",
        &path_str(loader),
    ]);
    // Fixed virtio-mmio bus slots: blk on bus.0, net on bus.1 (match system_vars).
    if let Some(blockdev) = blockdev_arg(qemu.disk, disk) {
        c.args([
            "-device",
            "virtio-blk-device,bus=virtio-mmio-bus.0,drive=blkdev0",
            "-blockdev",
            &blockdev,
        ]);
    }
    if let Some(netdev) = netdev_arg(qemu.net) {
        c.args([
            "-device",
            "virtio-net-device,bus=virtio-mmio-bus.1,netdev=netdev0",
            "-netdev",
            netdev,
        ]);
    }
    c
}

fn x86_command(
    ctx: &QemuContext,
    qemu: &QemuConfig,
    loader: &Path,
    disk: &Path,
) -> Result<Command> {
    let sdk = sdk_path(&ctx.root)?;
    let kernel = PathBuf::from(&sdk)
        .join("board")
        .join(&ctx.board.microkit_board)
        .join(&ctx.config)
        .join("elf/sel4_32.elf");
    if !kernel.is_file() {
        bail!(
            "missing {}; run MICROKIT_BOARDS={} lerux build-sdk",
            kernel.display(),
            ctx.board.microkit_board
        );
    }

    let mut c = Command::new("qemu-system-x86_64");
    c.args([
        "-machine",
        "q35",
        "-cpu",
        "qemu64,+fsgsbase,+pdpe1gb,+xsaveopt,+xsave",
        "-m",
        "2G",
        "-display",
        "none",
        "-serial",
        "mon:stdio",
        "-kernel",
        &path_str(&kernel),
        "-initrd",
        &path_str(loader),
    ]);
    // Fixed PCI slots: blk at 0x3, net at 0x4 (match system_vars BAR addresses).
    if let Some(blockdev) = blockdev_arg(qemu.disk, disk) {
        c.args([
            "-device",
            "virtio-blk-pci,id=blk0,addr=0x3.0x0,drive=blkdev0",
            "-blockdev",
            &blockdev,
        ]);
    }
    if let Some(netdev) = netdev_arg(qemu.net) {
        c.args([
            "-device",
            "virtio-net-pci,id=net0,addr=0x4.0x0,netdev=netdev0",
            "-netdev",
            netdev,
        ]);
    }
    Ok(c)
}

fn ensure_disk(disk: &Path) -> Result<()> {
    if disk.is_file() {
        return Ok(());
    }
    bail!("missing {}; run `lerux disk-img`", disk.display());
}

pub fn setup_test_helpers(ctx: &QemuContext) -> Result<Option<std::process::Child>> {
    let Some(qemu) = ctx.board.qemu() else {
        return Ok(None);
    };
    if qemu.http_one {
        return Ok(Some(crate::http_one::start_http_one_background(8081)?));
    }
    if qemu.tcp_echo {
        return Ok(Some(start_tcp_echo_background(18080)?));
    }
    Ok(None)
}

pub fn cleanup_http_conflicts() {
    let _ = Command::new("pkill")
        .args(["-f", "tcp-echo 18080"])
        .status();
    for pattern in [
        "qemu-system-x86_64.*hostfwd=tcp::18080-:8080",
        "qemu-system-aarch64.*hostfwd=tcp::18080-:8080",
        "qemu-system-riscv64.*hostfwd=tcp::18080-:8080",
    ] {
        let _ = Command::new("pkill").args(["-f", pattern]).status();
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
}

/// Boards whose smoke includes a host curl of the hostfwd port.
pub fn is_http_board(board: &Board) -> bool {
    board.curl_expect.is_some()
}

pub fn is_hardware_board(ctx: &QemuContext) -> bool {
    ctx.board.qemu.is_none()
}

pub fn load_qemu_context(
    root: &Path,
    board_name: &str,
    build_dir: &str,
    config: &str,
) -> Result<QemuContext> {
    let boards = load_boards(root)?;
    let board = get_board(&boards, board_name)?.clone();
    Ok(QemuContext {
        root: root.to_path_buf(),
        board_name: board_name.to_string(),
        board,
        build_dir: build_dir.to_string(),
        config: config.to_string(),
    })
}

pub fn print_http_hint(ctx: &QemuContext) {
    if ctx.board.qemu().is_some_and(|q| q.net == NetMode::Hostfwd) {
        eprintln!("Guest listens on :8080; hostfwd maps 127.0.0.1:18080. In another terminal:");
        eprintln!("  curl http://127.0.0.1:18080/");
    }
}

pub fn ensure_qemu_binary(root: &Path, board: &Board) -> Result<()> {
    let binary = format!("qemu-system-{}", board.arch);
    let path = host_path(root);
    // SAFETY: host build tooling mutates the current process environment only.
    unsafe {
        std::env::set_var("PATH", &path);
    }
    if !command_on_path(&binary) {
        bail!("{binary} not found in PATH");
    }
    Ok(())
}

#[allow(dead_code)]
pub fn probe_tcp_echo(port: u16) -> bool {
    port_is_listening(port)
}
