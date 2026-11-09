//! QEMU launch derived from `support/boards.toml` (`[board].qemu` table).
//!
//! The board entry is the whole interface: arch picks the base machine,
//! `disk`/`net`/`sp804`/`tcp_echo`/`http_one`/`https_one`/`grok_one`/`gpu`/`dns_one`
//! pick devices and host helpers. `dns_one` points QEMU's open of
//! `/etc/resolv.conf` at a file that names the local helper. QEMU stays the
//! process the smoke waits on.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};
use lerux_image::{
    ahci_disk_device, e1000e_net_device, x86_disk_backend, x86_net_backend, DiskBackend, NetBackend,
};

use crate::{
    board::{get_board, load_boards, Board, DiskMode, NetMode, QemuConfig},
    install::install_sp804_qemu,
    path::host_path,
    process::{command_on_path, path_str},
    system::board_build_dir,
    tcp_echo::{port_is_listening, start_tcp_echo_background},
};

#[derive(Clone)]
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

/// Microkit `x86_64_generic` 64-bit kernel ELF (GRUB ISO via `mb2-stub`).
pub fn sel4_elf(sdk: impl AsRef<Path>, microkit_board: &str, config: &str) -> PathBuf {
    sdk.as_ref()
        .join("board")
        .join(microkit_board)
        .join(config)
        .join("elf/sel4.elf")
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

    if qemu.qmp {
        let sock = qmp_socket(ctx);
        if let Some(parent) = sock.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("mkdir {}", parent.display()))?;
        }
        let _ = std::fs::remove_file(&sock);
        cmd.args(["-qmp", &format!("unix:{},server,nowait", path_str(&sock))]);
    }

    apply_dev_flags(&mut cmd, ctx);
    cmd.env("PATH", path);
    cmd.stdin(std::process::Stdio::inherit());
    if qemu.dns_one {
        apply_dns_resolv(&ctx.root, &board_build, &mut cmd)?;
    }
    Ok(cmd)
}

/// Point libslirp at `dns-one` without changing the machine resolver.
///
/// libslirp calls `fopen64("/etc/resolv.conf")` and uses the first
/// nameserver. `LD_PRELOAD` sends that one open to `dns-resolv.conf`. QEMU
/// remains the spawned process, so the smoke's `Child::kill` stops it.
fn apply_dns_resolv(root: &Path, board_build: &Path, cmd: &mut Command) -> Result<()> {
    let resolv = write_dns_resolv(board_build)?;
    let preload = ensure_resolv_preload(root, board_build)?;
    cmd.env("LERUX_RESOLV_CONF", &resolv);
    cmd.env("LD_PRELOAD", preload_env(&preload));
    Ok(())
}

fn write_dns_resolv(board_build: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(board_build)
        .with_context(|| format!("mkdir {}", board_build.display()))?;
    let resolv = board_build.join("dns-resolv.conf");
    std::fs::write(&resolv, b"nameserver 127.0.0.1\n")
        .with_context(|| format!("write {}", resolv.display()))?;
    Ok(resolv)
}

fn preload_env(preload: &Path) -> String {
    match std::env::var("LD_PRELOAD") {
        Ok(existing) if !existing.is_empty() => format!("{}:{existing}", path_str(preload)),
        _ => path_str(preload),
    }
}

fn ensure_resolv_preload(root: &Path, board_build: &Path) -> Result<PathBuf> {
    let src = root.join("tools/lerux-cli/resolv_preload.c");
    if !src.is_file() {
        bail!("missing {}", src.display());
    }
    std::fs::create_dir_all(board_build)
        .with_context(|| format!("mkdir {}", board_build.display()))?;
    let so = board_build.join("liblerux-resolv.so");
    if preload_current(&src, &so)? {
        return Ok(so);
    }
    let cc = compiler().context("cc is required to build the resolver preload")?;
    let tmp = board_build.join("liblerux-resolv.so.tmp");
    let _ = std::fs::remove_file(&tmp);
    let status = Command::new(cc)
        .args(["-shared", "-fPIC", "-O2", "-pthread", "-o"])
        .arg(&tmp)
        .arg(&src)
        .status()
        .with_context(|| format!("run {cc} for {}", src.display()))?;
    if !status.success() {
        let _ = std::fs::remove_file(&tmp);
        bail!("{cc} failed to compile {}", src.display());
    }
    std::fs::rename(&tmp, &so).with_context(|| format!("install {}", so.display()))?;
    Ok(so)
}

