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

needs_arm=false
needs_riscv=false
needs_x86=false
IFS=',' read -ra board_list <<< "${boards}"
for b in "${board_list[@]}"; do
    case "${b}" in
        qemu_virt_aarch64|*aarch64*)
            needs_arm=true
            ;;
        qemu_virt_riscv64|*riscv*)
            needs_riscv=true
            ;;
        x86_64_generic|x86_64_generic_vtx|*x86*)
            needs_x86=true
            ;;
    esac
done

if [[ "${needs_arm}" == true ]]; then
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
fi

riscv_gcc_ok() {
    command -v riscv64-unknown-elf-gcc >/dev/null 2>&1 || return 1
    local version
    version="$(riscv64-unknown-elf-gcc -dumpversion | cut -d. -f1)"
    [[ "${version}" -ge 13 ]]
}

if [[ "${needs_riscv}" == true ]] && ! riscv_gcc_ok; then
    echo "==> Installing RISC-V GNU toolchain into deps/toolchains/" >&2
    riscv_bin="$(bash "${root}/scripts/install-riscv-toolchain.sh")"
    export PATH="${riscv_bin}:${PATH}"
fi

if [[ "${needs_riscv}" == true ]] && ! riscv_gcc_ok; then
    echo "error: riscv64-unknown-elf-gcc 13+ not found after install attempt" >&2
    exit 1
fi

if [[ "${needs_x86}" == true ]] && ! command -v x86_64-linux-gnu-gcc >/dev/null 2>&1; then
    echo "error: x86_64-linux-gnu-gcc required for x86_64 boards" >&2
    exit 1
fi

if ! command -v qemu-system-aarch64 >/dev/null 2>&1; then
    echo "==> Installing QEMU (aarch64) into deps/toolchains/" >&2
    qemu_bin="$(bash "${root}/scripts/install-qemu.sh")"
    export PATH="${qemu_bin}:${PATH}"
fi

if [[ "${needs_riscv}" == true ]] && ! command -v qemu-system-riscv64 >/dev/null 2>&1; then
    echo "==> Installing QEMU (riscv64) into deps/toolchains/" >&2
    qemu_riscv_bin="$(bash "${root}/scripts/install-qemu-riscv.sh")"
    export PATH="${qemu_riscv_bin}:${PATH}"
fi

if ! command -v dtc >/dev/null 2>&1; then
    echo "==> Installing device-tree-compiler into deps/toolchains/" >&2
    dtc_bin="$(bash "${root}/scripts/install-dtc.sh")"
    export PATH="${dtc_bin}:${PATH}"
fi

if ! command -v xmllint >/dev/null 2>&1; then
    echo "==> Installing xmllint into deps/toolchains/" >&2
    xml_bin="$(bash "${root}/scripts/install-xmllint.sh")"
    export PATH="${xml_bin}:${PATH}"
fi

required_tools=(dtc xmllint cmake ninja python3)
if [[ "${needs_arm}" == true ]]; then
    required_tools+=(qemu-system-aarch64)
fi
if [[ "${needs_riscv}" == true ]]; then
    required_tools+=(qemu-system-riscv64)
fi
if [[ "${needs_x86}" == true ]]; then
    required_tools+=(qemu-system-x86_64)
fi
for tool in "${required_tools[@]}"; do
    if ! command -v "${tool}" >/dev/null 2>&1; then
        echo "error: ${tool} not found in PATH" >&2
        exit 1
    fi
done

# Microkit SDK build needs several rustup targets (tool + capDL initialiser).
if command -v rustup >/dev/null 2>&1; then
    rustup target add x86_64-unknown-linux-musl
    rustup toolchain install nightly-2026-03-18 -c rust-src 2>/dev/null || true
    rustup target add x86_64-unknown-linux-musl --toolchain nightly-2026-03-18 2>/dev/null || true
    if [[ "${needs_arm}" == true ]]; then
        rustup target add aarch64-unknown-none
        rustup target add aarch64-unknown-none --toolchain nightly-2026-03-18 2>/dev/null || true
    fi
    if [[ "${needs_riscv}" == true ]]; then
        rustup target add riscv64gc-unknown-none-elf
        rustup target add riscv64gc-unknown-none-elf --toolchain nightly-2026-03-18 2>/dev/null || true
    fi
    if [[ "${needs_x86}" == true ]]; then
        rustup target add x86_64-unknown-none
        rustup target add x86_64-unknown-none --toolchain nightly-2026-03-18 2>/dev/null || true
    fi
fi

# bindgen (capDL initialiser + lerux PDs) needs libclang.
if ! find /usr/lib -name 'libclang.so' 2>/dev/null | grep -q .; then
    bash "${root}/scripts/install-libclang.sh"
fi
# shellcheck disable=SC1091
source "${root}/scripts/libclang-env.sh"

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