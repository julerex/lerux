#!/usr/bin/env bash
# Install device-tree-compiler (dtc) into deps/toolchains/ if not on PATH (no sudo).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
toolchains_dir="${root}/deps/toolchains"
dtc_root="${toolchains_dir}/dtc"
dtc_bin="${dtc_root}/usr/bin/dtc"

if command -v dtc >/dev/null 2>&1; then
    echo "==> dtc already on PATH: $(command -v dtc)" >&2
    dirname "$(command -v dtc)"
    exit 0
fi

if [[ -x "${dtc_bin}" ]]; then
    echo "==> dtc already installed at ${dtc_root}" >&2
    echo "${dtc_root}/usr/bin"
    exit 0
fi

deb_url=""
if deb_file="$(apt-cache show device-tree-compiler 2>/dev/null | awk '/^Filename:/{print $2; exit}')"; then
    if [[ -n "${deb_file}" ]]; then
        deb_url="http://archive.ubuntu.com/ubuntu/${deb_file}"
    fi
fi
if [[ -z "${deb_url}" ]]; then
    deb_url="http://archive.ubuntu.com/ubuntu/pool/main/d/device-tree-compiler/device-tree-compiler_1.6.1-1_amd64.deb"
fi

echo "==> Downloading device-tree-compiler from ${deb_url}" >&2
mkdir -p "${toolchains_dir}"
tmp="$(mktemp)"
curl -fsSL -o "${tmp}" "${deb_url}"
rm -rf "${dtc_root}"
mkdir -p "${dtc_root}"
dpkg-deb -x "${tmp}" "${dtc_root}"
rm -f "${tmp}"

if [[ ! -x "${dtc_bin}" ]]; then
    echo "error: dtc install failed" >&2
    exit 1
fi

echo "${dtc_root}/usr/bin"