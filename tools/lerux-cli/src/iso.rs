//! UEFI ISO for x86 Multiboot 2 boards.
//!
//! `lerux iso` packs `mb2-stub.elf`, `sel4.elf` (64-bit), and `loader.img`
//! into one FAT, then publishes that FAT as `boot/esp.img` on an ISO9660
//! image. GRUB Multiboot 2-boots the stub. The stub ELF-loads `sel4.elf` to
//! its 1 MiB link address after `ExitBootServices`. The image is a file.
//! It does not write a USB device.
//!
//! `grub-mkimage` comes from the distro (`grub-efi-amd64-bin` on Debian,
//! `grub` on Arch). `xorriso` is the system binary (`libisoburn` on Arch).

use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use anyhow::{bail, Context, Result};

use crate::{
    board::{get_board, load_boards, Board},
    boot_volume::{recipe_stamp, GrubEfi, GrubHost, StubBootImage, StubPayloads, XORRISO_BOOT},
    image_digest,
    process::{self, command_on_path, path_str},
};

/// Build `build/<board>/lerux_<board>.iso` (or `--output`) for an x86_64 board.
///
/// Rebuilds the Microkit image when `build_if_missing` is set, including when
/// an older `loader.img` is already on disk. When `verify` is set, both SHA-256 sidecars must
/// match before anything is packed. Sidecars are not copied onto the ISO.
/// Prints USB flash instructions after packing. Always packs, then writes the
/// boot-recipe stamp.
pub fn build_iso(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
    output: Option<&Path>,
    build_if_missing: bool,
    verify: bool,
) -> Result<PathBuf> {
    let out = pack_iso(
        root,
        board,
        build_dir,
        config,
        output,
        build_if_missing,
        verify,
    )?;
    print_usb_instructions(board, &out);
    Ok(out)
}

/// Pack `build/<board>/lerux_<board>.iso` for a QEMU smoke (no USB flash text).
///
/// Repacks unless `lerux_<board>.iso.boot-recipe` matches [`recipe_stamp`] and
/// the ISO is at least as new as the kernel, the loader, the stub sources, and
/// the host GRUB files. A missing GRUB install fails here, before an older ISO
/// is reused.
pub fn ensure_iso_for_qemu(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
) -> Result<PathBuf> {
    let out = resolve_iso_output(root, build_dir, board, None)?;
    let host = GrubHost::discover()?;
    let board_build = root.join(build_dir).join(board);
    let loader = board_build.join("loader.img");
    let kernel = board_build.join("sel4.elf");
    let mut inputs = vec![kernel, loader];
    for rel in StubPayloads::SOURCES {
        inputs.push(root.join(rel));
    }
    inputs.extend(host.freshness_inputs()?);
    if iso_is_current(&out, &recipe_path(&out), &recipe_stamp(), &inputs) {
        return Ok(out);
    }
    pack_iso(
        root, board, build_dir, config, None, /* build_if_missing */ false,
        /* verify */ true,
    )
}

fn pack_iso(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
    output: Option<&Path>,
    build_if_missing: bool,
    verify: bool,
) -> Result<PathBuf> {
    require_single_normal_segment(board, "--board")?;
    require_relative_under_root(build_dir, "--build-dir")?;
    let boards = load_boards(root)?;
    let board_cfg = get_board(&boards, board)?;
    require_x86_iso_board(board_cfg)?;

    let out = resolve_iso_output(root, build_dir, board, output)?;
    let board_build = root.join(build_dir).join(board);
    let loader = board_build.join("loader.img");
    let kernel = board_build.join("sel4.elf");
    if build_if_missing {
        println!("==> building {board} before packing the ISO…");
        crate::build::image(root, board, build_dir, config)?;
    } else if !loader.is_file() || !kernel.is_file() {
        bail!(
            "missing {} or {}; run `BOARD={board} just image` or omit --no-build",
            loader.display(),
            kernel.display()
        );
    }

    let image = uefi_boot_volume(root, &kernel, &loader, verify)?;
    let xorriso = ensure_xorriso()?;
    write_uefi_iso(&xorriso, &image, &out)?;

    let size = fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    println!("==> Wrote ISO ({size} bytes) → {}", out.display());
    Ok(out)
}

