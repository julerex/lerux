use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::process::{ensure_dir, run_checked};

pub fn disk_img(root: &Path) -> Result<()> {
    let disk = root.join("support/disk.img");
    ensure_dir(&disk.parent().unwrap())?;
    run_checked("qemu-img", &["create", "-f", "raw", &disk.to_string_lossy(), "4M"])?;

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