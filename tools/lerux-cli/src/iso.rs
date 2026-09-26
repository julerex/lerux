//! Hybrid BIOS+UEFI ISO for x86 Multiboot 2 boards.
//!
//! `lerux iso` packs `sel4_32.elf` and `loader.img` into a hybrid ISO. Limine
//! Multiboot 2-boots that 32-bit kernel from the legacy/CSM entry. The kernel
//! is linked at 1MB, so a UEFI entry stops in Limine (no free load address).
//! The image is a file. It does not write a USB device.
//!
//! Limine 12.9.0 is pinned in `deps/versions.toml` and unpacked under
//! `deps/limine/` (gitignored). `xorriso` comes from `PATH` when installed,
//! otherwise from Ubuntu packages unpacked under `deps/toolchains/xorriso/`.

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

/// Limine menu embedded in the ISO. Paths are on the boot volume.
pub fn limine_conf() -> String {
    "\
# Multiboot 2. `serial` is BIOS-only; the menu is on the screen either way.
timeout: 3
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

/// Build `build/<board>/lerux.iso` (or `--output`) for an x86_64 board.
///
/// Builds the Microkit image when `loader.img` or `sel4_32.elf` is missing and
/// `build_if_missing` is set. When `verify` is set, both SHA-256 sidecars must
/// match before anything is packed. Sidecars are not copied onto the ISO.
pub fn build_iso(
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
    let kernel = board_build.join("sel4_32.elf");
    if !loader.is_file() || !kernel.is_file() {
        if build_if_missing {
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
    stage_iso_root(staging.path(), &kernel, &loader, verify)?;
    stage_limine_boot(staging.path(), &limine)?;
    let xorriso = ensure_xorriso(root)?;
    write_iso(&xorriso, &limine, staging.path(), &out)?;

    let size = fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    println!("==> Wrote ISO ({size} bytes) → {}", out.display());
    print_usb_instructions(&out);
    Ok(out)
}

/// Boot an existing ISO in QEMU as a raw disk, the same shape as `dd` onto a stick.
///
/// Matches `support/smoke-expects.toml` for `board` under SeaBIOS. Local check;
/// not a CI job. UEFI is not tested: `sel4_32.elf` is linked at 1MB, Limine
/// panics with "Could not find viable load address" on OVMF, and UEFI GRUB
/// places `loader.img` inside that image so seL4 asserts in `try_boot_sys`.
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

    // Guest CPU matches `qemu::x86_command` so the ISO smoke exercises the same
    // feature set as `-kernel sel4_32.elf`. `if=ide` is the dd-to-stick shape.
    let drive = format!("file={},format=raw,if=ide", iso.display());
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

fn stage_iso_root(dest: &Path, kernel: &Path, loader: &Path, verify: bool) -> Result<()> {
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
    copy_boot_file(kernel, &dest.join("boot/sel4_32.elf"))?;
    copy_boot_file(loader, &dest.join("boot/loader.img"))?;
    process::write_file(&dest.join("boot/limine.conf"), &limine_conf())?;
    Ok(())
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
        bail!("cc is not on PATH; it is required to build the Limine host tool");
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

struct XorrisoTool {
    /// Binary invoked as `xorriso -as mkisofs`.
    bin: PathBuf,
    /// Extra library directory for a locally unpacked `xorriso`.
    lib_dir: Option<PathBuf>,
}

fn ensure_xorriso(root: &Path) -> Result<XorrisoTool> {
    if let Ok(bin) = which::which("xorriso") {
        return Ok(XorrisoTool { bin, lib_dir: None });
    }
    if std::env::consts::ARCH != "x86_64" {
        bail!("xorriso is not on PATH. Install the xorriso package and re-run.");
    }
    let prefix = root.join("deps/toolchains/xorriso");
    if !xorriso_prefix_ready(&prefix) {
        unpack_xorriso_debs(&prefix)?;
    }
    Ok(XorrisoTool {
        bin: prefix.join("usr/bin/xorriso"),
        lib_dir: Some(prefix.join("usr/lib/x86_64-linux-gnu")),
    })
}

fn xorriso_prefix_ready(prefix: &Path) -> bool {
    let lib = prefix.join("usr/lib/x86_64-linux-gnu");
    prefix.join("usr/bin/xorriso").is_file()
        && lib.join("libisoburn.so.1").is_file()
        && lib.join("libburn.so.4").is_file()
        && lib.join("libisofs.so.6").is_file()
        && lib.join("libjte.so.2").is_file()
}

fn unpack_xorriso_debs(prefix: &Path) -> Result<()> {
    // Host archive names for Ubuntu 22.04. `apt_deb_url` prefers the live
    // `apt-cache` filename when the package index has moved on.
    let packages = [
        (
            "xorriso",
            "http://archive.ubuntu.com/ubuntu/pool/universe/libi/libisoburn/xorriso_1.5.4-2_amd64.deb",
        ),
        (
            "libisoburn1",
            "http://archive.ubuntu.com/ubuntu/pool/universe/libi/libisoburn/libisoburn1_1.5.4-2_amd64.deb",
        ),
        (
            "libburn4",
            "http://archive.ubuntu.com/ubuntu/pool/universe/libb/libburn/libburn4_1.5.4-1_amd64.deb",
        ),
        (
            "libisofs6",
            "http://archive.ubuntu.com/ubuntu/pool/universe/libi/libisofs/libisofs6_1.5.4-1_amd64.deb",
        ),
        (
            "libjte2",
            "http://archive.ubuntu.com/ubuntu/pool/universe/j/jigit/libjte2_1.22-3build1_amd64.deb",
        ),
    ];
    if prefix.exists() {
        fs::remove_dir_all(prefix).with_context(|| format!("remove {}", prefix.display()))?;
    }
    process::ensure_dir(prefix)?;
    for (package, fallback) in packages {
        let url = process::apt_deb_url(package, fallback);
        let tmp = tempfile::NamedTempFile::new().context("deb temp file")?;
        process::download(&url, tmp.path())?;
        process::run_checked(
            "dpkg-deb",
            &[
                "-x",
                &tmp.path().to_string_lossy(),
                &prefix.to_string_lossy(),
            ],
        )
        .with_context(|| format!("unpack {package}"))?;
    }
    if !xorriso_prefix_ready(prefix) {
        bail!(
            "xorriso unpack incomplete under {} (package xorriso and its libraries)",
            prefix.display()
        );
    }
    eprintln!("==> xorriso unpacked at {}", prefix.display());
    Ok(())
}

fn write_iso(xorriso: &XorrisoTool, limine: &LimineTool, staging: &Path, out: &Path) -> Result<()> {
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
    let mut cmd = Command::new(&xorriso.bin);
    if let Some(lib) = &xorriso.lib_dir {
        let mut ld = path_str(lib);
        if let Some(existing) = std::env::var_os("LD_LIBRARY_PATH") {
            ld.push(':');
            ld.push_str(&existing.to_string_lossy());
        }
        cmd.env("LD_LIBRARY_PATH", ld);
    }
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
        bail!("xorriso exited with {status} (package xorriso)");
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
    println!("The UEFI entry stops in Limine: this kernel is linked at 1MB.");
    println!("Screen: blue background, \"hello lerux\" on the first line, then \"lerux>\".");
    println!("Keyboard: PS/2 in the rear combo port. USB works only if firmware legacy emulation is still on.");
    println!("Commands: echo, help, pwd, history, calc, qos, clear. Others print \"unavailable\".");
    println!("Console: COM1 115200 8N1 on the COMA header.");
    println!("Expect: lerux-shell: prompt");
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
        assert!(cfg.contains("boot():/boot/sel4_32.elf"), "{cfg}");
        assert!(
            cfg.contains("module_path: boot():/boot/loader.img"),
            "{cfg}"
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
    fn stage_copies_kernel_and_loader_without_sidecars() {
        let tmp = tempfile::tempdir().unwrap();
        let kernel = tmp.path().join("sel4_32.elf");
        let loader = tmp.path().join("loader.img");
        fs::write(&kernel, b"kernel-bytes").unwrap();
        fs::write(&loader, b"loader-bytes").unwrap();
        image_digest::write_sidecar(&kernel).unwrap();
        image_digest::write_sidecar(&loader).unwrap();

        let dest = tmp.path().join("iso-root");
        stage_iso_root(&dest, &kernel, &loader, true).unwrap();

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
