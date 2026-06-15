#!/usr/bin/env bash
# Cross-build native x86_64-unknown-redox rustc from Linux using the prefix sysroot.
#
# RUST_CODEGEN_BACKEND=llvm   (default)
# RUST_CODEGEN_BACKEND=cranelift
#
# Output: build/rust-redox-install/usr/ (Redox-native rustc + rustlib)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BACKEND="${RUST_CODEGEN_BACKEND:-llvm}"
TARGET="${TARGET:-x86_64-unknown-redox}"
GNU_TARGET="${GNU_TARGET:-x86_64-unknown-redox}"
SYSROOT="${SYSROOT:-$ROOT/build/prefix/$TARGET/sysroot}"
HOST_SYSROOT="${HOST_SYSROOT:-$SYSROOT}"
OUT="$ROOT/build/rust-redox-install"
RUST_SRC="${RUST_SRC:-$ROOT/vendor/sources/rust}"
CLIF_SRC="${CLIF_SRC:-$ROOT/vendor/sources/rustc_codegen_cranelift}"
RECIPE_DIR="$ROOT/vendor/redox-recipes/dev/rust"
BUILD_DIR="$ROOT/build/rust-build"
BUILD_HOST="${BUILD_HOST:-x86_64-unknown-linux-gnu}"

host_glibc_version() {
    ldd --version 2>/dev/null | head -1 | grep -oE '[0-9]+\.[0-9]+' | head -1
}

max_glibc_from_lib() {
    local lib="$1"
    objdump -T "$lib" 2>/dev/null \
        | grep -oE 'GLIBC_2\.[0-9]+' \
        | sort -uV \
        | tail -1 \
        | sed 's/GLIBC_2\.//'
}

prefix_llvm_needs_newer_glibc() {
    local host_ver prefix_ver lib
    host_ver="$(host_glibc_version)"
    for lib in "$SYSROOT/lib"/libLLVM*.so*; do
        [[ -f "$lib" ]] || continue
        prefix_ver="$(max_glibc_from_lib "$lib")"
        [[ -n "$prefix_ver" && -n "$host_ver" ]] || continue
        if [[ "$(printf '%s\n' "$host_ver" "$prefix_ver" | sort -V | tail -1)" != "$host_ver" ]]; then
            return 0
        fi
    done
    return 1
}

remove_linux_gnu_llvm_stanzas() {
    awk '
        /^\[target\.(x86_64|aarch64)-unknown-linux-gnu\]/ { skip=2; next }
        skip > 0 { skip--; next }
        { print }
    ' "$CONFIG" > "$CONFIG.tmp" && mv "$CONFIG.tmp" "$CONFIG"
}

ensure_rust_history_for_ci_llvm() {
    local branch
    cd "$RUST_SRC"
    if ! git rev-parse --is-shallow-repository >/dev/null 2>&1 \
        || [[ "$(git rev-parse --is-shallow-repository)" != "true" ]]; then
        return 0
    fi
    branch="$(git rev-parse --abbrev-ref HEAD)"
    echo "build-rustc-redox: deepening shallow rust clone for CI LLVM (branch $branch)..." >&2
    git fetch --deepen=500 origin "$branch" \
        || git fetch --unshallow origin "$branch" \
        || git fetch --unshallow
}

if [[ ! -d "$SYSROOT/bin" ]]; then
    echo "build-rustc-redox: run just build-prefix first" >&2
    exit 1
fi

if [[ ! -d "$RUST_SRC" ]]; then
    echo "build-rustc-redox: run just fetch-vendor-sources first (missing $RUST_SRC)" >&2
    exit 1
fi

"$(dirname "$0")/patch-vendor-rust.sh"

mkdir -p "$BUILD_DIR" "$OUT"
CONFIG="$BUILD_DIR/config.toml"
cp "$RECIPE_DIR/config-cross.toml" "$CONFIG"
sed -i 's/submodules = false/submodules = true/' "$CONFIG"
sed -i "s|COOKBOOK_SYSROOT|${SYSROOT}|g" "$CONFIG"
sed -i "s|COOKBOOK_TOOLCHAIN|${HOST_SYSROOT}|g" "$CONFIG"
sed -i "s|COOKBOOK_TARGET|${TARGET}|g" "$CONFIG"
sed -i "s|COOKBOOK_GNU_TARGET|${GNU_TARGET}|g" "$CONFIG"

