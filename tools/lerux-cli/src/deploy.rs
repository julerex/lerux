//! Phase 52: one-command host deploy of `loader.img` onto a mounted SD boot partition.
//! Phase 60 Track C: optional SHA-256 sidecar verify before copy.

use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};

/// Copy the board's `loader.img` to a mounted FAT boot directory and print U-Boot steps.
///
/// `dest` must be an absolute directory (typically the mounted boot partition,
/// e.g. `/media/$USER/boot`) with no `..` segments. `--build-dir` and `--board`
/// are resolved under `root` and must not escape it.
///
/// When `verify` is true (default from CLI), the image is checked against
/// `loader.img.sha256` before copy, and the sidecar is copied alongside.
pub fn deploy_loader(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
    dest: &Path,
    build_if_missing: bool,
    verify: bool,
) -> Result<PathBuf> {
    let dest = resolve_dest(dest)?;
    require_single_normal_segment(board, "--board")?;
    let build_root = resolve_under_root(root, Path::new(build_dir), "--build-dir")?;
    let board_build = resolve_under_root(&build_root, Path::new(board), "--board")?;
    let loader = board_build.join("loader.img");
    if !loader.is_file() {
        if build_if_missing {
            println!("==> loader.img missing; building image for {board}…");
            crate::build::image(root, board, build_dir, config)?;
        } else {
            bail!(
                "missing {}; run `BOARD={board} just image` or pass --build",
                loader.display()
            );
        }
    }

    if verify {
        // Sidecar is written by `lerux image` / `lerux digest`. Do not invent a
        // digest from an unknown on-disk image — that would bless tampering.
        crate::image_digest::verify_sidecar(&loader)?;
    }

    let dest_loader = dest.join("loader.img");
    fs::copy(&loader, &dest_loader)
        .with_context(|| format!("copy {} → {}", loader.display(), dest_loader.display()))?;

    let side = crate::image_digest::sidecar_path(&loader);
    if side.is_file() {
        let dest_side = crate::image_digest::sidecar_path(&dest_loader);
        fs::copy(&side, &dest_side)
            .with_context(|| format!("copy {} → {}", side.display(), dest_side.display()))?;
        println!("==> Copied integrity sidecar → {}", dest_side.display());
    }

    // Sidecar with U-Boot commands for operators (and optional paste into uEnv).
    let uboot_txt = dest.join("lerux-uboot.txt");
    let body = uboot_commands(board);
    fs::write(&uboot_txt, body).with_context(|| format!("write {}", uboot_txt.display()))?;

    // Best-effort sync so unplug is safer.
    let _ = Command::new("sync").status();

    let size = fs::metadata(&dest_loader).map(|m| m.len()).unwrap_or(0);
    println!(
        "==> Deployed loader.img ({} bytes) → {}",
        size,
        dest_loader.display()
    );
    println!("==> Wrote U-Boot helper → {}", uboot_txt.display());
    println!();
    print_post_deploy_instructions(board, &dest_loader);
    Ok(dest_loader)
}

fn uboot_commands(board: &str) -> String {
    format!(
        "# lerux U-Boot boot for {board} (Phase 52)\n\
         # At the U-Boot prompt on the serial console:\n\
         fatload mmc 0 0x10000000 loader.img\n\
         go 0x10000000\n\
         #\n\
         # Host golden path after boot (serial free on the host):\n\
         #   LERUX_HW_SERIAL=/dev/ttyUSB0 BOARD={board} just test-hw\n\
         # REPL gate: docs/boards.md (RPi4 workstation install path)\n"
    )
}

fn print_post_deploy_instructions(board: &str, dest_loader: &Path) {
    println!("Next steps:");
    println!("  1. Unmount the SD card safely, insert into the Pi, power on.");
    println!("  2. Serial console: 115200 8N1 on GPIO UART (e.g. screen /dev/ttyUSB0 115200).");
    println!("  3. At U-Boot:");
    println!("       fatload mmc 0 0x10000000 loader.img");
    println!("       go 0x10000000");
    println!("  4. Boot smoke (host, serial not held by screen):");
    println!("       LERUX_HW_SERIAL=/dev/ttyUSB0 BOARD={board} just test-hw");
    println!("  5. Manual REPL checklist: ls, cat /boot.log, ip, fetch, edit /test.txt");
    println!();
    println!("Image on media: {}", dest_loader.display());
    println!("Full procedure: docs/boards.md#rpi4-workstation-install-path-phase-52");
}

fn has_parent_dir(path: &Path) -> bool {
    path.components().any(|c| matches!(c, Component::ParentDir))
}

/// `--dest` is a write target outside the repo (SD mount). Require an absolute
/// path with no `..` so a relative traversal cannot plant boot artifacts.
fn resolve_dest(dest: &Path) -> Result<PathBuf> {
    if !dest.is_absolute() {
        bail!(
            "--dest must be an absolute path to a mounted boot directory (e.g. /media/$USER/boot), got {}",
            dest.display()
        );
    }
    if has_parent_dir(dest) {
        bail!(
            "--dest must not contain '..' path segments: {}",
            dest.display()
        );
    }
    if !dest.is_dir() {
        bail!(
            "deploy destination {} is not a directory (mount the SD FAT boot partition first)",
            dest.display()
        );
    }
    dest.canonicalize()
        .with_context(|| format!("canonicalize {}", dest.display()))
}

