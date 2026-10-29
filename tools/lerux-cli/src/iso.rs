//! Hybrid BIOS+UEFI ISO for x86 Multiboot 2 boards.
//!
//! `lerux iso` packs `sel4_32.elf` and `loader.img` into a hybrid ISO. Limine
//! Multiboot 2-boots that 32-bit kernel from the legacy/CSM entry. The kernel
//! is linked at 1 MiB, so a UEFI entry stops in Limine (no free load address).
//! QEMU x86 smokes pack a separate `lerux-uefi.iso` that boots `sel4.elf`
//! through `mb2-stub`. The image is a file. It does not write a USB device.
//!
//! Limine 12.9.0 is pinned in `deps/versions.toml` and unpacked under
//! `deps/limine/` (gitignored). The ISO is packed by the system `xorriso`
//! from libisoburn, libisofs, and libburn. On Arch that is `libisoburn`.
//! On Debian the package name is `xorriso`.

use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::{
    board::{get_board, load_boards, Board},
    image_digest,
    process::{self, command_on_path, path_str},
};

/// Limine menu embedded in the `just iso` image. Paths are on the boot volume.
///
/// The Multiboot 2 kernel is `sel4_32.elf`, linked at 1 MiB. The module is
/// the Microkit loader. `timeout: 10` keeps the menu on screen for a metal boot.
pub fn limine_conf() -> String {
    "\
# Multiboot 2. `serial` is BIOS-only; the menu is on the screen either way.
timeout: 10
serial: yes
serial_baudrate: 115200

/lerux
    comment: seL4 Microkit
    protocol: multiboot2
    path: boot():/boot/sel4_32.elf
    module_path: boot():/boot/loader.img
"
    .to_string()
}

/// Limine menu for the QEMU UEFI ISO. The kernel is the 32 MiB trampoline.
fn limine_conf_uefi() -> String {
    "\
# Multiboot 2 via mb2-stub (UEFI). `serial` is BIOS-only.
timeout: 10
serial: yes
serial_baudrate: 115200

/lerux
    comment: seL4 Microkit
    protocol: multiboot2
    path: boot():/boot/mb2-stub.elf
    module_path: boot():/boot/sel4.elf
    module_path: boot():/boot/loader.img
"
    .to_string()
}

/// Build `build/<board>/lerux.iso` (or `--output`) for an x86_64 board.
///
/// Builds the Microkit image when `loader.img` or `sel4_32.elf` is missing and
/// `build_if_missing` is set. When `verify` is set, both SHA-256 sidecars must
/// match before anything is packed. Sidecars are not copied onto the ISO.
/// Prints USB flash instructions after packing.
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
        IsoPack {
            output: output.map(Path::to_path_buf),
            build_if_missing,
            verify,
            payload: IsoPayload::Sel4_32,
        },
    )?;
    print_usb_instructions(&out);
    Ok(out)
}

/// Pack `build/<board>/lerux-uefi.iso` for a QEMU smoke (no USB flash text).
///
/// Skips rebuild when `lerux-uefi.iso` is newer than `sel4.elf` and `loader.img`.
///
/// This file is not `lerux.iso`. `just iso` writes that name with `sel4_32.elf`.
pub fn ensure_iso_for_qemu(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
) -> Result<PathBuf> {
    let out = root.join(build_dir).join(board).join("lerux-uefi.iso");
    let board_build = root.join(build_dir).join(board);
    let loader = board_build.join("loader.img");
    let kernel = board_build.join("sel4.elf");
    if out.is_file()
        && loader.is_file()
        && kernel.is_file()
        && iso_inputs_fresh(&out, &[&loader, &kernel])
    {
        return Ok(out);
    }
    pack_iso(
        root,
        board,
        build_dir,
        config,
        IsoPack {
            output: Some(out),
            build_if_missing: false,
            verify: true,
            payload: IsoPayload::UefiStub,
        },
    )
}

#[derive(Clone, Copy)]
enum IsoPayload {
    /// `just iso`: Limine loads `sel4_32.elf` on the BIOS/CSM entry.
    Sel4_32,
    /// QEMU: Limine loads `mb2-stub.elf` under OVMF.
    UefiStub,
}

struct IsoPack {
    output: Option<PathBuf>,
    build_if_missing: bool,
    verify: bool,
    payload: IsoPayload,
}

