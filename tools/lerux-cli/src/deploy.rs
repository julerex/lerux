//! Phase 52: one-command host deploy of `loader.img` onto a mounted SD boot partition.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};

/// Copy the board's `loader.img` to a mounted FAT boot directory and print U-Boot steps.
///
/// `dest` is typically the mounted boot partition (e.g. `/media/$USER/boot`).
pub fn deploy_loader(
    root: &Path,
    board: &str,
    build_dir: &str,
    config: &str,
    dest: &Path,
    build_if_missing: bool,
) -> Result<PathBuf> {
    if !dest.is_dir() {
        bail!(
            "deploy destination {} is not a directory (mount the SD FAT boot partition first)",
            dest.display()
        );
    }

    let board_build = root.join(build_dir).join(board);
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

    let dest_loader = dest.join("loader.img");
    fs::copy(&loader, &dest_loader)
        .with_context(|| format!("copy {} → {}", loader.display(), dest_loader.display()))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn deploy_copies_loader() {
        let tmp = tempfile::tempdir().unwrap();
        let board_dir = tmp.path().join("build").join("fake_board");
        fs::create_dir_all(&board_dir).unwrap();
        let loader = board_dir.join("loader.img");
        {
            let mut f = fs::File::create(&loader).unwrap();
            f.write_all(b"fake-loader").unwrap();
        }
        let dest = tmp.path().join("boot");
        fs::create_dir_all(&dest).unwrap();

        // Call inner copy path without full image build: simulate via deploy_loader
        // with a minimal fake tree (board path under tmp as root).
        let out = deploy_loader(tmp.path(), "fake_board", "build", "debug", &dest, false).unwrap();
        assert_eq!(out, dest.join("loader.img"));
        assert_eq!(fs::read(dest.join("loader.img")).unwrap(), b"fake-loader");
        assert!(dest.join("lerux-uboot.txt").is_file());
    }
}
