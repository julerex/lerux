#!/usr/bin/env bash
#
# Direct-boot serial smoke test for lerux (Only Rust Redox).
#
# Boots the `direct-boot` kernel under QEMU (headless, no KVM required),
# captures the serial console, and asserts that boot reaches the kmain idle
# loop by checking for the expected early-boot markers. Exits non-zero if a
# required marker is missing, a kernel panic / triple fault is seen, or the
# boot does not reach the idle marker within $TIMEOUT seconds.
#
# Intended for CI (see .github/workflows/rust.yml) and local use:
#   ./qemu/smoke-test.sh             # build the direct-boot kernel, then boot + assert
#   ./qemu/smoke-test.sh --no-build  # assume build/kernel already exists
#
# Env overrides: QEMU, BUILD, MEMORY, TIMEOUT, SERIAL_LOG
set -uo pipefail

usage() {
    cat <<'EOF'
Direct-boot serial smoke test for lerux (Only Rust Redox).

Boots the direct-boot kernel under QEMU (headless, no KVM required), captures the
serial console, and asserts boot reaches the kmain idle loop. Exits non-zero on a
missing marker, a kernel panic / triple fault, or a $TIMEOUT-second timeout.

Usage:
  ./qemu/smoke-test.sh             # build the direct-boot kernel, then boot + assert
  ./qemu/smoke-test.sh --no-build  # assume build/kernel already exists

Env overrides: QEMU, BUILD, MEMORY, TIMEOUT, SERIAL_LOG
EOF
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

QEMU="${QEMU:-qemu-system-x86_64}"
BUILD_DIR="${BUILD:-$REPO_ROOT/build}"
KERNEL="$BUILD_DIR/kernel"
# Bump default for the rustc smoke (larger initfs with redoxfs + stub + services can stress the
# direct-boot synthetic memory map / reservations). 512M was fine for Phase B minimal; 1G+ is safe.
MEMORY="${MEMORY:-1024}"
TIMEOUT="${TIMEOUT:-90}"
SERIAL_LOG="${SERIAL_LOG:-$REPO_ROOT/qemu-serial.log}"

BUILD_KERNEL=1
for arg in "$@"; do
    case "$arg" in
        --no-build) BUILD_KERNEL=0 ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            echo "unknown argument: $arg" >&2
            exit 2
            ;;
    esac
done

# Every one of these substrings must appear on the serial console for a healthy
# boot through early bring-up.
REQUIRED_MARKERS=(
    "Redox OS starting..."
    "Memory:"
    "Paging: new kernel page tables active"
    "Permanently used:"
)
# Seeing this means we reached the kmain idle loop (direct-boot success).
SUCCESS_MARKER="direct-boot mode: skipping userspace bootstrap"
# Phase B userspace milestones (when USERSPACE_SMOKE=1).
USERSPACE_SUCCESS_MARKERS=(
    "init: switchroot to /scheme/initfs"
)

# Rustc-hosting milestone markers (for smoke-rustc once wired).
RUSTC_SUCCESS_MARKERS=(
    "redoxfs mounted"
    "rustc --version"
    "lerux-bootstrap-compiled"
)
USERSPACE_FAIL_MARKERS=(
    "direct-boot mode: skipping userspace bootstrap"
    "failed to open init"
)
# Bootstrap: START:END must differ (non-zero initfs size; Phase A).
BOOTSTRAP_MARKER_PREFIX="Bootstrap:"
# Any of these on the serial console means the boot went wrong.
FAIL_MARKERS=(
    "panicked at"
    "KERNEL PANIC"
    "Triple fault"
    "Kernel page fault"
)

if [ "$BUILD_KERNEL" -eq 1 ]; then
    echo "==> Building direct-boot kernel"
    if [ "${RUSTC_SMOKE:-0}" = "1" ]; then
        build_recipe="build-redoxfs-test-image"
        (cd "$REPO_ROOT" && just "$build_recipe") || true
    fi
    if [ "${USERSPACE_SMOKE:-0}" = "1" ]; then
        build_recipe="build-direct-userspace"
    else
        build_recipe="build-direct"
    fi
    if ! (cd "$REPO_ROOT" && just "$build_recipe"); then
        echo "SMOKE TEST FAILED: build failed" >&2
        exit 1
    fi
fi

if [ ! -f "$KERNEL" ]; then
    echo "SMOKE TEST FAILED: kernel not found at $KERNEL" >&2
    exit 1
fi

if ! command -v "$QEMU" >/dev/null 2>&1; then
    echo "SMOKE TEST FAILED: '$QEMU' not found on PATH" >&2
    exit 1
fi

: >"$SERIAL_LOG"

echo "==> Booting direct-boot kernel under QEMU (timeout ${TIMEOUT}s, serial -> $SERIAL_LOG)"
EXTRA_QEMU_ARGS=""
if [ "${RUSTC_SMOKE:-0}" = "1" ]; then
    EXTRA_QEMU_ARGS="-drive file=/tmp/lerux-rustc-test.img,format=raw,if=virtio"
fi
"$QEMU" \
    -kernel "$KERNEL" \
    -m "$MEMORY" \
    -display none \
    -no-reboot \
    -serial "file:$SERIAL_LOG" \
    $EXTRA_QEMU_ARGS &