/// Read the three payloads and build the GRUB EFI image.
pub(crate) fn uefi_boot_volume(
    root: &Path,
    kernel: &Path,
    loader: &Path,
    verify: bool,
) -> Result<StubBootImage> {
    if verify {
        image_digest::verify_sidecar(kernel)?;
        image_digest::verify_sidecar(loader)?;
    } else if !kernel.is_file() || !loader.is_file() {
        bail!(
            "missing boot files {} and {}",
            kernel.display(),
            loader.display()
        );
    }
    let kernel_bytes = fs::read(kernel).with_context(|| format!("read {}", kernel.display()))?;
    let loader_bytes = fs::read(loader).with_context(|| format!("read {}", loader.display()))?;
    let stub_path = build_mb2_stub(root)?;
    let stub_bytes =
        fs::read(&stub_path).with_context(|| format!("read {}", stub_path.display()))?;
    let payloads = StubPayloads::new(stub_bytes, kernel_bytes, loader_bytes)?;
    let efi = efi_image()?;
    Ok(StubBootImage::assemble(efi, payloads))
}

fn efi_image() -> Result<GrubEfi> {
    #[cfg(test)]
    {
        Ok(GrubEfi::from_bytes_for_test(b"test-efi".to_vec()))
    }
    #[cfg(not(test))]
    {
        GrubEfi::build(&GrubHost::discover()?)
    }
}

fn iso_is_current(
    iso: &Path,
    recipe_path: &Path,
    expected_recipe: &str,
    inputs: &[PathBuf],
) -> bool {
    let Ok(iso_meta) = fs::metadata(iso) else {
        return false;
    };
    let Ok(iso_mtime) = iso_meta.modified() else {
        return false;
    };
    let Ok(stored) = fs::read_to_string(recipe_path) else {
        return false;
    };
    if stored != expected_recipe {
        return false;
    }
    inputs
        .iter()
        .all(|input| mtime_at_or_before(input, iso_mtime))
}

fn mtime_at_or_before(path: &Path, iso_mtime: SystemTime) -> bool {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .map(|mtime| mtime <= iso_mtime)
        .unwrap_or(false)
}

fn recipe_path(iso: &Path) -> PathBuf {
    let mut name = iso
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    name.push(".boot-recipe");
    iso.with_file_name(name)
}

fn write_uefi_iso(xorriso: &Path, image: &StubBootImage, out: &Path) -> Result<()> {
    let esp = image.fat_image()?;
    let staging = tempfile::tempdir().context("iso staging directory")?;
    let esp_path = staging.path().join("boot/esp.img");
    if let Some(parent) = esp_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(&esp_path, &esp).with_context(|| format!("write {}", esp_path.display()))?;

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let partial = sibling_partial(out)?;
    if partial.exists() {
        fs::remove_file(&partial).with_context(|| format!("remove {}", partial.display()))?;
    }

    println!("==> xorriso → {}", out.display());
    let mut cmd = Command::new(xorriso);
    cmd.args(XORRISO_BOOT);
    cmd.arg(staging.path());
    cmd.arg("-o");
    cmd.arg(&partial);
    let status = cmd.status().context("failed to run xorriso")?;
    if !status.success() {
        let _ = fs::remove_file(&partial);
        bail!("xorriso exited with {status} (package xorriso)");
    }

    fs::rename(&partial, out)
        .with_context(|| format!("rename {} → {}", partial.display(), out.display()))?;
    write_recipe_stamp(out)?;
    Ok(())
}

