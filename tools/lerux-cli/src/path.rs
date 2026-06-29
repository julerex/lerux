use std::path::Path;

use crate::install::{
    install_arm_toolchain, install_qemu_aarch64, install_qemu_riscv64, install_riscv_toolchain,
};
use crate::process::command_on_path;

pub fn host_path(root: &Path) -> String {
    let mut paths = Vec::new();

    if !command_on_path("aarch64-none-elf-gcc") {
        if let Ok(bin) = install_arm_toolchain(root) {
            paths.push(bin);
        }
    }

    if !command_on_path("qemu-system-aarch64") {
        if let Ok(bin) = install_qemu_aarch64(root) {
            paths.push(bin);
        }
    }

    if !command_on_path("qemu-system-riscv64") {
        if let Ok(bin) = install_qemu_riscv64(root) {
            paths.push(bin);
        }
    }

    if !command_on_path("riscv64-unknown-elf-gcc") {
        if let Ok(bin) = install_riscv_toolchain(root) {
            paths.push(bin);
        }
    }

    let mut joined = std::env::var("PATH").unwrap_or_default();
    for p in paths.into_iter().rev() {
        joined = format!("{}:{}", p.display(), joined);
    }
    joined
}