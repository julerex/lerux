#!/usr/bin/env bash
# Print PATH prefix for lerux host tools (ARM GCC, QEMU) — one directory per line, last line is the joined PATH.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
paths=()

if ! command -v aarch64-none-elf-gcc >/dev/null 2>&1; then
    if arm_bin="$(bash "${root}/scripts/install-arm-toolchain.sh" 2>/dev/null)"; then
        paths+=("${arm_bin}")
    fi
fi

if ! command -v qemu-system-aarch64 >/dev/null 2>&1; then
    if qemu_bin="$(bash "${root}/scripts/install-qemu.sh" 2>/dev/null)"; then
        paths+=("${qemu_bin}")
    fi
fi

if ! command -v qemu-system-riscv64 >/dev/null 2>&1; then
    if qemu_riscv_bin="$(bash "${root}/scripts/install-qemu-riscv.sh" 2>/dev/null)"; then
        paths+=("${qemu_riscv_bin}")
    fi
fi

if ! command -v riscv64-unknown-elf-gcc >/dev/null 2>&1; then
    if riscv_bin="$(bash "${root}/scripts/install-riscv-toolchain.sh" 2>/dev/null)"; then
        paths+=("${riscv_bin}")
    fi
fi

joined="${PATH}"
for p in "${paths[@]}"; do
    joined="${p}:${joined}"
done

echo "${joined}"