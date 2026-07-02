use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Result};

use crate::{
    board::{get_board, load_boards, Board},
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

pub fn qemu_command(ctx: &QemuContext) -> Result<Command> {
    let board_build = board_build_dir(&ctx.root, &ctx.board_name, &ctx.build_dir);
    let loader = board_build.join("loader.img");
    let disk = ctx.root.join("support/disk.img");

    let mut path = host_path(&ctx.root);
    if needs_sp804(&ctx.board.qemu) {
        let sp804 = install_sp804_qemu(&ctx.root)?;
        path = format!("{}:{}", sp804.display(), path);
    }

    let mut cmd = match ctx.board.qemu.as_str() {
        "aarch64" => {
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
                &format!(
                    "loader,file={},addr=0x70000000,cpu-num=0",
                    path_str(&loader)
                ),
            ]);
            c
        }
        "aarch64_init"
        | "aarch64_virtio"
        | "aarch64_blk"
        | "aarch64_blk_composed"
        | "aarch64_composed"
        | "aarch64_http"
        | "aarch64_http_composed"
        | "aarch64_ipc_composed"
        | "aarch64_net"
        | "aarch64_fetch"
        | "aarch64_net_composed" => {
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
                &format!(
                    "loader,file={},addr=0x70000000,cpu-num=0",
                    path_str(&loader)
                ),
            ]);
            if matches!(
                ctx.board.qemu.as_str(),
                "aarch64_virtio" | "aarch64_composed"
            ) {
                ensure_disk(&disk)?;
                c.args([
                    "-device",
                    "virtio-net-device,netdev=netdev0",
                    "-netdev",
                    "user,id=netdev0",
                    "-device",
                    "virtio-blk-device,drive=blkdev0",
                    "-blockdev",
                    &format!(
                        "node-name=blkdev0,read-only=on,driver=file,filename={}",
                        path_str(&disk)
                    ),
                ]);
            } else if matches!(
                ctx.board.qemu.as_str(),
                "aarch64_blk" | "aarch64_blk_composed" | "aarch64_ipc_composed"
            ) {
                ensure_disk(&disk)?;
                // Net device occupies the first virtio-mmio slot; blk stays at +0xc00
                // in the same page (see virtio-blk-driver VIRTIO_BLK_MMIO_OFFSET).
                c.args([
                    "-device",
                    "virtio-net-device,netdev=netdev0",
                    "-netdev",
                    "user,id=netdev0",
                    "-device",
                    "virtio-blk-device,drive=blkdev0",
                    "-blockdev",
                    &format!(
                        "node-name=blkdev0,read-only=off,driver=file,filename={}",
                        path_str(&disk)
                    ),
                ]);
            } else if matches!(
                ctx.board.qemu.as_str(),
                "aarch64_net" | "aarch64_fetch" | "aarch64_net_composed"
            ) {
                c.args([
                    "-device",
                    "virtio-net-device,netdev=netdev0",
                    "-netdev",
                    "user,id=netdev0",
                ]);
            } else if matches!(
                ctx.board.qemu.as_str(),
                "aarch64_http" | "aarch64_http_composed"
            ) {
                c.args([
                    "-device",
                    "virtio-net-device,netdev=netdev0",
                    "-netdev",
                    "user,id=netdev0,hostfwd=tcp::18080-:8080",
                ]);
            }
            c
        }
        "riscv64" => {
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
                &path_str(&loader),
            ]);
            c
        }
        "riscv64_virtio" => {
            ensure_disk(&disk)?;
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
                &path_str(&loader),
                "-device",
                "virtio-blk-device,bus=virtio-mmio-bus.0,drive=blkdev0",
                "-blockdev",
                &format!(
                    "node-name=blkdev0,read-only=on,driver=file,filename={}",
                    path_str(&disk)
                ),
                "-device",
                "virtio-net-device,bus=virtio-mmio-bus.1,netdev=netdev0",
                "-netdev",
                "user,id=netdev0",
            ]);
            c
        }
        "riscv64_blk" => {
            ensure_disk(&disk)?;
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
                &path_str(&loader),
                "-device",
                "virtio-blk-device,bus=virtio-mmio-bus.0,drive=blkdev0",
                "-blockdev",
                &format!(
                    "node-name=blkdev0,read-only=off,driver=file,filename={}",
                    path_str(&disk)
                ),
            ]);
            c
        }
        "riscv64_http" => {
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
                &path_str(&loader),
                "-device",
                "virtio-net-device,bus=virtio-mmio-bus.1,netdev=netdev0",
                "-netdev",
                "user,id=netdev0,hostfwd=tcp::18080-:8080",
            ]);
            c
        }
        "riscv64_net" => {
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
                &path_str(&loader),
                "-device",
                "virtio-net-device,bus=virtio-mmio-bus.1,netdev=netdev0",
                "-netdev",
                "user,id=netdev0",
            ]);
            c
        }
        "x86_64" | "x86_64_virtio" | "x86_64_blk" | "x86_64_http" | "x86_64_net" => {
            x86_command(ctx, &loader, &disk)?
        }
        other => bail!("unsupported qemu profile {other}"),
    };

    cmd.env("PATH", path);
    cmd.stdin(std::process::Stdio::inherit());
    Ok(cmd)
}

