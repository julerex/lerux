#!/usr/bin/env bash
# Extract a single binary from an Ubuntu .deb into deps/toolchains/<name>/usr/bin (no sudo).
set -euo pipefail

if [[ $# -lt 3 ]]; then
    echo "usage: install-deb-tool.sh <name> <deb-url> <binary-basename>" >&2
    exit 1
fi

name="$1"
deb_url="$2"
binary="$3"

root="$(cd "$(dirname "$0")/.." && pwd)"
toolchains_dir="${root}/deps/toolchains"
install_root="${toolchains_dir}/${name}"
bin_path="${install_root}/usr/bin/${binary}"

if command -v "${binary}" >/dev/null 2>&1; then
    dirname "$(command -v "${binary}")"
    exit 0
fi

if [[ -x "${bin_path}" ]]; then
    echo "${install_root}/usr/bin"
    exit 0
fi

mkdir -p "${toolchains_dir}"
tmp="$(mktemp)"
curl -fsSL -o "${tmp}" "${deb_url}"
rm -rf "${install_root}"
mkdir -p "${install_root}"
dpkg-deb -x "${tmp}" "${install_root}"
rm -f "${tmp}"

if [[ ! -x "${bin_path}" ]]; then
    echo "error: ${binary} not found in ${deb_url}" >&2
    exit 1
fi

echo "${install_root}/usr/bin"