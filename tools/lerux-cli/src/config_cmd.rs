//! Host-side config schema helpers.

use std::path::Path;

use anyhow::Result;

/// QEMU-friendly defaults for documentation (device-framework images seed config on disk).
const QEMU_DEFAULTS: &[(&str, &str)] = &[
    ("net.mode", "dhcp"),
    ("net.ip", "10.0.2.15"),
    ("net.gateway", "10.0.2.2"),
    ("net.dns", "10.0.2.3"),
    ("net.prefix", "24"),
    ("hostname", "lerux"),
    ("log.level", "info"),
    ("log.rotate", "1"),
    ("boot.seeded", "1"),
];

pub fn print_schema() {
    println!(
        r#"lerux config schema

Well-known keys are documented in docs/config.md. Device-framework shells read
files such as /config/hostname from the File Allocation Table volume.

Host:
  lerux config schema
  lerux config defaults
  lerux config seed-disk   # ensure support/disk.img exists (raw MBR; no LERUXFS)
  lerux disk-img           # same raw image helper
"#
    );
}

pub fn print_defaults() {
    for (k, v) in QEMU_DEFAULTS {
        println!("{k}={v}");
    }
}

/// Ensure `support/disk.img` exists with an MBR boot signature (no LERUXFS format).
pub fn seed_disk(root: &Path, _disk_path: Option<&Path>) -> Result<()> {
    crate::disk_img::disk_img(root)?;
    println!("==> support/disk.img ready (raw sector image; guest FAT server formats when needed)");
    Ok(())
}