USE_CI_LLVM_BOOTSTRAP=0
if [[ "$HOST_SYSROOT" == "$SYSROOT" ]] && prefix_llvm_needs_newer_glibc; then
    host_ver="$(host_glibc_version)"
    if [[ "${ALLOW_CI_LLVM_BOOTSTRAP:-}" != "1" ]]; then
        echo "build-rustc-redox: prefix LLVM needs newer glibc than host ($host_ver)" >&2
        echo "build-rustc-redox: build prefix locally: PREFIX_BINARY=0 just build-prefix" >&2
        echo "build-rustc-redox: or use a host with glibc >= 2.38" >&2
        echo "build-rustc-redox: set ALLOW_CI_LLVM_BOOTSTRAP=1 to attempt CI LLVM (may 404 on Redox fork)" >&2
        exit 1
    fi
    USE_CI_LLVM_BOOTSTRAP=1
    echo "build-rustc-redox: prefix LLVM needs newer glibc than host ($host_ver); using CI LLVM for bootstrap host ($BUILD_HOST)" >&2
    sed -i 's/download-ci-llvm = false/download-ci-llvm = true/' "$CONFIG"
    remove_linux_gnu_llvm_stanzas
fi

if [[ "$BACKEND" == "cranelift" ]]; then
    if [[ ! -d "$CLIF_SRC" ]]; then
        echo "build-rustc-redox: missing $CLIF_SRC — run just fetch-vendor-sources" >&2
        exit 1
    fi
    cat "$RECIPE_DIR/config-cranelift.toml" >> "$CONFIG"
    echo "build-rustc-redox: Cranelift backend (experimental)"
else
    echo "build-rustc-redox: LLVM backend (upstream default)"
fi

export PATH="$SYSROOT/bin:$PATH"

# Hack from upstream rust recipe: llvm-config wrapper for cross (Redox target only)
mkdir -p "$BUILD_DIR/bin"
if [[ -x "$SYSROOT/bin/${TARGET}-llvm-config" ]]; then
    cp "$SYSROOT/bin/${TARGET}-llvm-config" "$BUILD_DIR/bin/llvm-config"
    export PATH="$BUILD_DIR/bin:$PATH"
fi

ARCH="${TARGET%%-*}"
export "CARGO_TARGET_${ARCH^^}_UNKNOWN_REDOX_RUSTFLAGS=-Clink-args=-L${SYSROOT}/lib -Clink-args=-Wl,-rpath-link,${SYSROOT}/lib"

if [[ "$USE_CI_LLVM_BOOTSTRAP" == "1" ]]; then
    ensure_rust_history_for_ci_llvm
    unset LD_LIBRARY_PATH
else
    export LD_LIBRARY_PATH="${HOST_SYSROOT}/lib:${LD_LIBRARY_PATH:-}"
    export RUSTFLAGS_BOOTSTRAP="-Clink-args=-L${HOST_SYSROOT}/lib -Clink-args=-Wl,-rpath-link,${HOST_SYSROOT}/lib"
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="$RUSTFLAGS_BOOTSTRAP"
fi

unset AR AS CC CXX LD LDFLAGS NM OBJCOPY OBJDUMP RANLIB READELF RUSTFLAGS CARGO_ENCODED_RUSTFLAGS STRIP

cd "$RUST_SRC"
python3 x.py install \
    --config "$CONFIG" \
    --jobs "$(nproc)"

INSTALL_SRC="$RUST_SRC/build/${TARGET}/stage2-tools-bin" 2>/dev/null || true
# x.py install --prefix puts files under 'install/' relative to rust source
RUST_INSTALL="$RUST_SRC/install"
if [[ ! -d "$RUST_INSTALL/bin" ]]; then
    RUST_INSTALL="$RUST_SRC/build/tmp/install"
fi
if [[ ! -d "$RUST_INSTALL/bin" ]]; then
    RUST_INSTALL="$(find "$RUST_SRC" -path '*/install/bin/rustc' -printf '%h\n' 2>/dev/null | head -1)"
    RUST_INSTALL="$(dirname "$RUST_INSTALL" 2>/dev/null || echo "$RUST_SRC/install")"
fi

rm -rf "$OUT/usr"
mkdir -p "$OUT/usr"
if [[ -d "$RUST_SRC/install" ]]; then
    rsync -a "$RUST_SRC/install/" "$OUT/usr/"
else
    echo "build-rustc-redox: could not find install/ — check $RUST_SRC/build logs" >&2
    exit 1
fi

# rust-lld workaround (upstream recipe)
LLD_DIR="$OUT/usr/lib/rustlib/$TARGET/bin"
mkdir -p "$LLD_DIR/gcc-ld"
if [[ -x "$SYSROOT/bin/lld" ]]; then
    cp "$SYSROOT/bin/lld" "$LLD_DIR/rust-lld"
    ln -sf rust-lld "$LLD_DIR/gcc-ld/ld.lld"
fi

echo "build-rustc-redox: installed to $OUT/usr (backend=$BACKEND)"
file "$OUT/usr/bin/rustc" 2>/dev/null || true
