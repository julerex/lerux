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

if [[ ! -d "$SYSROOT/bin" ]]; then
    echo "build-rustc-redox: run just build-prefix first" >&2
    exit 1
fi

if [[ ! -d "$RUST_SRC" ]]; then
    echo "build-rustc-redox: run just fetch-vendor-sources first (missing $RUST_SRC)" >&2
    exit 1
fi

mkdir -p "$BUILD_DIR" "$OUT"
CONFIG="$BUILD_DIR/config.toml"
cp "$RECIPE_DIR/config-cross.toml" "$CONFIG"
sed -i "s|COOKBOOK_SYSROOT|${SYSROOT}|g" "$CONFIG"
sed -i "s|COOKBOOK_TOOLCHAIN|${HOST_SYSROOT}|g" "$CONFIG"
sed -i "s|COOKBOOK_TARGET|${TARGET}|g" "$CONFIG"
sed -i "s|COOKBOOK_GNU_TARGET|${GNU_TARGET}|g" "$CONFIG"

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
export LD_LIBRARY_PATH="${HOST_SYSROOT}/lib:${LD_LIBRARY_PATH:-}"

# Hack from upstream rust recipe: llvm-config wrapper for cross
mkdir -p "$BUILD_DIR/bin"
if [[ -x "$SYSROOT/bin/${TARGET}-llvm-config" ]]; then
    cp "$SYSROOT/bin/${TARGET}-llvm-config" "$BUILD_DIR/bin/llvm-config"
    export PATH="$BUILD_DIR/bin:$PATH"
fi

ARCH="${TARGET%%-*}"
export "CARGO_TARGET_${ARCH^^}_UNKNOWN_REDOX_RUSTFLAGS=-Clink-args=-L${SYSROOT}/lib -Clink-args=-Wl,-rpath-link,${SYSROOT}/lib"
export RUSTFLAGS_BOOTSTRAP="-Clink-args=-L${HOST_SYSROOT}/lib -Clink-args=-Wl,-rpath-link,${HOST_SYSROOT}/lib"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS="$RUSTFLAGS_BOOTSTRAP"

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