fn write_recipe_stamp(iso: &Path) -> Result<()> {
    let dest = recipe_path(iso);
    let partial = sibling_partial(&dest)?;
    fs::write(&partial, recipe_stamp()).with_context(|| format!("write {}", partial.display()))?;
    fs::rename(&partial, &dest)
        .with_context(|| format!("rename {} → {}", partial.display(), dest.display()))?;
    Ok(())
}

fn sibling_partial(path: &Path) -> Result<PathBuf> {
    let mut name = path
        .file_name()
        .context("path has no file name")?
        .to_os_string();
    name.push(".partial");
    Ok(path.with_file_name(name))
}

/// Boot an existing ISO in QEMU as a raw disk under OVMF (UEFI).
///
/// Matches the metal USB path: GRUB `EFI/BOOT/BOOTX64.EFI` → `mb2-stub` → seL4.
/// Requires a combined OVMF firmware (`OVMF_PATH` or a packaged `OVMF.fd`).
pub fn boot_test(root: &Path, iso: &Path, board: &str, build_dir: &str) -> Result<()> {
    let ovmf = require_ovmf()?;
    println!("==> ISO boot test (OVMF UEFI, raw disk)");
    boot_iso_ovmf(root, iso, board, build_dir, &ovmf)
}

fn boot_iso_ovmf(root: &Path, iso: &Path, board: &str, build_dir: &str, ovmf: &Path) -> Result<()> {
    if !iso.is_file() {
        bail!("missing ISO {}", iso.display());
    }
    if !command_on_path("qemu-system-x86_64") {
        bail!("qemu-system-x86_64 is not on PATH");
    }

    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args([
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
        "-no-reboot",
    ]);
    append_ovmf_iso_boot(&mut cmd, ovmf, iso);
    let mut test = crate::smoke_expects::smoke_test_for_board(root, board)?;
    test.curls.clear();
    let log = root
        .join(build_dir)
        .join("smoke-logs")
        .join(format!("{board}.iso-uefi.serial.log"));
    crate::test::run_smoke_with_capture(cmd, &test, Some(&log))
}

/// Attach combined OVMF and the ISO on ICH9 AHCI bus `ide.1`.
///
/// q35 AHCI ports are one-unit buses (`ide.0`, `ide.1`, …). Data disks that
/// use port 0 stay on `ide.0`; the ISO uses `ide.1` with `bootindex=0` so
/// OVMF prefers it.
pub fn append_ovmf_iso_boot(cmd: &mut Command, ovmf: &Path, iso: &Path) {
    // Not `readonly=on`: QEMU rejects `ide-hd` on a read-only block node.
    let drive = format!("file={},format=raw,if=none,id=leruxboot", path_str(iso));
    cmd.args(["-bios", &path_str(ovmf)]);
    cmd.args([
        "-drive",
        &drive,
        "-device",
        "ide-hd,drive=leruxboot,bus=ide.1,unit=0,bootindex=0",
    ]);
}

/// Combined OVMF image for QEMU `-bios` (not a split CODE/VARS pair).
pub fn require_ovmf() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("OVMF_PATH") {
        let p = PathBuf::from(&path);
        if p.is_file() {
            return Ok(p);
        }
        bail!("OVMF_PATH={path} is not a file");
    }
    for candidate in [
        "/usr/share/ovmf/OVMF.fd",
        "/usr/share/OVMF/OVMF.fd",
        "/usr/share/qemu/OVMF.fd",
        "/usr/share/edk2/x64/OVMF.4m.fd",
    ] {
        let p = PathBuf::from(candidate);
        if p.is_file() {
            return Ok(p);
        }
    }
    bail!(
        "OVMF firmware not found. Install the ovmf package or set OVMF_PATH to a combined OVMF.fd"
    )
}