fn compiler() -> Option<&'static str> {
    ["cc", "gcc", "clang"]
        .into_iter()
        .find(|name| crate::process::command_on_path(name))
}

fn preload_current(src: &Path, so: &Path) -> Result<bool> {
    if !so.is_file() {
        return Ok(false);
    }
    let src_time = std::fs::metadata(src)
        .with_context(|| format!("stat {}", src.display()))?
        .modified()
        .context("resolver preload source mtime")?;
    let so_time = std::fs::metadata(so)
        .with_context(|| format!("stat {}", so.display()))?
        .modified()
        .context("resolver preload mtime")?;
    Ok(so_time >= src_time)
}

/// QMP socket for [`QemuConfig::qmp`]. The smoke test connects here to send keys.
pub fn qmp_socket(ctx: &QemuContext) -> PathBuf {
    board_build_dir(&ctx.root, &ctx.board_name, &ctx.build_dir).join("console.qmp")
}

/// Host-side mirror of `-serial mon:stdio` for x86 QMP smokes. QEMU 6.2 does not
/// reload an x86 OVMF+ISO guest on QMP `system_reset`, so the smoke harness
/// restarts the process and appends to this file for the second boot.
pub fn stdio_smoke_log(ctx: &QemuContext) -> PathBuf {
    board_build_dir(&ctx.root, &ctx.board_name, &ctx.build_dir).join("smoke.stdio.log")
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
        Ok("0" | "false" | "no") => false,
        Ok("1" | "true" | "yes") => true,
        _ => std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok(),
    }
}

/// Guest serial for a headless smoke. Graphic windows use `vc` instead.
fn headless_serial() -> [&'static str; 2] {
    ["-serial", "mon:stdio"]
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
        c.args(headless_serial());
        c.arg("-nographic");
    }
    if qemu.ramfb {
        c.args(["-device", "ramfb"]);
    }
    c.args([
        "-device",
        &format!("loader,file={},addr=0x70000000,cpu-num=0", path_str(loader)),
    ]);
    // Virtio-mmio slot order matters: net first, blk at +0xc00, gpu at +0xa00
    // in the same page. The first device occupies the highest address.
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
    if qemu.gpu {
        c.args(["-device", "virtio-gpu-device"]);
    }
    c
}

fn riscv64_command(qemu: &QemuConfig, loader: &Path, disk: &Path) -> Command {
    let mut c = Command::new("qemu-system-riscv64");
    c.args(["-machine", "virt", "-m", "size=2G", "-nographic"]);
    c.args(headless_serial());
    c.args(["-kernel", &path_str(loader)]);
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
    if qemu.gpu {
        c.args(["-device", "virtio-gpu-device,bus=virtio-mmio-bus.2"]);
    }
    c
}

const X86_VIRTIO_BLK_PCI: &str = "virtio-blk-pci,id=blk0,addr=0x3.0x0,drive=blkdev0";
const X86_VIRTIO_NET_PCI: &str = "virtio-net-pci,id=net0,addr=0x4.0x0,netdev=netdev0";

fn x86_net_device(board: &str) -> Result<String> {
    match x86_net_backend(board)? {
        NetBackend::Virtio => Ok(X86_VIRTIO_NET_PCI.to_string()),
        NetBackend::E1000e => Ok(e1000e_net_device()),
    }
}

