#!/usr/bin/env bash
# Build or download the Redox prefix sysroot (dynamic toolchain for rootfs).
#
# PREFIX_BINARY=1 (default): download official gcc/rust/clang tarballs from static.redox-os.org
# PREFIX_BINARY=0: cook from source via vendored tryredox/redox (requires LERUX_REDOX_REF)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

HOST_TARGET="${HOST_TARGET:-x86_64-unknown-linux-gnu}"
TARGET="${TARGET:-x86_64-unknown-redox}"
PREFIX_BINARY="${PREFIX_BINARY:-1}"
LERUX_REDOX_REF="${LERUX_REDOX_REF:-$ROOT/../tryredox}"
PREFIX="$ROOT/build/prefix/$TARGET"
SYSROOT="$PREFIX/sysroot"
BASE_URL="https://static.redox-os.org/toolchain/$HOST_TARGET/$TARGET"

download_tarball() {
    local name="$1"
    local dest="$PREFIX/$name"
    if [[ -f "$dest/.complete" ]]; then
        echo "prefix: $name already present"
        return 0
    fi
    echo "prefix: downloading $name.tar.gz"
    mkdir -p "$dest.partial"
    wget -q -O "$dest.partial.tar.gz" "$BASE_URL/$name.tar.gz"
    tar --extract --file "$dest.partial.tar.gz" --directory "$dest.partial" --strip-components=1
    rm -f "$dest.partial.tar.gz"
    touch "$dest.partial/.complete"
    rm -rf "$dest"
    mv "$dest.partial" "$dest"
}

merge_sysroot() {
    if [[ -f "$SYSROOT/.complete" ]]; then
        echo "prefix: sysroot already at $SYSROOT"
        return 0
    fi
    echo "prefix: merging sysroot at $SYSROOT"
    rm -rf "$SYSROOT.partial" "$SYSROOT"
    mkdir -p "$SYSROOT.partial"
    for part in gcc-install rust-install clang-install; do
        cp -a "$PREFIX/$part/." "$SYSROOT.partial/"
    done
    touch "$SYSROOT.partial/.complete"
    mv "$SYSROOT.partial" "$SYSROOT"
    echo "prefix: sysroot ready at $SYSROOT"
}

build_from_source() {
    local redox_dir=""
    if [[ -f "$LERUX_REDOX_REF/redox/build/container.tag" ]]; then
        redox_dir="$LERUX_REDOX_REF/redox"
    elif [[ -d "$LERUX_REDOX_REF/redox" ]]; then
        redox_dir="$LERUX_REDOX_REF/redox"
    elif [[ -d "$LERUX_REDOX_REF/redox-master" ]]; then
        redox_dir="$LERUX_REDOX_REF/redox-master"
    fi
    if [[ -z "$redox_dir" ]]; then
        echo "PREFIX_BINARY=0 requires Redox build tree at LERUX_REDOX_REF=$LERUX_REDOX_REF" >&2
        exit 1
    fi
    echo "prefix: cooking from source via $redox_dir (this takes a long time)"
    if [[ -f "$redox_dir/build/container.tag" ]] \
        && ! podman image exists redox-base >/dev/null 2>&1; then
        echo "prefix: stale container.tag (redox-base image missing), rebuilding..." >&2
        rm -f "$redox_dir/build/container.tag"
    fi
    if [[ ! -f "$redox_dir/build/container.tag" ]]; then
        echo "prefix: building podman container first (slow one-time step)..." >&2
        make -C "$redox_dir" build/container.tag \
            PREFIX_BINARY=0 \
            TARGET="$TARGET" \
            HOST_TARGET="$HOST_TARGET" \
            ROOT="$redox_dir"
    fi
    make -C "$redox_dir" prefix \
        PREFIX_BINARY=0 \
        TARGET="$TARGET" \
        HOST_TARGET="$HOST_TARGET" \
        ROOT="$redox_dir"
    rm -rf "$PREFIX"
    mkdir -p "$(dirname "$PREFIX")"
    ln -sf "$redox_dir/prefix/$TARGET/sysroot" "$SYSROOT"
}

mkdir -p "$PREFIX"

if [[ "$PREFIX_BINARY" == "1" ]]; then
    for part in gcc-install rust-install clang-install; do
        download_tarball "$part"
    done
    merge_sysroot
else
    build_from_source
fi

# Convenience symlink for scripts
mkdir -p "$ROOT/build"
ln -sfn "$SYSROOT" "$ROOT/build/prefix-sysroot"

echo "build-prefix: done -> $SYSROOT"
