use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::process::{ensure_dir, run_checked};

pub fn disk_img(root: &Path) -> Result<()> {
    write_disk_img(root, false)
}

/// Replace `support/disk.img`. Scripted smokes call this so a previous local
/// format cannot make `mkdir` fail with "already exists".
pub(crate) fn fresh_disk_img(root: &Path) -> Result<()> {
    write_disk_img(root, true)
}

fn write_disk_img(root: &Path, replace: bool) -> Result<()> {
    let disk = root.join("support/disk.img");
    if replace && disk.is_file() {
        std::fs::remove_file(&disk).with_context(|| format!("remove {}", disk.display()))?;
    }
    let parent = disk
        .parent()
        .context("support/disk.img has no parent directory")?;
    ensure_dir(parent)?;
    run_checked(
        "qemu-img",
        &["create", "-f", "raw", &disk.to_string_lossy(), "4M"],
    )?;

    let status = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!(
            "printf '\\x55\\xAA' | dd of='{}' bs=1 seek=510 conv=notrunc status=none",
            disk.display()
        ))
        .status()
        .context("write MBR signature")?;
    if !status.success() {
        bail!("failed to write MBR signature");
    }
    Ok(())
}
