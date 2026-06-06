#!/usr/bin/env bash
# Build a redoxfs disk image with the dynamic toolchain prefix and smoke-test files.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SYSROOT="${SYSROOT:-$ROOT/build/prefix/x86_64-unknown-redox/sysroot}"
RUST_REDOX="${RUST_REDOX:-$ROOT/build/rust-redox-install/usr}"
DISK="$ROOT/build/rootfs.img"
UUID_FILE="$ROOT/build/rootfs.uuid"
MNT="$ROOT/build/rootfs-mnt"
REDOXFS_MANIFEST="$ROOT/vendor/redoxfs/Cargo.toml"
DISK_SIZE="${DISK_SIZE:-4G}"

if [[ ! -d "$SYSROOT/bin" ]]; then
    echo "mk-rootfs: run just build-prefix first (missing $SYSROOT)" >&2
    exit 1
fi

host_tool() {
    local bin="$1"
    cargo build --release --manifest-path "$REDOXFS_MANIFEST" --bin "$bin" --features std
    echo "$ROOT/vendor/redoxfs/target/release/$bin"
}

echo "mk-rootfs: building host redoxfs tools"
MKFS="$(host_tool redoxfs-mkfs)"
AR="$(host_tool redoxfs-ar)"

echo "mk-rootfs: creating $DISK ($DISK_SIZE)"
rm -f "$DISK"
truncate -s "$DISK_SIZE" "$DISK"
MKFS_LOG="$ROOT/build/rootfs-mkfs.log"
"$MKFS" "$DISK" 2>&1 | tee "$MKFS_LOG"

if grep -q 'uuid ' "$MKFS_LOG"; then
    grep -oE 'uuid [0-9a-f-]{36}' "$MKFS_LOG" | head -1 | cut -d' ' -f2 > "$UUID_FILE"
fi
if [[ ! -s "$UUID_FILE" ]]; then
    uuidgen | tr '[:upper:]' '[:lower:]' > "$UUID_FILE"
fi
UUID="$(tr -d ' \n' < "$UUID_FILE")"
echo "mk-rootfs: filesystem uuid $UUID"

# Populate via redoxfs-ar (host archive into image)
STAGING="$ROOT/build/rootfs-staging"
rm -rf "$STAGING"
mkdir -p "$STAGING/usr" "$STAGING/lib/init.d" "$STAGING/share/tests" "$STAGING/tmp"

echo "mk-rootfs: staging toolchain"
mkdir -p "$STAGING/usr"

# Prefix: relibc, libLLVM, gcc cross tools, rustlib
rsync -a "$SYSROOT/" "$STAGING/usr/"

# Native Redox rustc (from just build-rustc-redox) overrides host cross tools
if [[ -d "$RUST_REDOX/bin" ]]; then
    echo "mk-rootfs: merging native Redox rustc from $RUST_REDOX"
    rsync -a "$RUST_REDOX/" "$STAGING/usr/"
else
    echo "mk-rootfs: warning — no build/rust-redox-install/usr (only host cross rustc in image)" >&2
    echo "mk-rootfs: run just build-rustc-redox before qemu-rustc-smoke for on-lerux compile" >&2
fi

cat > "$STAGING/share/tests/hello.rs" <<'EOF'
fn main() {
    println!("hello from lerux");
}
EOF

cat > "$STAGING/lib/init.d/60_rustc-smoke.service" <<'EOF'
[unit]
description = "Compile hello.rs with rustc on rootfs"
after = ["50_rootfs.service"]

[service]
cmd = "/usr/bin/rustc"
args = ["/share/tests/hello.rs", "-o", "/tmp/hello"]
type = "oneshot"
EOF

cat > "$STAGING/lib/init.d/61_run-hello.service" <<'EOF'
[unit]
description = "Run compiled hello binary"
after = ["60_rustc-smoke.service"]

[service]
cmd = "/tmp/hello"
type = "oneshot"
EOF

# Write uuid for kernel env injection
echo "$UUID" > "$UUID_FILE"

# Use a tarball + redoxfs-ar if available; fallback: copy tree into raw image via FUSE-less stub
echo "mk-rootfs: populating image with redoxfs-ar"
AR_LOG="$ROOT/build/rootfs-ar.log"
"$AR" "$DISK" "$STAGING" 2>&1 | tee "$AR_LOG"
if grep -q 'uuid ' "$AR_LOG"; then
    grep -oE 'uuid [0-9a-f-]{36}' "$AR_LOG" | tail -1 | cut -d' ' -f2 > "$UUID_FILE"
    UUID="$(tr -d ' \n' < "$UUID_FILE")"
    echo "mk-rootfs: updated uuid from redoxfs-ar: $UUID"
fi

echo "mk-rootfs: done -> $DISK (uuid $UUID)"
