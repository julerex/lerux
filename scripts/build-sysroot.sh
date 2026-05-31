#!/usr/bin/env bash
# Build lerux userspace sysroot from in-tree vendor/relibc + userspace/runtime.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RELIBC="$ROOT/vendor/relibc"
DESTDIR="$ROOT/.toolchain/x86_64-unknown-redox"
BUILD="$RELIBC/target/x86_64-unknown-redox/release"
TOOLCHAIN_BIN="${TOOLCHAIN_BIN:-$HOME/.rustup/toolchains/nightly-2025-11-15-x86_64-unknown-linux-gnu/bin}"
export PATH="$TOOLCHAIN_BIN:$PATH"
CARGO="$TOOLCHAIN_BIN/cargo"
AR="${AR:-ar}"
NM="${NM:-llvm-nm}"
OBJCOPY="${OBJCOPY:-llvm-objcopy}"
LLD="${LLD:-$TOOLCHAIN_BIN/../lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld}"
TARGET=x86_64-unknown-redox
TOOLCHAIN_URL="${TOOLCHAIN_URL:-https://static.redox-os.org/toolchain/x86_64-unknown-redox/relibc-install.tar.gz}"
GCC_LIB="$ROOT/.toolchain/lib/gcc/x86_64-unknown-redox/13.2.0"

sysroot_ready() {
    [[ -f "$DESTDIR/lib/libc.a" && -f "$DESTDIR/lib/crt0.o" ]] \
        && [[ -f "$GCC_LIB/libgcc_eh.a" ]] \
        && [[ $(stat -c%s "$GCC_LIB/libgcc_eh.a") -gt 1000 ]]
}

if sysroot_ready; then
    echo "Sysroot already present at $DESTDIR; skipping (remove .toolchain to rebuild)"
    exit 0
fi

cd "$RELIBC"

mkdir -p "$BUILD" "$DESTDIR/lib"

echo "Building relibc (static) for $TARGET..."
"$CARGO" rustc --release \
    -Z build-std=core,alloc,compiler_builtins \
    --target "$TARGET" \
    --features math_libm \
    -- --emit "link=$BUILD/librelibc.a"

export NM OBJCOPY
./renamesyms.sh "$BUILD/librelibc.a" "$RELIBC/target/$TARGET/release/deps/"
./stripcore.sh "$BUILD/librelibc.a"

for crt in crt0 crti crtn; do
    "$CARGO" rustc --release \
        --manifest-path "src/$crt/Cargo.toml" \
        -Z build-std=core,alloc,compiler_builtins \
        --target "$TARGET" \
        -- --emit "obj=$BUILD/$crt.o" -C panic=abort
done

# libc.a = librelibc.a (+ empty libm when using math_libm)
cp -f "$BUILD/librelibc.a" "$BUILD/libc.a"
touch "$BUILD/openlibm/libopenlibm.a" 2>/dev/null || mkdir -p "$BUILD/openlibm" && : > "$BUILD/openlibm/libopenlibm.a"

echo "Building ld.so..."
"$CARGO" rustc --release \
    --manifest-path ld_so/Cargo.toml \
    -Z build-std=core,alloc,compiler_builtins \
    --target "$TARGET" \
    -- --emit "obj=$BUILD/ld_so.o" -C panic=abort

"$LLD" -flavor gnu -shared -Bsymbolic --no-relax \
    -T "ld_so/ld_script/$TARGET.ld" \
    --gc-sections "$BUILD/ld_so.o" "$BUILD/libc.a" \
    -o "$BUILD/ld.so"

echo "Building libc.so..."
"$LLD" -flavor gnu -shared --gc-sections \
    -z pack-relative-relocs \
    --sort-common \
    --whole-archive "$BUILD/libc.a" --no-whole-archive \
    -soname libc.so.6 \
    -o "$BUILD/libc.so"

echo "Installing to $DESTDIR..."
mkdir -p "$DESTDIR/lib"
cp -f "$BUILD/libc.a" "$BUILD/libc.so" "$BUILD/crt0.o" "$BUILD/crti.o" "$BUILD/crtn.o" "$BUILD/ld.so" "$DESTDIR/lib/"
ln -sfn crt0.o "$DESTDIR/lib/crt1.o"
ln -sfn crt0.o "$DESTDIR/lib/Scrt1.o"
ln -sfn libc.so "$DESTDIR/lib/libc.so.6"
cp -f "$BUILD/ld.so" "$DESTDIR/lib/ld64.so.1"
"$AR" -rcs "$DESTDIR/lib/libm.a"
"$AR" -rcs "$DESTDIR/lib/libdl.a"
"$AR" -rcs "$DESTDIR/lib/libpthread.a"
"$AR" -rcs "$DESTDIR/lib/librt.a"

install_redox_libgcc() {
    if [[ -f "$GCC_LIB/libgcc_eh.a" ]] && [[ $(stat -c%s "$GCC_LIB/libgcc_eh.a") -gt 1000 ]]; then
        return 0
    fi
    echo "Fetching Redox libgcc (rustc's liblibc links -lgcc_eh; relibc is built in-tree)..."
    local tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' RETURN
    curl -fsSL "$TOOLCHAIN_URL" | tar -xzf - -C "$tmp" ./lib/gcc
    mkdir -p "$ROOT/.toolchain/lib"
    cp -a "$tmp/lib/gcc" "$ROOT/.toolchain/lib/"
}

install_redox_libgcc

echo "Sysroot installed to $DESTDIR"