fn x86_block_disk_device(board_name: &str) -> Result<String> {
    match x86_disk_backend(board_name)? {
        DiskBackend::Virtio => Ok(X86_VIRTIO_BLK_PCI.to_string()),
        DiskBackend::Ahci => Ok(ahci_disk_device("blkdev0")),
    }
}

fn x86_command(
    ctx: &QemuContext,
    qemu: &QemuConfig,
    _loader: &Path,
    disk: &Path,
) -> Result<Command> {
    // QEMU `-kernel` rejects ELF64. Boot the same GRUB ISO as metal under OVMF.
    let ovmf = crate::iso::require_ovmf()?;
    let iso =
        crate::iso::ensure_iso_for_qemu(&ctx.root, &ctx.board_name, &ctx.build_dir, &ctx.config)?;

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
    ]);
    c.args(headless_serial());
    crate::iso::append_ovmf_iso_boot(&mut c, &ovmf, &iso);
    // Fixed PCI slots: blk at 0x3, net at 0x4 (match system_vars BAR addresses).
    // AHCI data disks use ide.0; the ISO sits on ide.1 (bootindex 0).
    if let Some(blockdev) = blockdev_arg(qemu.disk, disk) {
        c.args([
            "-device",
            &x86_block_disk_device(&ctx.board_name)?,
            "-blockdev",
            &blockdev,
        ]);
    }
    if let Some(netdev) = netdev_arg(qemu.net) {
        let device = x86_net_device(&ctx.board_name)?;
        c.args(["-device", &device, "-netdev", netdev]);
    }
    append_usb_kbd(&mut c, qemu);
    Ok(c)
}