fn require_single_normal_segment(name: &str, what: &str) -> Result<()> {
    let path = Path::new(name);
    let mut comps = path.components();
    match (comps.next(), comps.next()) {
        (Some(Component::Normal(seg)), None) if seg == name => Ok(()),
        _ => bail!("{what} must be a single path segment, got {name:?}"),
    }
}

/// Join `user` onto `root` and require the result stays inside the repository.
fn resolve_under_root(root: &Path, user: &Path, what: &str) -> Result<PathBuf> {
    if user.as_os_str().is_empty() {
        bail!("{what} must not be empty");
    }
    if user.is_absolute() {
        bail!(
            "{what} must be a relative path under the repository, got {}",
            user.display()
        );
    }
    if has_parent_dir(user) {
        bail!(
            "{what} must not contain '..' path segments: {}",
            user.display()
        );
    }
    let joined = root.join(user);
    if !joined.exists() {
        return Ok(joined);
    }
    let canon_root = root
        .canonicalize()
        .with_context(|| format!("canonicalize {}", root.display()))?;
    let canon = joined
        .canonicalize()
        .with_context(|| format!("canonicalize {}", joined.display()))?;
    if !canon.starts_with(&canon_root) {
        bail!(
            "{what} resolves outside the repository ({} is not under {})",
            canon.display(),
            canon_root.display()
        );
    }
    Ok(canon)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_loader(tmp: &Path, payload: &[u8]) -> PathBuf {
        let board_dir = tmp.join("build").join("fake_board");
        fs::create_dir_all(&board_dir).unwrap();
        let loader = board_dir.join("loader.img");
        fs::write(&loader, payload).unwrap();
        crate::image_digest::write_sidecar(&loader).unwrap();
        let dest = tmp.join("boot");
        fs::create_dir_all(&dest).unwrap();
        dest
    }

    #[test]
    fn deploy_copies_loader() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = setup_loader(tmp.path(), b"fake-loader");
        let dest_canon = dest.canonicalize().unwrap();

        let out = deploy_loader(
            tmp.path(),
            "fake_board",
            "build",
            "debug",
            &dest,
            false,
            true,
        )
        .unwrap();
        assert_eq!(out, dest_canon.join("loader.img"));
        assert_eq!(
            fs::read(dest_canon.join("loader.img")).unwrap(),
            b"fake-loader"
        );
        assert!(dest_canon.join("lerux-uboot.txt").is_file());
        assert!(dest_canon.join("loader.img.sha256").is_file());
    }

    #[test]
    fn deploy_refuses_tampered_image() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = setup_loader(tmp.path(), b"good");
        fs::write(tmp.path().join("build/fake_board/loader.img"), b"evil").unwrap();
        let err = deploy_loader(
            tmp.path(),
            "fake_board",
            "build",
            "debug",
            &dest,
            false,
            true,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("integrity check failed"), "{err}");
    }

    #[test]
    fn deploy_rejects_relative_dest() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = setup_loader(tmp.path(), b"fake-loader");
        let err = deploy_loader(
            tmp.path(),
            "fake_board",
            "build",
            "debug",
            Path::new("../../../tmp/attacker/boot"),
            false,
            true,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("absolute path"), "{err}");
        assert!(!dest.join("loader.img").exists());
    }

    #[test]
    fn deploy_rejects_dest_parent_dir_segments() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = setup_loader(tmp.path(), b"fake-loader");
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        let sneaky = dest.join("..").join("outside");
        let err = deploy_loader(
            tmp.path(),
            "fake_board",
            "build",
            "debug",
            &sneaky,
            false,
            true,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains(".."), "{err}");
        assert!(!outside.join("loader.img").exists());
        assert!(!dest.join("loader.img").exists());
    }

    #[test]
    fn deploy_rejects_build_dir_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = setup_loader(tmp.path(), b"fake-loader");
        let err = deploy_loader(
            tmp.path(),
            "fake_board",
            "../../../tmp/attacker/build",
            "debug",
            &dest,
            false,
            true,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains(".."), "{err}");
        assert!(!dest.join("loader.img").exists());
    }

    #[test]
    fn deploy_rejects_build_dir_symlink_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = setup_loader(tmp.path(), b"in-tree");
        let outside = tempfile::tempdir().unwrap();
        let evil_board = outside.path().join("fake_board");
        fs::create_dir_all(&evil_board).unwrap();
        fs::write(evil_board.join("loader.img"), b"evil-loader").unwrap();
        crate::image_digest::write_sidecar(&evil_board.join("loader.img")).unwrap();

        let escaped = tmp.path().join("escaped-build");
        std::os::unix::fs::symlink(outside.path(), &escaped).unwrap();

        let err = deploy_loader(
            tmp.path(),
            "fake_board",
            "escaped-build",
            "debug",
            &dest,
            false,
            true,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("outside the repository"), "{err}");
        assert!(!dest.join("loader.img").exists());
    }

    #[test]
    fn deploy_rejects_board_path_segment() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = setup_loader(tmp.path(), b"fake-loader");
        let err = deploy_loader(
            tmp.path(),
            "../fake_board",
            "build",
            "debug",
            &dest,
            false,
            true,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("single path segment"), "{err}");
        assert!(!dest.join("loader.img").exists());
    }
}
