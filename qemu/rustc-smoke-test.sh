#!/usr/bin/env bash
# Smoke test: boot lerux with virtio rootfs and assert rustc compiles hello.rs.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

NO_BUILD=0
for arg in "$@"; do
    case "$arg" in
        --no-build) NO_BUILD=1 ;;
        *) echo "Unknown argument: $arg" >&2; exit 2 ;;
    esac
done

TIMEOUT="${RUSTC_SMOKE_TIMEOUT:-600}"
SERIAL_LOG="${SERIAL_LOG:-$ROOT/build/rustc-smoke-serial.log}"
MARKERS=(
    "init: switchroot to /scheme/initfs"
    "60_rustc-smoke.service"
    "hello from lerux"
)

if [[ "$NO_BUILD" -eq 0 ]]; then
    just build-rootfs-userspace-rustc
fi

if [[ ! -f "$ROOT/build/rootfs.img" ]]; then
    echo "missing build/rootfs.img — run just mk-rootfs" >&2
    exit 1
fi

rm -f "$SERIAL_LOG"
echo "rustc-smoke: booting QEMU (timeout ${TIMEOUT}s)..."

set +e
timeout "$TIMEOUT" qemu-system-x86_64 \
    -kernel "$ROOT/build/kernel" \
    -m 4096 \
    -smp 2 \
    -serial file:"$SERIAL_LOG" \
    -display none \
    -no-reboot \
    -drive file="$ROOT/build/rootfs.img",if=virtio,format=raw
qemu_status=$?
set -e

if [[ ! -f "$SERIAL_LOG" ]]; then
    echo "rustc-smoke: no serial log produced" >&2
    exit 1
fi

echo "rustc-smoke: serial log at $SERIAL_LOG"
failed=0
for marker in "${MARKERS[@]}"; do
    if grep -q "$marker" "$SERIAL_LOG"; then
        echo "OK: saw '$marker'"
    else
        echo "MISSING: '$marker'" >&2
        failed=1
    fi
done

# Fallback markers when rootfs services differ
if grep -q "hello from lerux" "$SERIAL_LOG"; then
    echo "rustc-smoke: hello output found"
elif grep -q "rustc" "$SERIAL_LOG"; then
    echo "rustc-smoke: partial (rustc mentioned but hello missing)" >&2
    failed=1
fi

if [[ "$qemu_status" -eq 124 ]]; then
    echo "rustc-smoke: QEMU timed out after ${TIMEOUT}s" >&2
    failed=1
fi

if [[ "$failed" -ne 0 ]]; then
    echo "--- last 40 lines of serial ---" >&2
    tail -40 "$SERIAL_LOG" >&2 || true
    exit 1
fi

echo "rustc-smoke: passed"
