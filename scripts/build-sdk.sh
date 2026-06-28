#!/usr/bin/env bash
# Build the Microkit SDK from synced sources (compiles seL4 kernel per board).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
workspace="${root}/deps/workspace"
microkit="${workspace}/microkit"
sel4="${workspace}/seL4"

if [[ ! -d "${microkit}" || ! -d "${sel4}" ]]; then
    echo "error: run scripts/fetch.sh first" >&2
    exit 1
fi

if ! command -v aarch64-none-elf-gcc >/dev/null 2>&1; then
    echo "error: aarch64-none-elf-gcc not found in PATH" >&2
    echo "Install ARM GNU toolchain 12.2.Rel1 (see microkit DEVELOPER.md) or run: just fetch-sdk" >&2
    exit 1
fi

# Microkit SDK build compiles its tool for x86_64-unknown-linux-musl.
if command -v rustup >/dev/null 2>&1; then
    rustup toolchain install nightly-2026-03-18 -c rust-src 2>/dev/null || true
    rustup target add x86_64-unknown-linux-musl --toolchain nightly-2026-03-18 2>/dev/null || \
        rustup target add x86_64-unknown-linux-musl
fi

cd "${microkit}"

# Avoid inheriting a parent workspace's CARGO_TARGET_DIR (e.g. lerux/.cargo/config.toml).
unset CARGO_TARGET_DIR

if [[ ! -d pyenv ]]; then
    python3 -m venv pyenv
    ./pyenv/bin/pip install --upgrade pip setuptools wheel
    ./pyenv/bin/pip install -r requirements.txt
fi

./pyenv/bin/python build_sdk.py --sel4="${sel4}" --skip-docs

sdk="$(find "${microkit}/release" -maxdepth 1 -type d -name 'microkit-sdk-*' | sort | tail -1)"
if [[ -z "${sdk}" ]]; then
    echo "error: SDK build produced no microkit-sdk-* directory" >&2
    exit 1
fi

echo "${sdk}" > "${root}/deps/.sdk-path"
echo "==> Microkit SDK: ${sdk}"