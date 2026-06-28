#!/usr/bin/env bash
# Install ARM GNU bare-metal toolchain into deps/toolchains/ if not on PATH.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
toolchains_dir="${root}/deps/toolchains"
url="https://developer.arm.com/-/media/Files/downloads/gnu/12.2.rel1/binrel/arm-gnu-toolchain-12.2.rel1-x86_64-aarch64-none-elf.tar.xz"

if command -v aarch64-none-elf-gcc >/dev/null 2>&1; then
    echo "==> aarch64-none-elf-gcc already on PATH: $(command -v aarch64-none-elf-gcc)" >&2
    dirname "$(command -v aarch64-none-elf-gcc)"
    exit 0
fi

# Tarball extracts to arm-gnu-toolchain-12.2.rel1-x86_64-aarch64-none-elf (lowercase rel1).
install_dir="$(find "${toolchains_dir}" -maxdepth 1 -type d -name 'arm-gnu-toolchain-*-aarch64-none-elf' 2>/dev/null | head -1)"
if [[ -n "${install_dir}" && -x "${install_dir}/bin/aarch64-none-elf-gcc" ]]; then
    echo "==> ARM toolchain already installed at ${install_dir}" >&2
    echo "${install_dir}/bin"
    exit 0
fi

echo "==> Downloading ARM GNU toolchain 12.2.Rel1" >&2
mkdir -p "${toolchains_dir}"
tmp="$(mktemp)"
curl -fsSL -o "${tmp}" "${url}"
tar -xf "${tmp}" -C "${toolchains_dir}"
rm -f "${tmp}"

install_dir="$(find "${toolchains_dir}" -maxdepth 1 -type d -name 'arm-gnu-toolchain-*-aarch64-none-elf' | head -1)"
if [[ -z "${install_dir}" || ! -x "${install_dir}/bin/aarch64-none-elf-gcc" ]]; then
    echo "error: ARM toolchain install failed" >&2
    exit 1
fi

echo "${install_dir}/bin"