#!/usr/bin/env bash
# Install qemu-system-riscv64 into deps/toolchains/ if not on PATH (no sudo).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
toolchains_dir="${root}/deps/toolchains"
qemu_root="${toolchains_dir}/qemu-misc"
qemu_bin="${qemu_root}/usr/bin/qemu-system-riscv64"

if command -v qemu-system-riscv64 >/dev/null 2>&1; then
    echo "==> qemu-system-riscv64 already on PATH: $(command -v qemu-system-riscv64)" >&2
    dirname "$(command -v qemu-system-riscv64)"
    exit 0
fi

if [[ -x "${qemu_bin}" ]]; then
    echo "==> QEMU RISC-V already installed at ${qemu_root}" >&2
    echo "${qemu_root}/usr/bin"
    exit 0
fi

deb_url=""
if deb_file="$(apt-cache show qemu-system-misc 2>/dev/null | awk '/^Filename:/{print $2; exit}')"; then
    if [[ -n "${deb_file}" ]]; then
        deb_url="http://archive.ubuntu.com/ubuntu/${deb_file}"
    fi
fi
if [[ -z "${deb_url}" ]]; then
    deb_url="http://archive.ubuntu.com/ubuntu/pool/main/q/qemu/qemu-system-misc_6.2%2bdfsg-2ubuntu6.31_amd64.deb"
fi

echo "==> Downloading qemu-system-misc from ${deb_url}" >&2
mkdir -p "${toolchains_dir}"
tmp="$(mktemp)"
curl -fsSL -o "${tmp}" "${deb_url}"
rm -rf "${qemu_root}"
mkdir -p "${qemu_root}"
dpkg-deb -x "${tmp}" "${qemu_root}"
rm -f "${tmp}"

if [[ ! -x "${qemu_bin}" ]]; then
    echo "error: QEMU RISC-V install failed" >&2
    exit 1
fi

echo "${qemu_root}/usr/bin"