/// Build `support/mb2-stub/mb2-stub.elf` with clang (`support/mb2-stub/build.sh`).
pub fn build_mb2_stub(root: &Path) -> Result<PathBuf> {
    let script = root.join("support/mb2-stub/build.sh");
    let out = root.join("support/mb2-stub/mb2-stub.elf");
    if !script.is_file() {
        bail!("missing {}", script.display());
    }
    if !command_on_path("clang") {
        bail!("clang is required to build support/mb2-stub (UEFI Multiboot 2 trampoline)");
    }
    eprintln!("==> Building mb2-stub (UEFI Multiboot 2 trampoline)");
    process::run_checked("bash", &[&path_str(&script), &path_str(&out)])
        .context("build support/mb2-stub")?;
    if !out.is_file() {
        bail!("mb2-stub build did not write {}", out.display());
    }
    Ok(out)
}

fn ensure_xorriso() -> Result<PathBuf> {
    which::which("xorriso").context(
        "xorriso is not on PATH. Install this system's package (Arch: pacman -S libisoburn) and re-run.",
    )
}

fn print_usb_instructions(board: &str, iso: &Path) {
    println!();
    println!("USB flash drive (this command does not write a disk):");
    println!("  lsblk");
    println!(
        "  sudo dd if={} of=/dev/sdX bs=4M status=progress conv=fsync",
        iso.display()
    );
    if board == "swift_sfx14_41g" {
        println!("Replace sdX with the flash drive from lsblk. Do not write the internal NVMe.");
        println!("Firmware boot menu → the USB entry that says UEFI (GRUB EFI/BOOT/BOOTX64.EFI).");
        println!("mb2-stub loads sel4.elf (64-bit) at 1 MiB after ExitBootServices.");
        println!(
            "Screen: the GOP mode line, then \"1 browser\", \"2 maths\", and \"3 wifi\". This laptop has no COM port."
        );
        println!("Expect: 1 browser");
        return;
    }
    println!(
        "Replace sdX with the flash drive from lsblk. Do not use sda (Ubuntu SSD) or sdb (data disk)."
    );
    println!(
        "Firmware boot menu (F12) → the USB entry that says UEFI (GRUB EFI/BOOT/BOOTX64.EFI)."
    );
    println!("mb2-stub loads sel4.elf (64-bit) at 1 MiB after ExitBootServices.");
    println!(
        "Screen: blue background, \"hello lerux\" on the first line, then \"1 browser\" and \"2 maths\"."
    );
    println!(
        "Keyboard: PS/2 in the rear combo port, or a USB HID boot keyboard on a root xHCI port."
    );
    println!(
        "1 or browser prints \"browser unavailable\". 2 or maths reads expressions until quit."
    );
    println!("Console: COM1 115200 8N1 on the COMA header.");
    println!("Expect: 1 browser");
    println!("Full procedure: docs/boards.md#gigabyte-z97-d3h-install-path");
}

pub(crate) fn iso_file_name(board: &str) -> String {
    format!("lerux_{board}.iso")
}

fn require_x86_iso_board(board: &Board) -> Result<()> {
    if board.arch == "x86_64" {
        Ok(())
    } else {
        bail!(
            "ISO boot images are Multiboot 2 and only built for x86_64 boards (got arch {})",
            board.arch
        )
    }
}

fn resolve_iso_output(
    root: &Path,
    build_dir: &str,
    board: &str,
    output: Option<&Path>,
) -> Result<PathBuf> {
    let Some(output) = output else {
        return Ok(root.join(build_dir).join(board).join(iso_file_name(board)));
    };
    if has_parent_dir(output) {
        bail!(
            "--output must not contain '..' path segments: {}",
            output.display()
        );
    }
    if output.is_absolute() {
        if output.starts_with("/dev") {
            bail!(
                "--output must be an ISO file, not a device node under /dev: {}",
                output.display()
            );
        }
        return Ok(output.to_path_buf());
    }
    let joined = root.join(output);
    let build_root = root.join(build_dir);
    if !joined.starts_with(&build_root) {
        bail!(
            "relative --output must stay under {build_dir} (got {})",
            output.display()
        );
    }
    Ok(joined)
}