fn pack_iso(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
    pack: IsoPack,
) -> Result<PathBuf> {
    require_single_normal_segment(board, "--board")?;
    require_relative_under_root(build_dir, "--build-dir")?;
    let boards = load_boards(root)?;
    let board_cfg = get_board(&boards, board)?;
    require_x86_iso_board(board_cfg)?;

    let out = resolve_iso_output(root, build_dir, board, pack.output.as_deref())?;
    let board_build = root.join(build_dir).join(board);
    let loader = board_build.join("loader.img");
    let kernel = match pack.payload {
        IsoPayload::Sel4_32 => board_build.join("sel4_32.elf"),
        IsoPayload::UefiStub => board_build.join("sel4.elf"),
    };
    if !loader.is_file() || !kernel.is_file() {
        if pack.build_if_missing {
            println!("==> boot image missing; building {board}…");
            crate::build::image(root, board, build_dir, config)?;
        } else {
            bail!(
                "missing {} or {}; run `BOARD={board} just image` or omit --no-build",
                loader.display(),
                kernel.display()
            );
        }
    }

    let limine = ensure_limine(root)?;
    let staging = tempfile::tempdir().context("iso staging directory")?;
    match pack.payload {
        IsoPayload::Sel4_32 => stage_iso_root(staging.path(), &kernel, &loader, pack.verify)?,
        IsoPayload::UefiStub => {
            stage_iso_uefi(root, staging.path(), &kernel, &loader, pack.verify)?
        }
    }
    stage_limine_boot(staging.path(), &limine)?;
    let xorriso = require_xorriso()?;
    write_iso(&xorriso, &limine, staging.path(), &out)?;

    let size = fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    println!("==> Wrote ISO ({size} bytes) → {}", out.display());
    Ok(out)
}

fn iso_inputs_fresh(iso: &Path, inputs: &[&Path]) -> bool {
    let Ok(iso_meta) = fs::metadata(iso) else {
        return false;
    };
    let Ok(iso_mtime) = iso_meta.modified() else {
        return false;
    };
    inputs.iter().all(|input| {
        fs::metadata(input)
            .and_then(|m| m.modified())
            .map(|t| t <= iso_mtime)
            .unwrap_or(false)
    })
}

/// Boot an existing ISO in QEMU as a raw disk under SeaBIOS.
///
/// Matches the metal stick: the Limine entry that does not say UEFI loads
/// `sel4_32.elf`. UEFI stops in Limine because that kernel is linked at 1 MiB.
pub fn boot_test(root: &Path, iso: &Path, board: &str, build_dir: &str) -> Result<()> {
    println!("==> ISO boot test (SeaBIOS, raw disk)");
    boot_iso(root, iso, board, build_dir)
}

fn boot_iso(root: &Path, iso: &Path, board: &str, build_dir: &str) -> Result<()> {
    if !iso.is_file() {
        bail!("missing ISO {}", iso.display());
    }
    if !command_on_path("qemu-system-x86_64") {
        bail!("qemu-system-x86_64 is not on PATH");
    }

    let drive = format!("file={},format=raw,if=ide", path_str(iso));
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
        "-drive",
        &drive,
        "-boot",
        "c",
        "-no-reboot",
    ]);
    let mut test = crate::smoke_expects::smoke_test_for_board(root, board)?;
    test.curls.clear();
    let log = root
        .join(build_dir)
        .join("smoke-logs")
        .join(format!("{board}.iso-disk.serial.log"));
    crate::test::run_smoke_with_capture(cmd, &test, Some(&log))
}

/// Attach combined OVMF and the Limine ISO on ICH9 AHCI bus `ide.1`.
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
    match crate::iso_host::find_ovmf() {
        crate::iso_host::OvmfFact::Found(path) => Ok(path),
        crate::iso_host::OvmfFact::BadPath(path) => bail!("OVMF_PATH={path} is not a file"),
        crate::iso_host::OvmfFact::Missing => bail!(
            "OVMF firmware not found. Install the ovmf package or set OVMF_PATH to a combined OVMF.fd.\n\
             Debian/Ubuntu: sudo apt install ovmf\n\
             Arch's edk2-ovmf package ships split CODE and VARS images. This boot test uses one combined OVMF.fd."
        ),
    }
}

fn stage_checked(kernel: &Path, loader: &Path, verify: bool) -> Result<()> {
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
    Ok(())
}

fn stage_iso_root(dest: &Path, kernel: &Path, loader: &Path, verify: bool) -> Result<()> {
    stage_checked(kernel, loader, verify)?;
    copy_boot_file(kernel, &dest.join("boot/sel4_32.elf"))?;
    copy_boot_file(loader, &dest.join("boot/loader.img"))?;
    process::write_file(&dest.join("boot/limine.conf"), &limine_conf())?;
    Ok(())
}