/// PCI 00:14.0 matches the Z97 xHCI and the system description's MSI route.
fn append_usb_kbd(cmd: &mut Command, qemu: &QemuConfig) {
    if !qemu.usb_kbd {
        return;
    }
    cmd.args([
        "-device",
        "qemu-xhci,id=xhci,addr=0x14.0x0",
        "-device",
        "usb-kbd,bus=xhci.0",
    ]);
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
    if qemu.tcp_echo {
        helpers.push(start_tcp_echo_background(18080)?);
    }
    if qemu.dns_one {
        helpers.push(crate::dns_one::start_dns_one_background()?);
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
    use std::{
        collections::BTreeMap,
        io::Read,
        process::{Child, Command, ExitStatus, Stdio},
        thread,
        time::{Duration, Instant},
    };

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

    fn render_riscv64(qemu_toml: &str) -> String {
        let qemu: QemuConfig = toml::from_str(qemu_toml).unwrap();
        format!(
            "{:?}",
            riscv64_command(&qemu, Path::new("loader.img"), Path::new("disk.img"))
        )
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
    fn usb_kbd_adds_qemu_xhci_with_msi() {
        let qemu: QemuConfig = toml::from_str("usb_kbd = true").unwrap();
        let mut cmd = Command::new("qemu-system-x86_64");
        append_usb_kbd(&mut cmd, &qemu);
        let line = format!("{cmd:?}");
        assert!(line.contains("qemu-xhci,id=xhci,addr=0x14.0x0"), "{line}");
        assert!(line.contains("usb-kbd,bus=xhci.0"), "{line}");
    }

    #[test]
    fn dns_one_defaults_off() {
        let qemu: QemuConfig = toml::from_str("net = \"user\"").unwrap();
        assert!(!qemu.dns_one);
    }

    fn cmd_env(cmd: &Command, key: &str) -> String {
        cmd.get_envs()
            .find(|(name, _)| *name == key)
            .and_then(|(_, value)| value)
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    #[test]
    fn dns_command_points_qemu_at_the_local_resolver() {
        let dir = std::env::temp_dir().join(format!("lerux-dns-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut cmd = Command::new("qemu-system-aarch64");
        cmd.arg("-nographic");
        apply_dns_resolv(&root, &dir, &mut cmd).expect("resolv");
        let line = format!("{cmd:?}");
        assert!(line.contains("qemu-system-aarch64"), "{line}");
        assert!(line.contains("-nographic"), "{line}");
        assert!(!line.contains("sudo"), "{line}");
        assert!(!line.contains("unshare"), "{line}");
        let resolv = cmd_env(&cmd, "LERUX_RESOLV_CONF");
        assert!(resolv.ends_with("dns-resolv.conf"), "{resolv}");
        assert_eq!(
            std::fs::read(&resolv).expect("resolv bytes"),
            b"nameserver 127.0.0.1\n"
        );
        assert!(
            cmd_env(&cmd, "LD_PRELOAD").contains("liblerux-resolv.so"),
            "{}",
            cmd_env(&cmd, "LD_PRELOAD")
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn resolv_preload_serves_the_smoke_file() {
        let dir = std::env::temp_dir().join(format!("lerux-resolv-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let so = ensure_resolv_preload(&root, &dir).expect("preload");
        let fake = dir.join("dns-resolv.conf");
        std::fs::write(&fake, b"nameserver 203.0.113.53\n").expect("write resolv");
        let reader = dir.join("read-resolv.c");
        std::fs::write(
            &reader,
            b"#define _FILE_OFFSET_BITS 64\n#include <stdio.h>\nint main(void){char b[64];FILE *f=fopen(\"/etc/resolv.conf\",\"r\");if(!f||!fgets(b,sizeof b,f))return 2;fputs(b,stdout);return 0;}\n",
        )
        .expect("write reader");
        let bin = dir.join("read-resolv");
        let cc = compiler().expect("cc");
        let compiled = Command::new(cc)
            .args(["-O2", "-o"])
            .arg(&bin)
            .arg(&reader)
            .status()
            .expect("compile reader");
        assert!(compiled.success(), "reader compile");
        let hooked = Command::new(&bin)
            .env("LD_PRELOAD", &so)
            .env("LERUX_RESOLV_CONF", &fake)
            .output()
            .expect("run reader");
        assert!(
            hooked.status.success(),
            "hooked reader: {}",
            String::from_utf8_lossy(&hooked.stderr)
        );
        assert_eq!(hooked.stdout, b"nameserver 203.0.113.53\n");
        let plain = Command::new(&bin).output().expect("plain reader");
        assert!(plain.status.success(), "plain reader");
        assert_ne!(plain.stdout, hooked.stdout);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ps2_console_does_not_add_xhci() {
        let qemu: QemuConfig = toml::from_str("qmp = true").unwrap();
        let mut cmd = Command::new("qemu-system-x86_64");
        append_usb_kbd(&mut cmd, &qemu);
        assert!(!format!("{cmd:?}").contains("qemu-xhci"));
    }

    #[test]
    fn gpu_device_follows_net_and_blk() {
        let line = render_aarch64("disk = \"rw\"\nnet = \"user\"\ngpu = true", false);
        let net = line.find("virtio-net-device").expect("net device");
        let blk = line.find("virtio-blk-device").expect("blk device");
        let gpu = line.find("virtio-gpu-device").expect("gpu device");
        assert!(net < blk && blk < gpu, "{line}");
    }

    #[test]
    fn riscv_gpu_on_bus_two_only_when_enabled() {
        let with_gpu = render_riscv64("disk = \"rw\"\nnet = \"user\"\ngpu = true");
        let bus0 = with_gpu.find("virtio-mmio-bus.0").expect("blk bus");
        let bus1 = with_gpu.find("virtio-mmio-bus.1").expect("net bus");
        let bus2 = with_gpu.find("virtio-mmio-bus.2").expect("gpu bus");
        assert!(bus0 < bus1, "{with_gpu}");
        assert!(bus1 < bus2, "{with_gpu}");
        let without_gpu = render_riscv64("disk = \"rw\"\nnet = \"user\"");
        assert!(!without_gpu.contains("virtio-mmio-bus.2"), "{without_gpu}");
    }

    #[test]
    fn headless_serial_stays_on_the_monitor_stdio() {
        let mut cmd = Command::new("qemu");
        cmd.args(headless_serial());
        let line = format!("{cmd:?}");
        assert!(
            line.contains("\"-serial\" \"mon:stdio\""),
            "guest serial stays on the monitor stdio: {line}"
        );
        let riscv = render_riscv64("qmp = true");
        assert!(riscv.contains("\"-serial\" \"mon:stdio\""), "{riscv}");
        let aarch64 = render_aarch64("qmp = true", false);
        assert!(aarch64.contains("\"-serial\" \"mon:stdio\""), "{aarch64}");
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

    #[test]
    fn x86_block_disk_ahci_uses_ide_hd_on_stock_controller() {
        let dev = x86_block_disk_device("x86_64_generic_fs_ahci").unwrap();
        assert_eq!(dev, "ide-hd,bus=ide.0,drive=blkdev0,unit=0");
        assert!(!dev.contains("ich9-ahci"));
        assert!(!dev.contains("virtio-blk"));
    }

    #[test]
    fn x86_block_disk_sddf_uses_virtio_pci() {
        let dev = x86_block_disk_device("x86_64_generic_fs_sddf").unwrap();
        assert_eq!(dev, X86_VIRTIO_BLK_PCI);
    }

    #[test]
    fn x86_net_device_e1000e_disables_vnet_headers() {
        let dev = x86_net_device("x86_64_generic_net_e1000e").unwrap();
        assert_eq!(
            dev,
            "e1000e,id=net0,addr=4.0,netdev=netdev0,disable_vnet_hdr=1"
        );
        assert!(!dev.contains("mac="));
        assert!(!dev.contains("virtio"));
    }

    #[test]
    fn x86_net_device_sddf_stays_virtio() {
        let dev = x86_net_device("x86_64_generic_net_sddf").unwrap();
        assert_eq!(dev, X86_VIRTIO_NET_PCI);
    }

    #[test]
    fn x86_fetch_native_uses_ide_hd_and_e1000e() {
        assert_eq!(
            x86_block_disk_device("x86_64_generic_fetch_native").expect("disk"),
            "ide-hd,bus=ide.0,drive=blkdev0,unit=0"
        );
        assert_eq!(
            x86_net_device("x86_64_generic_fetch_native").expect("net"),
            "e1000e,id=net0,addr=4.0,netdev=netdev0,disable_vnet_hdr=1"
        );
        assert_eq!(
            x86_block_disk_device("x86_64_generic_fetch_sddf").expect("virtio disk"),
            X86_VIRTIO_BLK_PCI
        );
        assert_eq!(
            x86_net_device("x86_64_generic_fetch_sddf").expect("virtio net"),
            X86_VIRTIO_NET_PCI
        );
    }

    const INTEGER_HELP_TYPES: &[&str] = &[
        "uint8", "uint16", "uint32", "uint64", "int8", "int16", "int32", "int64",
    ];

    /// Help text shaped like `qemu-system-x86_64 -device e1000e,help`.
    const E1000E_HELP_FIXTURE: &str = "\
e1000e options:
  disable_vnet_hdr=<uint8> - Do not use virtio headers (default: 0)
  multifunction=<bool>   - on/off (default: false)
  mac=<str>              - Ethernet MAC
";

    fn device_name(line: &str) -> &str {
        line.split(',').next().unwrap_or(line)
    }

    fn property_types(help: &str) -> BTreeMap<String, String> {
        let mut types = BTreeMap::new();
        for raw in help.lines() {
            let line = raw.trim();
            let Some((name, rest)) = line.split_once("=<") else {
                continue;
            };
            let Some((ty, _)) = rest.split_once('>') else {
                continue;
            };
            if name.contains(' ') {
                continue;
            }
            types.insert(name.to_string(), ty.to_string());
        }
        types
    }

    fn integer_on_off(line: &str, types: &BTreeMap<String, String>) -> Option<String> {
        for part in line.split(',').skip(1) {
            let Some((key, value)) = part.split_once('=') else {
                continue;
            };
            let Some(ty) = types.get(key) else {
                continue;
            };
            if INTEGER_HELP_TYPES.contains(&ty.as_str()) && (value == "on" || value == "off") {
                return Some(format!("{key}={value} is <{ty}>"));
            }
        }
        None
    }

    fn q35_device_lines() -> Vec<String> {
        vec![
            e1000e_net_device(),
            ahci_disk_device("blkdev0"),
            X86_VIRTIO_BLK_PCI.to_string(),
            X86_VIRTIO_NET_PCI.to_string(),
            "qemu-xhci,id=xhci,addr=0x14.0x0".to_string(),
            "usb-kbd,bus=xhci.0".to_string(),
        ]
    }

    fn wait_for_help(child: &mut Child, name: &str) -> ExitStatus {
        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return status,
                Ok(None) if started.elapsed() > Duration::from_secs(5) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("qemu help for {name} did not exit within 5s");
                }
                Ok(None) => thread::sleep(Duration::from_millis(20)),
                Err(err) => panic!("wait qemu help for {name}: {err}"),
            }
        }
    }

    fn read_pipe(pipe: Option<impl Read>, name: &str) -> String {
        let Some(mut pipe) = pipe else {
            return String::new();
        };
        let mut text = String::new();
        pipe.read_to_string(&mut text)
            .unwrap_or_else(|err| panic!("read qemu help for {name}: {err}"));
        text
    }

    fn q35_device_help(name: &str) -> String {
        let mut child = Command::new("qemu-system-x86_64")
            .args([
                "-machine",
                "q35",
                "-nodefaults",
                "-display",
                "none",
                "-S",
                "-monitor",
                "none",
                "-device",
                &format!("{name},help"),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|err| panic!("spawn qemu help for {name}: {err}"));
        let status = wait_for_help(&mut child, name);
        let out = read_pipe(child.stdout.take(), name);
        let err = read_pipe(child.stderr.take(), name);
        assert!(status.success(), "qemu help for {name} failed: {err}{out}");
        out
    }

    #[test]
    fn integer_property_rejects_on_off_from_help_fixture() {
        let types = property_types(E1000E_HELP_FIXTURE);
        assert_eq!(
            types.get("disable_vnet_hdr").map(String::as_str),
            Some("uint8")
        );
        assert!(
            integer_on_off("e1000e,id=net0,disable_vnet_hdr=on", &types).is_some(),
            "uint8 on must fail"
        );
        assert!(integer_on_off("e1000e,disable_vnet_hdr=off", &types).is_some());
        assert!(integer_on_off("e1000e,disable_vnet_hdr=1", &types).is_none());
        assert!(
            integer_on_off("e1000e,multifunction=on", &types).is_none(),
            "bool on stays legal"
        );
        assert!(integer_on_off("e1000e,id=net0", &types).is_none());
    }

    #[test]
    fn q35_device_lines_match_help_types() {
        let mut help = BTreeMap::new();
        for line in q35_device_lines() {
            let name = device_name(&line).to_string();
            if !help.contains_key(&name) {
                let text = q35_device_help(&name);
                help.insert(name.clone(), text);
            }
            let types = property_types(&help[&name]);
            assert!(
                integer_on_off(&line, &types).is_none(),
                "{line} uses on/off for an integer property"
            );
            if name == "e1000e" {
                assert_eq!(
                    types.get("disable_vnet_hdr").map(String::as_str),
                    Some("uint8"),
                    "live help:\n{}",
                    help[&name]
                );
            }
        }
    }
}
