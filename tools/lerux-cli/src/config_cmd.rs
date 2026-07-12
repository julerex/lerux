//! Phase 54: host-side config schema helpers and optional disk preseed.

use std::{
    fs::OpenOptions,
    io::{Seek, SeekFrom, Write},
    path::Path,
};

use anyhow::{bail, Context, Result};
use lerux_fs::{
    alloc_contiguous, encode_dir_entry, encode_free_map_fresh, encode_superblock, make_entry,
    Superblock, DATA_START_LBA, FREE_MAP_LBA, ROOT_DIR_LBA, SUPERBLOCK_LBA, TOTAL_LBAS,
};
use lerux_interface_types::SECTOR_SIZE;

/// QEMU-friendly defaults (match supervisor seed).
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
        r#"lerux config schema (Phase 54)

Keys live under /config/ on LERUXFS2 (secret.* → /config/secrets/).
See docs/config.md for the full table.

Well-known keys:
  hostname          short name (default: lerux)
  net.mode          dhcp | static
  net.ip            dotted IPv4 (static fallback)
  net.gateway       dotted IPv4
  net.dns           dotted IPv4
  net.prefix        1-32
  log.level         error | warn | info | debug
  log.rotate        0 | 1  (rotate /boot.log → /boot.log.1)
  boot.seeded       1 after first seed (do not clear unless wiping)
  secret.<name>     secret material (value hidden in config list)

Shell:
  config list | get <k> | set <k> <v> | del <k>
  hostname

Host:
  lerux config schema
  lerux config defaults
  lerux config seed-disk   # write LERUXFS2 + defaults onto support/disk.img
"#
    );
}

pub fn print_defaults() {
    for (k, v) in QEMU_DEFAULTS {
        println!("{k}={v}");
    }
}

/// Format `support/disk.img` (or path) with LERUXFS2 and write default config files.
pub fn seed_disk(root: &Path, disk_path: Option<&Path>) -> Result<()> {
    let disk = disk_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("support/disk.img"));
    if !disk.is_file() {
        crate::disk_img::disk_img(root)?;
    }

    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&disk)
        .with_context(|| format!("open {}", disk.display()))?;

    let sb = Superblock::new();
    let mut sector = [0u8; SECTOR_SIZE];
    encode_superblock(&mut sector, &sb);
    write_lba(&mut f, SUPERBLOCK_LBA, &sector)?;

    let mut map = [0u8; SECTOR_SIZE];
    encode_free_map_fresh(&mut map, &sb);
    // Reserve space for config files as we allocate.
    let mut dir = [0u8; SECTOR_SIZE];
    // /config directory
    let config_lba = alloc_contiguous(&mut map, &sb, 1).context("alloc /config dir")?;
    let secrets_lba = alloc_contiguous(&mut map, &sb, 1).context("alloc secrets dir")?;
    let mut config_dir = [0u8; SECTOR_SIZE];
    let secrets_dir = [0u8; SECTOR_SIZE];

    // root: config (dir)
    encode_dir_entry(&mut dir, 0, &make_entry(b"config", config_lba, 0, true));

    let mut slot = 0usize;
    for (key, val) in QEMU_DEFAULTS {
        if key.starts_with("secret.") {
            continue;
        }
        let data_lba =
            alloc_contiguous(&mut map, &sb, 1).with_context(|| format!("alloc {key}"))?;
        let mut data = [0u8; SECTOR_SIZE];
        let n = val.len().min(SECTOR_SIZE);
        data[..n].copy_from_slice(val.as_bytes());
        write_lba(&mut f, data_lba, &data)?;
        encode_dir_entry(
            &mut config_dir,
            slot,
            &make_entry(key.as_bytes(), data_lba, n as u32, false),
        );
        slot += 1;
        if slot >= 16 {
            bail!("too many config keys for single dir sector");
        }
    }

    write_lba(&mut f, config_lba, &config_dir)?;
    write_lba(&mut f, secrets_lba, &secrets_dir)?;
    // Put secrets dir entry inside config/
    encode_dir_entry(
        &mut config_dir,
        slot,
        &make_entry(b"secrets", secrets_lba, 0, true),
    );
    write_lba(&mut f, config_lba, &config_dir)?;

    write_lba(&mut f, ROOT_DIR_LBA, &dir)?;
    write_lba(&mut f, FREE_MAP_LBA, &map)?;

    // Preserve MBR signature at LBA 0.
    println!(
        "==> seeded LERUXFS2 + {} config keys on {} (data starts LBA {DATA_START_LBA}, map covers {TOTAL_LBAS} LBAs)",
        QEMU_DEFAULTS.len(),
        disk.display()
    );
    println!("    Guest will log: config already seeded (boot.seeded=1)");
    Ok(())
}

fn write_lba(f: &mut std::fs::File, lba: u32, sector: &[u8; SECTOR_SIZE]) -> Result<()> {
    let off = (lba as u64) * (SECTOR_SIZE as u64);
    f.seek(SeekFrom::Start(off))
        .with_context(|| format!("seek LBA {lba}"))?;
    f.write_all(sector)
        .with_context(|| format!("write LBA {lba}"))?;
    Ok(())
}