fn stage_iso_uefi(
    root: &Path,
    dest: &Path,
    kernel: &Path,
    loader: &Path,
    verify: bool,
) -> Result<()> {
    stage_checked(kernel, loader, verify)?;
    let stub = build_mb2_stub(root)?;
    copy_boot_file(&stub, &dest.join("boot/mb2-stub.elf"))?;
    copy_boot_file(kernel, &dest.join("boot/sel4.elf"))?;
    copy_boot_file(loader, &dest.join("boot/loader.img"))?;
    process::write_file(&dest.join("boot/limine.conf"), &limine_conf_uefi())?;
    Ok(())
}

/// Build `support/mb2-stub/mb2-stub.elf` with clang (`support/mb2-stub/build.sh`).
pub fn build_mb2_stub(root: &Path) -> Result<PathBuf> {
    let script = root.join("support/mb2-stub/build.sh");
    let out = root.join("support/mb2-stub/mb2-stub.elf");
    if !script.is_file() {
        bail!("missing {}", script.display());
    }
    if !command_on_path("clang") {
        bail!(
            "clang is required to build support/mb2-stub (UEFI Limine trampoline).\n{}",
            crate::iso_host::package_hint(crate::iso_host::current_distro(), "clang", "clang")
        );
    }
    eprintln!("==> Building mb2-stub (UEFI Multiboot 2 trampoline)");
    process::run_checked("bash", &[&path_str(&script), &path_str(&out)])
        .context("build support/mb2-stub")?;
    if !out.is_file() {
        bail!("mb2-stub build did not write {}", out.display());
    }
    Ok(out)
}

struct LimineTool {
    install: PathBuf,
    bios_cd: PathBuf,
    uefi_cd: PathBuf,
    bios_sys: PathBuf,
    bootx64: PathBuf,
}

fn stage_limine_boot(dest: &Path, limine: &LimineTool) -> Result<()> {
    copy_boot_file(&limine.bios_cd, &dest.join("boot/limine-bios-cd.bin"))?;
    copy_boot_file(&limine.uefi_cd, &dest.join("boot/limine-uefi-cd.bin"))?;
    copy_boot_file(&limine.bios_sys, &dest.join("boot/limine-bios.sys"))?;
    copy_boot_file(&limine.bootx64, &dest.join("EFI/BOOT/BOOTX64.EFI"))?;
    Ok(())
}

