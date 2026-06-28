#!/usr/bin/env bash
# Download the official Microkit SDK release (prebuilt seL4 per board).
# Use when the ARM bare-metal toolchain for `build-sdk` is not installed.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
version="2.2.0"
dest="${root}/deps/microkit-sdk"
url="https://github.com/seL4/microkit/releases/download/${version}/microkit-sdk-${version}-linux-x86-64.tar.gz"

if [[ -d "${dest}/bin" ]]; then
    echo "==> SDK already present at ${dest}"
else
    echo "==> Downloading Microkit SDK ${version}"
    mkdir -p "${dest}"
    curl -fsSL "${url}" | tar -xzf - -C "${dest}" --strip-components=1
    chmod -R a+X "${dest}"
fi

echo "${dest}" > "${root}/deps/.sdk-path"
echo "==> Microkit SDK: ${dest}"