fn has_parent_dir(path: &Path) -> bool {
    path.components().any(|c| matches!(c, Component::ParentDir))
}

fn require_single_normal_segment(name: &str, what: &str) -> Result<()> {
    let path = Path::new(name);
    let mut comps = path.components();
    match (comps.next(), comps.next()) {
        (Some(Component::Normal(seg)), None) if seg == name => Ok(()),
        _ => bail!("{what} must be a single path segment, got {name:?}"),
    }
}

fn require_relative_under_root(user: &str, what: &str) -> Result<()> {
    let path = Path::new(user);
    if user.is_empty() {
        bail!("{what} must not be empty");
    }
    if path.is_absolute() {
        bail!("{what} must be a relative path under the repository, got {user}");
    }
    if has_parent_dir(path) {
        bail!("{what} must not contain '..' path segments: {user}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    fn x86_board(arch: &str) -> Board {
        Board {
            arch: arch.to_string(),
            microkit_board: "x86_64_generic".to_string(),
            target: "x86_64-sel4-microkit".to_string(),
            template: "serial-hello-x86.system.template".to_string(),
            pds: vec!["hello".to_string()],
            qemu: None,
            ci: false,
            curl_expect: None,
            system_vars: BTreeMap::new(),
        }
    }

    #[test]
    fn ovmf_iso_boot_uses_ide_bus_one() {
        let mut cmd = Command::new("qemu-system-x86_64");
        append_ovmf_iso_boot(
            &mut cmd,
            Path::new("/usr/share/ovmf/OVMF.fd"),
            Path::new("lerux_pc_z97_d3h.iso"),
        );
        let line = format!("{cmd:?}");
        assert!(line.contains("\"-bios\""), "{line}");
        assert!(line.contains("id=leruxboot"), "{line}");
        assert!(
            line.contains("ide-hd,drive=leruxboot,bus=ide.1,unit=0,bootindex=0"),
            "{line}"
        );
    }

    #[test]
    fn non_x86_board_is_rejected() {
        let err = require_x86_iso_board(&x86_board("aarch64"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("x86_64"), "{err}");
        assert!(err.contains("aarch64"), "{err}");
    }

    #[test]
    fn volume_copies_stub_kernel_and_loader_without_sidecars() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let kernel = tmp.path().join("sel4.elf");
        let loader = tmp.path().join("loader.img");
        fs::write(&kernel, b"kernel-bytes").unwrap();
        fs::write(&loader, b"loader-bytes").unwrap();
        image_digest::write_sidecar(&kernel).unwrap();
        image_digest::write_sidecar(&loader).unwrap();

        let image = uefi_boot_volume(&root, &kernel, &loader, true).unwrap();
        let dest = tmp.path().join("esp");
        image.write_to(&dest).unwrap();

        assert!(dest.join("boot/mb2-stub.elf").is_file());
        assert_eq!(
            fs::read(dest.join("boot/sel4.elf")).unwrap(),
            b"kernel-bytes"
        );
        assert_eq!(
            fs::read(dest.join("boot/loader.img")).unwrap(),
            b"loader-bytes"
        );
        assert!(!dest.join("boot/sel4.elf.sha256").exists());
        assert!(!dest.join("boot/loader.img.sha256").exists());
        let cfg = fs::read_to_string(dest.join("boot/grub/grub.cfg")).unwrap();
        let stub_at = cfg.find("multiboot2 /boot/mb2-stub.elf").expect("stub");
        let kernel_at = cfg.find("module2 /boot/sel4.elf").expect("kernel");
        let loader_at = cfg.find("module2 /boot/loader.img").expect("loader");
        assert!(stub_at < kernel_at && kernel_at < loader_at, "{cfg}");
        assert!(!cfg.contains("multiboot2 /boot/sel4.elf"), "{cfg}");
        assert!(!dest.join("boot/limine.conf").exists());
        let stub = fs::read(dest.join("boot/mb2-stub.elf")).unwrap();
        let magic = 0xe85250d6u32.to_le_bytes();
        let at = stub
            .windows(4)
            .position(|window| window == magic)
            .expect("multiboot2 magic");
        let header_len = mb2_header_len(&stub, at);
        assert!(
            at + header_len <= 8192,
            "multiboot header at {at} length {header_len} is outside GRUB's search"
        );
        assert!(
            mb2_requests_framebuffer(&stub, at),
            "stub header does not request a framebuffer; EFI would warn and pass no mode"
        );
        let loads = elf32_load_segments(&stub);
        assert!(
            loads
                .iter()
                .any(|seg| seg.vaddr == 0x01FE_0000 && seg.paddr == seg.vaddr),
            "mode page load address changed: {loads:?}"
        );
        assert!(
            loads
                .iter()
                .any(|seg| seg.vaddr == 0x0200_0000 && seg.paddr == seg.vaddr),
            "stub load address changed: {loads:?}"
        );
    }

    #[derive(Debug)]
    struct ElfLoad {
        vaddr: u32,
        paddr: u32,
    }

    fn mb2_header_len(elf: &[u8], at: usize) -> usize {
        u32::from_le_bytes(elf[at + 8..at + 12].try_into().unwrap()) as usize
    }

    /// Header tag type 5, size 20, with width, height, and depth left at 0.
    fn mb2_requests_framebuffer(elf: &[u8], at: usize) -> bool {
        let len = mb2_header_len(elf, at);
        if at + len > elf.len() || len < 24 {
            return false;
        }
        let header = &elf[at..at + len];
        let mut off = 16usize;
        while off + 8 <= header.len() {
            let kind = u16::from_le_bytes(header[off..off + 2].try_into().unwrap());
            let size = u32::from_le_bytes(header[off + 4..off + 8].try_into().unwrap()) as usize;
            if kind == 0 {
                return false;
            }
            if kind == 5 && size == 20 && off + 20 <= header.len() {
                let width = u32::from_le_bytes(header[off + 8..off + 12].try_into().unwrap());
                let height = u32::from_le_bytes(header[off + 12..off + 16].try_into().unwrap());
                let depth = u32::from_le_bytes(header[off + 16..off + 20].try_into().unwrap());
                return width == 0 && height == 0 && depth == 0;
            }
            if size < 8 {
                return false;
            }
            off += size.div_ceil(8) * 8;
        }
        false
    }

    fn elf32_load_segments(elf: &[u8]) -> Vec<ElfLoad> {
        let phoff = u32::from_le_bytes(elf[28..32].try_into().unwrap()) as usize;
        let phentsize = u16::from_le_bytes(elf[42..44].try_into().unwrap()) as usize;
        let phnum = u16::from_le_bytes(elf[44..46].try_into().unwrap()) as usize;
        let mut loads = Vec::new();
        for i in 0..phnum {
            let off = phoff + i * phentsize;
            let kind = u32::from_le_bytes(elf[off..off + 4].try_into().unwrap());
            if kind == 1 {
                let vaddr = u32::from_le_bytes(elf[off + 8..off + 12].try_into().unwrap());
                let paddr = u32::from_le_bytes(elf[off + 12..off + 16].try_into().unwrap());
                loads.push(ElfLoad { vaddr, paddr });
            }
        }
        loads
    }

    #[test]
    fn volume_refuses_missing_sidecar() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let kernel = tmp.path().join("sel4.elf");
        let loader = tmp.path().join("loader.img");
        fs::write(&kernel, b"kernel-bytes").unwrap();
        fs::write(&loader, b"loader-bytes").unwrap();
        image_digest::write_sidecar(&loader).unwrap();

        let dest = tmp.path().join("esp");
        let err = match uefi_boot_volume(&root, &kernel, &loader, true) {
            Ok(_) => panic!("missing sidecar must fail before a boot image exists"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("missing integrity sidecar"), "{err}");
        assert!(!dest.join("boot/sel4.elf").exists());
        assert!(!dest.join("boot/loader.img").exists());
    }

    #[test]
    fn missing_recipe_is_not_current() {
        let tmp = tempfile::tempdir().unwrap();
        let iso = tmp.path().join("lerux_board.iso");
        fs::write(&iso, b"iso").unwrap();
        let kernel = tmp.path().join("sel4.elf");
        fs::write(&kernel, b"kernel").unwrap();
        assert!(!iso_is_current(
            &iso,
            &recipe_path(&iso),
            &recipe_stamp(),
            &[kernel],
        ));
    }

    #[test]
    fn matching_recipe_and_older_input_is_current() {
        let tmp = tempfile::tempdir().unwrap();
        let kernel = tmp.path().join("sel4.elf");
        fs::write(&kernel, b"kernel").unwrap();
        let iso = tmp.path().join("lerux_board.iso");
        fs::write(&iso, b"iso").unwrap();
        let stamp = recipe_stamp();
        fs::write(recipe_path(&iso), &stamp).unwrap();
        assert!(iso_is_current(&iso, &recipe_path(&iso), &stamp, &[kernel]));
    }

    #[test]
    fn newer_input_is_not_current() {
        let tmp = tempfile::tempdir().unwrap();
        let kernel = tmp.path().join("sel4.elf");
        fs::write(&kernel, b"kernel").unwrap();
        let iso = tmp.path().join("lerux_board.iso");
        fs::write(&iso, b"iso").unwrap();
        let stamp = recipe_stamp();
        fs::write(recipe_path(&iso), &stamp).unwrap();
        let later =
            fs::metadata(&iso).unwrap().modified().unwrap() + std::time::Duration::from_secs(2);
        fs::File::options()
            .write(true)
            .open(&kernel)
            .unwrap()
            .set_modified(later)
            .unwrap();
        assert!(!iso_is_current(&iso, &recipe_path(&iso), &stamp, &[kernel],));
    }

    #[test]
    fn recipe_path_is_iso_sibling() {
        let path = recipe_path(Path::new("build/pc_z97_d3h/lerux_pc_z97_d3h.iso"));
        assert_eq!(
            path,
            Path::new("build/pc_z97_d3h/lerux_pc_z97_d3h.iso.boot-recipe")
        );
    }

    #[test]
    fn resolve_output_default_and_guards() {
        let tmp = tempfile::tempdir().unwrap();
        let default = resolve_iso_output(tmp.path(), "build", "pc_z97_d3h", None).unwrap();
        assert_eq!(
            default,
            tmp.path().join("build/pc_z97_d3h/lerux_pc_z97_d3h.iso")
        );
        let swift = resolve_iso_output(tmp.path(), "build", "swift_sfx14_41g", None).unwrap();
        assert_eq!(
            swift,
            tmp.path()
                .join("build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso")
        );

        let custom = resolve_iso_output(
            tmp.path(),
            "build",
            "pc_z97_d3h",
            Some(Path::new("build/pc_z97_d3h/custom.iso")),
        )
        .unwrap();
        assert_eq!(custom, tmp.path().join("build/pc_z97_d3h/custom.iso"));

        let outside = resolve_iso_output(
            tmp.path(),
            "build",
            "pc_z97_d3h",
            Some(Path::new("build/../outside.iso")),
        )
        .unwrap_err()
        .to_string();
        assert!(outside.contains(".."), "{outside}");

        let device = resolve_iso_output(
            tmp.path(),
            "build",
            "pc_z97_d3h",
            Some(Path::new("/dev/sdb")),
        )
        .unwrap_err()
        .to_string();
        assert!(device.contains("/dev"), "{device}");

        let relative = resolve_iso_output(
            tmp.path(),
            "build",
            "pc_z97_d3h",
            Some(Path::new("other.iso")),
        )
        .unwrap_err()
        .to_string();
        assert!(relative.contains("build"), "{relative}");
    }
}