fn copy_boot_file(src: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::copy(src, dest).with_context(|| format!("copy {} → {}", src.display(), dest.display()))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct VersionsToml {
    limine: PinnedLimine,
}

#[derive(Debug, Deserialize)]
struct PinnedLimine {
    version: String,
    url: String,
    sha256: String,
}

fn load_limine_pin(root: &Path) -> Result<PinnedLimine> {
    let path = root.join("deps/versions.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let parsed: VersionsToml =
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    if parsed.limine.sha256.len() != 64 {
        bail!(
            "deps/versions.toml [limine].sha256 must be 64 hex chars, got {}",
            parsed.limine.sha256.len()
        );
    }
    Ok(parsed.limine)
}

fn ensure_limine(root: &Path) -> Result<LimineTool> {
    let pin = load_limine_pin(root)?;
    let dir = root.join("deps/limine");
    let extracted = dir.join("limine-binary");
    let stamp = dir.join("sha256");
    let ready = stamp_matches(&stamp, &pin.sha256) && limine_tree_ready(&extracted);
    if !ready {
        process::ensure_dir(&dir)?;
        let tarball = dir.join("limine-binary.tar.xz");
        let have_tarball = tarball.is_file() && image_digest::sha256_file(&tarball)? == pin.sha256;
        if !have_tarball {
            if tarball.exists() {
                fs::remove_file(&tarball)
                    .with_context(|| format!("remove {}", tarball.display()))?;
            }
            process::download(&pin.url, &tarball)?;
            let got = image_digest::sha256_file(&tarball)?;
            if got != pin.sha256 {
                let _ = fs::remove_file(&tarball);
                bail!(
                    "Limine tarball sha256 mismatch\n  expected {}\n  actual   {got}",
                    pin.sha256
                );
            }
        }
        if extracted.exists() {
            fs::remove_dir_all(&extracted)
                .with_context(|| format!("remove {}", extracted.display()))?;
        }
        crate::install::extract_tar_xz(&tarball, &dir)?;
        build_limine_tool(&extracted)?;
        fs::write(&stamp, format!("{}\n", pin.sha256))
            .with_context(|| format!("write {}", stamp.display()))?;
        eprintln!(
            "==> Limine {} ready at {}",
            pin.version,
            extracted.display()
        );
    }
    Ok(LimineTool {
        install: extracted.join("limine"),
        bios_cd: extracted.join("limine-bios-cd.bin"),
        uefi_cd: extracted.join("limine-uefi-cd.bin"),
        bios_sys: extracted.join("limine-bios.sys"),
        bootx64: extracted.join("BOOTX64.EFI"),
    })
}

fn stamp_matches(stamp: &Path, sha256: &str) -> bool {
    fs::read_to_string(stamp)
        .map(|text| text.trim() == sha256)
        .unwrap_or(false)
}

fn limine_tree_ready(extracted: &Path) -> bool {
    extracted.join("limine").is_file()
        && extracted.join("limine-bios-cd.bin").is_file()
        && extracted.join("limine-uefi-cd.bin").is_file()
        && extracted.join("limine-bios.sys").is_file()
        && extracted.join("BOOTX64.EFI").is_file()
}

fn build_limine_tool(extracted: &Path) -> Result<()> {
    if !command_on_path("cc") {
        bail!(
            "cc is not on PATH. The Limine host tool is compiled with cc.\n{}",
            crate::iso_host::package_hint(crate::iso_host::current_distro(), "gcc", "gcc")
        );
    }
    let source = extracted.join("limine.c");
    if !source.is_file() {
        bail!("missing {} in the Limine binary release", source.display());
    }
    eprintln!("==> Building Limine host tool");
    process::run_checked(
        "cc",
        &[
            "-std=c99",
            "-D_FILE_OFFSET_BITS=64",
            "-O2",
            &format!("-I{}", extracted.display()),
            &path_str(&source),
            "-o",
            &path_str(&extracted.join("limine")),
        ],
    )
    .context("compile Limine host tool (limine.c)")?;
    Ok(())
}

/// `xorriso` from the system libisoburn package.
///
/// Arch installs that program with `libisoburn`, which links libisofs and
/// libburn. Debian installs it with the `xorriso` package.
fn require_xorriso() -> Result<PathBuf> {
    match which::which("xorriso") {
        Ok(bin) => Ok(bin),
        Err(_) => bail!(
            "xorriso is not on PATH.\n{}",
            crate::iso_host::package_hint(
                crate::iso_host::current_distro(),
                "libisoburn",
                "xorriso",
            )
        ),
    }
}

fn write_iso(xorriso: &Path, limine: &LimineTool, staging: &Path, out: &Path) -> Result<()> {
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let mut partial_name = out
        .file_name()
        .context("ISO output has no file name")?
        .to_os_string();
    partial_name.push(".partial");
    let partial = out.with_file_name(partial_name);
    if partial.exists() {
        fs::remove_file(&partial).with_context(|| format!("remove {}", partial.display()))?;
    }

    println!("==> xorriso → {}", out.display());
    let mut cmd = Command::new(xorriso);
    cmd.args([
        "-as",
        "mkisofs",
        "-R",
        "-r",
        "-J",
        "-b",
        "boot/limine-bios-cd.bin",
        "-no-emul-boot",
        "-boot-load-size",
        "4",
        "-boot-info-table",
        "-hfsplus",
        "-apm-block-size",
        "2048",
        "--efi-boot",
        "boot/limine-uefi-cd.bin",
        "-efi-boot-part",
        "--efi-boot-image",
        "--protective-msdos-label",
    ]);
    cmd.arg(staging);
    cmd.arg("-o");
    cmd.arg(&partial);
    let status = cmd.status().context("failed to run xorriso")?;
    if !status.success() {
        let _ = fs::remove_file(&partial);
        bail!(
            "xorriso exited with {status}\n{}",
            crate::iso_host::package_hint(
                crate::iso_host::current_distro(),
                "libisoburn",
                "xorriso",
            )
        );
    }

    println!("==> limine bios-install");
    let status = Command::new(&limine.install)
        .arg("bios-install")
        .arg(&partial)
        .status()
        .context("failed to run limine bios-install")?;
    if !status.success() {
        let _ = fs::remove_file(&partial);
        bail!("limine bios-install exited with {status}");
    }

    fs::rename(&partial, out)
        .with_context(|| format!("rename {} → {}", partial.display(), out.display()))?;
    Ok(())
}

fn print_usb_instructions(iso: &Path) {
    println!();
    println!("USB flash drive (this command does not write a disk):");
    println!("  lsblk");
    println!(
        "  sudo dd if={} of=/dev/sdX bs=4M status=progress conv=fsync",
        iso.display()
    );
    println!(
        "Replace sdX with the flash drive from lsblk. Do not use sda (Ubuntu SSD) or sdb (data disk)."
    );
    println!(
        "Firmware boot menu (F12) → the USB entry that does not say UEFI. Enable CSM if that entry is missing."
    );
    println!("The UEFI entry stops in Limine: sel4_32.elf is linked at 1 MiB.");
    println!("Limine waits 10 seconds, then boots /lerux. A key during that wait holds the menu.");
    println!("Screen: blue background, \"hello lerux\" on the first line, then \"1 browser\" and \"2 maths\".");
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
        return Ok(root.join(build_dir).join(board).join("lerux.iso"));
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
    fn limine_conf_is_multiboot2() {
        let cfg = limine_conf();
        assert!(cfg.contains("protocol: multiboot2"), "{cfg}");
        assert!(cfg.contains("timeout: 10"), "{cfg}");
        assert!(cfg.contains("boot():/boot/sel4_32.elf"), "{cfg}");
        assert!(!cfg.contains("mb2-stub.elf"), "{cfg}");
        assert!(
            cfg.contains("module_path: boot():/boot/loader.img"),
            "{cfg}"
        );
        let uefi = limine_conf_uefi();
        assert!(uefi.contains("boot():/boot/mb2-stub.elf"), "{uefi}");
        assert!(
            uefi.contains("module_path: boot():/boot/sel4.elf"),
            "{uefi}"
        );
    }

    #[test]
    fn ovmf_iso_boot_uses_ide_bus_one() {
        let mut cmd = Command::new("qemu-system-x86_64");
        append_ovmf_iso_boot(
            &mut cmd,
            Path::new("/usr/share/ovmf/OVMF.fd"),
            Path::new("lerux.iso"),
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
    fn stage_copies_sel4_32_and_loader_without_sidecars() {
        let tmp = tempfile::tempdir().unwrap();
        let kernel = tmp.path().join("sel4_32.elf");
        let loader = tmp.path().join("loader.img");
        fs::write(&kernel, b"kernel-bytes").unwrap();
        fs::write(&loader, b"loader-bytes").unwrap();
        image_digest::write_sidecar(&kernel).unwrap();
        image_digest::write_sidecar(&loader).unwrap();

        let dest = tmp.path().join("iso-root");
        stage_iso_root(&dest, &kernel, &loader, true).unwrap();

        assert!(!dest.join("boot/mb2-stub.elf").exists());
        assert_eq!(
            fs::read(dest.join("boot/sel4_32.elf")).unwrap(),
            b"kernel-bytes"
        );
        assert_eq!(
            fs::read(dest.join("boot/loader.img")).unwrap(),
            b"loader-bytes"
        );
        assert!(!dest.join("boot/sel4_32.elf.sha256").exists());
        assert!(!dest.join("boot/loader.img.sha256").exists());
        let cfg = fs::read_to_string(dest.join("boot/limine.conf")).unwrap();
        assert!(cfg.contains("sel4_32.elf"), "{cfg}");
        assert!(
            cfg.contains("module_path: boot():/boot/loader.img"),
            "{cfg}"
        );
    }

    #[test]
    fn stage_refuses_missing_sidecar() {
        let tmp = tempfile::tempdir().unwrap();
        let kernel = tmp.path().join("sel4_32.elf");
        let loader = tmp.path().join("loader.img");
        fs::write(&kernel, b"kernel-bytes").unwrap();
        fs::write(&loader, b"loader-bytes").unwrap();
        image_digest::write_sidecar(&loader).unwrap();

        let dest = tmp.path().join("iso-root");
        let err = stage_iso_root(&dest, &kernel, &loader, true)
            .unwrap_err()
            .to_string();
        assert!(err.contains("missing integrity sidecar"), "{err}");
        assert!(!dest.join("boot/sel4_32.elf").exists());
        assert!(!dest.join("boot/loader.img").exists());
    }

    #[test]
    fn resolve_output_default_and_guards() {
        let tmp = tempfile::tempdir().unwrap();
        let default = resolve_iso_output(tmp.path(), "build", "pc_z97_d3h", None).unwrap();
        assert_eq!(default, tmp.path().join("build/pc_z97_d3h/lerux.iso"));

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
