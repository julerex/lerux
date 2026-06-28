#!/usr/bin/env bash
# Build the Microkit SDK from synced sources (compiles seL4 kernel per board).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
workspace="${root}/deps/workspace"
microkit="${workspace}/microkit"
sel4="${workspace}/seL4"

# Boards to build (comma-separated). Default: aarch64 QEMU virt only for fast bring-up.
boards="${MICROKIT_BOARDS:-qemu_virt_aarch64}"
configs="${MICROKIT_CONFIGS:-debug}"

if [[ ! -d "${microkit}" || ! -d "${sel4}" ]]; then
    echo "error: run scripts/fetch.sh first" >&2
    exit 1
fi

if ! command -v aarch64-none-elf-gcc >/dev/null 2>&1; then
    echo "==> Installing ARM GNU toolchain into deps/toolchains/"
    arm_bin="$(bash "${root}/scripts/install-arm-toolchain.sh")"
    export PATH="${arm_bin}:${PATH}"
fi

if ! command -v aarch64-none-elf-gcc >/dev/null 2>&1; then
    echo "error: aarch64-none-elf-gcc not found after install attempt" >&2
    echo "Run: just fetch-sdk  (prebuilt SDK fallback)" >&2
    exit 1
fi

for tool in qemu-system-aarch64 cmake ninja python3; do
    if ! command -v "${tool}" >/dev/null 2>&1; then
        echo "error: ${tool} not found in PATH" >&2
        [[ "${tool}" == "qemu-system-aarch64" ]] && echo "Install: sudo apt install qemu-system-arm" >&2
        exit 1
    fi
done

# Microkit SDK build compiles its tool for x86_64-unknown-linux-musl (default rustup toolchain).
if command -v rustup >/dev/null 2>&1; then
    rustup target add x86_64-unknown-linux-musl
    # rust-sel4 initialiser in SDK build may need nightly
    rustup toolchain install nightly-2026-03-18 -c rust-src 2>/dev/null || true
    rustup target add x86_64-unknown-linux-musl --toolchain nightly-2026-03-18 2>/dev/null || true
fi

cd "${microkit}"

# Avoid inheriting a parent workspace's CARGO_TARGET_DIR (e.g. lerux/.cargo/config.toml).
unset CARGO_TARGET_DIR

if [[ ! -d pyenv ]]; then
    python3 -m venv pyenv
    ./pyenv/bin/pip install --upgrade pip setuptools wheel
    ./pyenv/bin/pip install -r requirements.txt
fi

./pyenv/bin/python build_sdk.py \
    --sel4="${sel4}" \
    --skip-docs \
    --skip-tar \
    --boards "${boards}" \
    --configs "${configs}"

sdk="$(find "${microkit}/release" -maxdepth 1 -type d -name 'microkit-sdk-*' | sort | tail -1)"
if [[ -z "${sdk}" ]]; then
    echo "error: SDK build produced no microkit-sdk-* directory" >&2
    exit 1
fi

echo "${sdk}" > "${root}/deps/.sdk-path"
echo "==> Microkit SDK: ${sdk}"