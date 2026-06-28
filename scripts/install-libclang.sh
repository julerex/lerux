#!/usr/bin/env bash
# Install libclang + libLLVM for bindgen into deps/toolchains/ (no sudo).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
toolchains_dir="${root}/deps/toolchains"
clang_root="${toolchains_dir}/libclang"

if find /usr/lib -name 'libclang.so' 2>/dev/null | grep -q .; then
    echo "==> libclang found on system" >&2
    exit 0
fi

if [[ -f "${clang_root}/usr/lib/x86_64-linux-gnu/libclang-14.so.14.0.0" ]]; then
    echo "==> libclang already installed at ${clang_root}" >&2
    exit 0
fi

fetch_deb() {
    local pkg="$1"
    local fallback_url="$2"
    local url=""
    if deb_file="$(apt-cache show "${pkg}" 2>/dev/null | awk '/^Filename:/{print $2; exit}')"; then
        [[ -n "${deb_file}" ]] && url="http://archive.ubuntu.com/ubuntu/${deb_file}"
    fi
    [[ -z "${url}" ]] && url="${fallback_url}"
    local tmp
    tmp="$(mktemp)"
    curl -fsSL -o "${tmp}" "${url}"
    dpkg-deb -x "${tmp}" "${clang_root}"
    rm -f "${tmp}"
}

echo "==> Downloading libclang/llvm packages into deps/toolchains/" >&2
rm -rf "${clang_root}"
mkdir -p "${clang_root}"

fetch_deb libllvm14 \
    "http://archive.ubuntu.com/ubuntu/pool/main/l/llvm-toolchain-14/libllvm14_14.0.0-1ubuntu1.1_amd64.deb"
fetch_deb libclang1-14 \
    "http://archive.ubuntu.com/ubuntu/pool/universe/l/llvm-toolchain-14/libclang1-14_14.0.0-1ubuntu1.1_amd64.deb"
fetch_deb libclang-14-dev \
    "http://archive.ubuntu.com/ubuntu/pool/universe/l/llvm-toolchain-14/libclang-14-dev_14.0.0-1ubuntu1.1_amd64.deb"

if [[ ! -f "${clang_root}/usr/lib/x86_64-linux-gnu/libclang-14.so.14.0.0" ]]; then
    echo "error: libclang install failed" >&2
    exit 1
fi