//! QEMU launch derived from `support/boards.toml` (`[board].qemu` table).
//!
//! The board entry is the whole interface: arch picks the base machine,
//! `disk`/`net`/`sp804`/`tcp_echo`/`http_one`/`https_one`/`grok_one` pick devices and host helpers.

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
    /// Phase 70: `-s` gdbstub (also `LERUX_QEMU_GDB=1`).
    pub gdb: bool,
    /// Phase 70: QEMU `-snapshot` overlay (also `LERUX_QEMU_SNAPSHOT=1`).
    pub snapshot: bool,
    /// Phase 72: `lerux run` may open a GTK window when ramfb is on.
    /// Smokes keep this false so CI stays headless (`-nographic`).
    pub graphic: bool,
}

const HOSTFWD: &str = "user,id=netdev0,hostfwd=tcp::18080-:8080";

/// Microkit `x86_64_generic` 32-bit kernel ELF (Multiboot / QEMU `-kernel`).
pub fn sel4_32_elf(sdk: impl AsRef<Path>, microkit_board: &str, config: &str) -> PathBuf {
    sdk.as_ref()
        .join("board")
        .join(microkit_board)
        .join(config)
        .join("elf/sel4_32.elf")
}

pub fn qemu_command(ctx: &QemuContext) -> Result<Command> {
    let board_build = board_build_dir(&ctx.root, &ctx.board_name, &ctx.build_dir);
    let loader = board_build.join("loader.img");
    let disk = ctx.root.join("support/disk.img");

    let Some(qemu) = ctx.board.qemu() else {
        bail!(
            "board {:?} is hardware-only (no QEMU profile); run `lerux image --board {}` then `lerux deploy --dest …`",
            ctx.board_name, ctx.board_name
        );
    };

    let window = want_graphic_window(qemu, ctx.graphic);
    let mut path = host_path(&ctx.root);
    if qemu.sp804 {
        let sp804 = install_sp804_qemu(&ctx.root, window)?;
        path = format!("{}:{}", sp804.display(), path);
    }

    if qemu.disk != DiskMode::None {
        ensure_disk(&disk)?;
    }

    let mut cmd = match ctx.board.arch.as_str() {
        "aarch64" => aarch64_command(qemu, &loader, &disk, window),
        "riscv64" => riscv64_command(qemu, &loader, &disk),
        "x86_64" => x86_command(ctx, qemu, &loader, &disk)?,
        other => bail!("unsupported arch {other}"),
    };

    apply_dev_flags(&mut cmd, ctx);
    cmd.env("PATH", path);
    cmd.stdin(std::process::Stdio::inherit());
    Ok(cmd)
}

fn env_flag(name: &str) -> bool {
    matches!(std::env::var(name).as_deref(), Ok("1" | "true" | "yes"))
}

fn apply_dev_flags(cmd: &mut Command, ctx: &QemuContext) {
    if ctx.gdb || env_flag("LERUX_QEMU_GDB") {
        cmd.arg("-s");
    }
    if ctx.snapshot || env_flag("LERUX_QEMU_SNAPSHOT") {
        cmd.arg("-snapshot");
    }
    if let Ok(dir) = std::env::var("LERUX_FS_HOST")
        && !dir.is_empty()
    {
        cmd.args([
            "-virtfs",
            &format!("local,path={dir},mount_tag=host,security_model=mapped-xattr,id=fsdev0"),
        ]);
    }
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

fn want_graphic_window(qemu: &QemuConfig, graphic: bool) -> bool {
    if !graphic || !qemu.ramfb {
        return false;
    }
    match std::env::var("LERUX_QEMU_GRAPHIC").as_deref() {
        Ok("0") | Ok("false") | Ok("no") => false,
        Ok("1") | Ok("true") | Ok("yes") => true,
        _ => std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok(),
    }
}

fn aarch64_command(qemu: &QemuConfig, loader: &Path, disk: &Path, window: bool) -> Command {
    let mut c = Command::new("qemu-system-aarch64");
    c.args([
        "-machine",
        "virt,virtualization=on",
        "-cpu",
        "cortex-a53",
        "-m",
        "size=2G",
    ]);
    if window {
        // Human `lerux run`: GTK window + ramfb. Serial is a QEMU console
        // (View → serial0 / Ctrl-Alt-2), not the calling terminal.
        c.args(["-display", "gtk", "-serial", "vc"]);
    } else {
        c.args(["-serial", "mon:stdio", "-nographic"]);
    }
    if qemu.ramfb {
        c.args(["-device", "ramfb"]);
    }
    c.args([
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
    let kernel = sel4_32_elf(&sdk, &ctx.board.microkit_board, &ctx.config);
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

pub fn setup_test_helpers(ctx: &QemuContext) -> Result<Vec<std::process::Child>> {
    let mut helpers = Vec::new();
    let Some(qemu) = ctx.board.qemu() else {
        return Ok(helpers);
    };
    if qemu.http_one {
        helpers.push(crate::http_one::start_http_one_background(8081)?);
    }
    if qemu.https_one {
        helpers.push(crate::https_one::start_https_one_background(8443)?);
    }
    if qemu.grok_one {
        helpers.push(crate::grok_one::start_grok_one_background(8444)?);
    }
    if qemu.tcp_echo {
        helpers.push(start_tcp_echo_background(18080)?);
    }
    Ok(helpers)
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
        gdb: false,
        snapshot: false,
        graphic: false,
    })
}

pub fn print_http_hint(ctx: &QemuContext) {
    if ctx.board.qemu().is_some_and(|q| q.net == NetMode::Hostfwd) {
        eprintln!("Guest listens on :8080; hostfwd maps 127.0.0.1:18080. In another terminal:");
        eprintln!("  curl http://127.0.0.1:18080/");
    }
}

pub fn print_graphic_hint(ctx: &QemuContext) {
    let Some(qemu) = ctx.board.qemu() else {
        return;
    };
    if !want_graphic_window(qemu, ctx.graphic) {
        return;
    }
    eprintln!("QEMU opens a GTK window (ramfb). Close the window to quit.");
    eprintln!("Serial shell is in the QEMU window: View → serial0, or Ctrl-Alt-2.");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn render_aarch64(qemu_toml: &str, window: bool) -> String {
        let qemu: QemuConfig = toml::from_str(qemu_toml).unwrap();
        let cmd = aarch64_command(
            &qemu,
            Path::new("loader.img"),
            Path::new("disk.img"),
            window,
        );
        format!("{cmd:?}")
    }

    #[test]
    fn graphic_ramfb_should_not_attach_serial_to_stdio() {
        let line = render_aarch64("ramfb = true", true);
        assert!(
            !line.contains("mon:stdio"),
            "graphic ramfb must not wire serial through the calling terminal: {line}"
        );
    }

    #[test]
    fn graphic_ramfb_should_put_serial_on_qemu_vc() {
        let line = render_aarch64("ramfb = true", true);
        assert!(
            line.contains("\"-serial\" \"vc\""),
            "graphic ramfb serial should live in the QEMU window: {line}"
        );
    }

    #[test]
    fn headless_ramfb_should_keep_stdio_serial() {
        let line = render_aarch64("ramfb = true", false);
        assert!(
            line.contains("\"-serial\" \"mon:stdio\""),
            "smokes must keep serial on the calling terminal: {line}"
        );
        assert!(
            line.contains("\"-nographic\""),
            "smokes must stay headless: {line}"
        );
    }
}
