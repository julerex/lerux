//! Rescue and diagnostic lines for the shell.
//!
//! Each name is a built-in. A missing device is that name plus `err`. The
//! parsers read a buffer the caller already has. They do not write a volume,
//! and they do not unlock a locked volume.
//!
//! `SMART` is Self-Monitoring, Analysis and Reporting Technology. `FAT` is the
//! File Allocation Table. `NTFS` is the New Technology File System. `GPT` is a
//! globally unique identifier partition table. `DNS` is the Domain Name System.
//! `NVMe` is Non-Volatile Memory Express.

#![cfg_attr(not(test), no_std)]

mod parse;
mod step;
mod text;

pub use parse::{
    allow_copy, ata_smart_line, dns_ipv4, endpoint, fat_boot_ok, format_count, guess_gpt, identify,
    memtest, ntfs_boot, nvme_celsius, nvme_model, qemu_fat_boot, read_gpt, CopyPolicy, Endpoint,
    FsKind, Gpt,
};
pub use step::{ethtool_line, owns, step, Flags, Role, Step, View};

#[cfg(test)]
mod tests;