QEMU_PID=$!

cleanup() {
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
}
trap cleanup EXIT

log_has() { grep -qF -- "$1" "$SERIAL_LOG" 2>/dev/null; }
log_has_any() {
    local m
    for m in "$@"; do log_has "$m" && return 0; done
    return 1
}

# Bootstrap log is "Bootstrap: START:END"; Phase A requires non-zero initfs (START != END).
bootstrap_has_nonzero_size() {
    local line start end
    line=$(grep -F "$BOOTSTRAP_MARKER_PREFIX" "$SERIAL_LOG" 2>/dev/null | head -1) || return 1
    start=${line#*Bootstrap: }
    start=${start%%:*}
    end=${line##*:}
    [ -n "$start" ] && [ -n "$end" ] && [ "$start" != "$end" ]
}

# Poll the serial log so we can stop as soon as the boot succeeds or fails,
# instead of always waiting out the full timeout.
deadline=$((SECONDS + TIMEOUT))
outcome="timeout"
while [ "$SECONDS" -lt "$deadline" ]; do
    if [ "${USERSPACE_SMOKE:-0}" = "1" ]; then
        if log_has_any "${USERSPACE_SUCCESS_MARKERS[@]}"; then
            outcome="userspace"
            break
        fi
    elif [ "${RUSTC_SMOKE:-0}" = "1" ]; then
        # For the rustc milestone, wait until all three markers are present (mount + version + compile).
        # This gives a clear early exit when the full chain (redoxfs service + stub on FS) has run.
        if log_has "${RUSTC_SUCCESS_MARKERS[0]}" && log_has "${RUSTC_SUCCESS_MARKERS[1]}" && log_has "${RUSTC_SUCCESS_MARKERS[2]}"; then
            outcome="rustc"
            break
        fi
    elif log_has "$SUCCESS_MARKER"; then
        outcome="idle"
        break
    fi
    if log_has_any "${FAIL_MARKERS[@]}"; then
        outcome="fault"
        break
    fi
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
        outcome="exited"
        break
    fi
    sleep 0.5
done

cleanup
trap - EXIT

echo "----- serial log (${SERIAL_LOG}) -----"
cat "$SERIAL_LOG" 2>/dev/null || true
echo "--------------------------------------"
echo "==> QEMU outcome: $outcome"

# The captured serial log is the source of truth for pass/fail.
fail=0
if [ "${USERSPACE_SMOKE:-0}" = "1" ]; then
    for m in "${REQUIRED_MARKERS[@]}"; do
        if log_has "$m"; then
            printf '  [ ok ] %s\n' "$m"
        else
            printf '  [MISS] %s\n' "$m"
            fail=1
        fi
    done
    for m in "${USERSPACE_SUCCESS_MARKERS[@]}"; do
        if log_has "$m"; then
            printf '  [ ok ] %s\n' "$m"
        else
            printf '  [MISS] %s\n' "$m"
            fail=1
        fi
    done
    for m in "${USERSPACE_FAIL_MARKERS[@]}"; do
        if log_has "$m"; then
            printf '  [FAIL] saw userspace failure marker: %s\n' "$m"
            fail=1
        fi
    done
elif [ "${RUSTC_SMOKE:-0}" = "1" ]; then
    # Rustc-hosting milestone: REQUIRED + the three RUSTC markers (mount from service, --version and compiled from stub on FS).
    for m in "${REQUIRED_MARKERS[@]}"; do
        if log_has "$m"; then
            printf '  [ ok ] %s\n' "$m"
        else
            printf '  [MISS] %s\n' "$m"
            fail=1
        fi
    done
    for m in "${RUSTC_SUCCESS_MARKERS[@]}"; do
        if log_has "$m"; then
            printf '  [ ok ] %s\n' "$m"
        else
            printf '  [MISS] %s\n' "$m"
            fail=1
        fi
    done
else
    for m in "${REQUIRED_MARKERS[@]}" "$SUCCESS_MARKER"; do
        if log_has "$m"; then
            printf '  [ ok ] %s\n' "$m"
        else
            printf '  [MISS] %s\n' "$m"
            fail=1
        fi
    done
fi
if bootstrap_has_nonzero_size; then
    printf '  [ ok ] Bootstrap: non-zero initfs size\n'
else
    printf '  [MISS] Bootstrap: non-zero initfs size\n'
    fail=1
fi
for m in "${FAIL_MARKERS[@]}"; do
    if log_has "$m"; then
        printf '  [FAIL] saw failure marker: %s\n' "$m"
        fail=1
    fi
done

if [ "$fail" -eq 0 ]; then
    if [ "${USERSPACE_SMOKE:-0}" = "1" ]; then
        echo "SMOKE TEST PASSED: direct-boot reached init and early daemons."
    elif [ "${RUSTC_SMOKE:-0}" = "1" ]; then
        echo "SMOKE TEST PASSED: redoxfs mounted + bootstrap rustc ran and compiled on lerux (RUSTC markers present)."
    else
        echo "SMOKE TEST PASSED: direct-boot reached the kmain idle loop."
    fi
    exit 0
fi

echo "SMOKE TEST FAILED (outcome=$outcome)." >&2
exit 1
