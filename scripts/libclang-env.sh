#!/usr/bin/env bash
# Sets LIBCLANG_PATH and LD_LIBRARY_PATH for bindgen. Installs into deps/ if needed.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! find /usr/lib -name 'libclang.so' 2>/dev/null | grep -q .; then
    if [[ ! -f "${root}/deps/toolchains/libclang/usr/lib/x86_64-linux-gnu/libclang-14.so.14.0.0" ]]; then
        bash "${root}/scripts/install-libclang.sh" >&2
    fi
fi

clang_root="${root}/deps/toolchains/libclang"
if [[ -d "${clang_root}/usr/lib/llvm-14/lib" ]]; then
    export LIBCLANG_PATH="${clang_root}/usr/lib/llvm-14/lib"
    export LD_LIBRARY_PATH="${clang_root}/usr/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH:-}"
fi