fn x86_command(ctx: &QemuContext, loader: &Path, disk: &Path) -> Result<Command> {
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

    match ctx.board.qemu.as_str() {
        "x86_64_virtio" => {
            ensure_disk(disk)?;
            c.args([
                "-device",
                "virtio-blk-pci,id=blk0,addr=0x3.0x0,drive=blkdev0",
                "-blockdev",
                &format!(
                    "node-name=blkdev0,read-only=on,driver=file,filename={}",
                    path_str(disk)
                ),
                "-device",
                "virtio-net-pci,id=net0,addr=0x4.0x0,netdev=netdev0",
                "-netdev",
                "user,id=netdev0",
            ]);
        }
        "x86_64_blk" => {
            ensure_disk(disk)?;
            c.args([
                "-device",
                "virtio-blk-pci,id=blk0,addr=0x3.0x0,drive=blkdev0",
                "-blockdev",
                &format!(
                    "node-name=blkdev0,read-only=off,driver=file,filename={}",
                    path_str(disk)
                ),
            ]);
        }
        "x86_64_http" => {
            c.args([
                "-device",
                "virtio-net-pci,id=net0,addr=0x4.0x0,netdev=netdev0",
                "-netdev",
                "user,id=netdev0,hostfwd=tcp::18080-:8080",
            ]);
        }
        "x86_64_net" => {
            c.args([
                "-device",
                "virtio-net-pci,id=net0,addr=0x4.0x0,netdev=netdev0",
                "-netdev",
                "user,id=netdev0",
            ]);
        }
        _ => {}
    }
    Ok(c)
}

fn ensure_disk(disk: &Path) -> Result<()> {
    if disk.is_file() {
        return Ok(());
    }
    bail!("missing {}; run `lerux disk-img`", disk.display());
}

fn needs_sp804(profile: &str) -> bool {
    matches!(
        profile,
        "aarch64_init"
            | "aarch64_composed"
            | "aarch64_blk_composed"
            | "aarch64_http_composed"
            | "aarch64_ipc_composed"
            | "aarch64_net_composed"
    )
}

pub fn is_fetch_board(board: &str) -> bool {
    board == "qemu_virt_aarch64_fetch"
}

pub fn setup_test_helpers(ctx: &QemuContext) -> Result<Option<std::process::Child>> {
    if is_fetch_board(&ctx.board_name) {
        return Ok(Some(crate::http_one::start_http_one_background(8081)?));
    }
    if matches!(
        ctx.board.qemu.as_str(),
        "aarch64_virtio" | "aarch64_composed" | "riscv64_virtio" | "x86_64_virtio"
    ) {
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

pub fn is_http_board(board: &str) -> bool {
    matches!(
        board,
        "qemu_virt_aarch64_http"
            | "qemu_virt_aarch64_http_composed"
            | "qemu_virt_riscv64_http"
            | "x86_64_generic_http"
    )
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
    if matches!(
        ctx.board.qemu.as_str(),
        "aarch64_http" | "aarch64_http_composed" | "riscv64_http" | "x86_64_http"
    ) {
        eprintln!("Guest listens on :8080; hostfwd maps 127.0.0.1:18080. In another terminal:");
        eprintln!("  curl http://127.0.0.1:18080/");
    }
}

pub fn ensure_qemu_binary(root: &Path, profile: &str) -> Result<()> {
    let binary = match profile {
        "riscv64" | "riscv64_virtio" | "riscv64_blk" | "riscv64_http" | "riscv64_net" => {
            "qemu-system-riscv64"
        }
        "x86_64" | "x86_64_virtio" | "x86_64_blk" | "x86_64_http" | "x86_64_net" => {
            "qemu-system-x86_64"
        }
        _ => "qemu-system-aarch64",
    };
    let path = host_path(root);
    // SAFETY: host build tooling mutates the current process environment only.
    unsafe {
        std::env::set_var("PATH", &path);
    }
    if !command_on_path(binary) {
        bail!("{binary} not found in PATH");
    }
    Ok(())
}

#[allow(dead_code)]
pub fn probe_tcp_echo(port: u16) -> bool {
    port_is_listening(port)
}
