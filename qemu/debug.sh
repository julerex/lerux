#!/usr/bin/env bash
#
# direct-boot GDB debugging helper for lerux (Only Rust Redox)
#
# Boots the direct-boot kernel under QEMU stopped at entry (-S) with a GDB stub
# (-s, tcp::1234) and exception/reset logging, then optionally attaches GDB with
# the kernel symbols and the boot-path breakpoints documented in NOTES.md.
#
# Usage:
#   ./qemu/debug.sh             # launch QEMU (paused) + attach GDB in this terminal
#   ./qemu/debug.sh --no-gdb    # only launch QEMU (paused); attach GDB yourself
#
# Two-terminal workflow (equivalent to `just qemu-direct -- -s -S` + `just gdb`):
#   Terminal 1:  ./qemu/debug.sh --no-gdb
#   Terminal 2:  just gdb
#
# QEMU interrupt/reset logging is written to qemu-int.log so triple faults and
# page faults can be inspected after the fact (look for `v=0e` / `Triple fault`).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

QEMU="${QEMU:-qemu-system-x86_64}"
BUILD_DIR="${BUILD:-$REPO_ROOT/build}"
KERNEL="$BUILD_DIR/kernel"
SYMS="$BUILD_DIR/kernel.sym"
MEMORY="${MEMORY:-512}"
INT_LOG="${INT_LOG:-$REPO_ROOT/qemu-int.log}"
GDB_PORT="${GDB_PORT:-1234}"

ATTACH_GDB=1
for arg in "$@"; do
    case "$arg" in
        --no-gdb) ATTACH_GDB=0 ;;
        *) echo "unknown argument: $arg" >&2; exit 2 ;;
    esac
done

# Build the direct-boot kernel (produces build/kernel and build/kernel.sym).
echo "==> Building direct-boot kernel"
( cd "$REPO_ROOT" && just build-direct )

if [ ! -f "$KERNEL" ]; then
    echo "kernel not found at $KERNEL" >&2
    exit 1
fi

echo "==> Launching QEMU (paused at entry, gdb stub on :$GDB_PORT)"
echo "    interrupt/reset log: $INT_LOG"
"$QEMU" \
    -kernel "$KERNEL" \
    -m "$MEMORY" \
    -serial mon:stdio \
    -display none \
    -no-reboot \
    -d int,cpu_reset \
    -D "$INT_LOG" \
    -s -S &
QEMU_PID=$!
trap 'kill "$QEMU_PID" 2>/dev/null || true' EXIT

if [ "$ATTACH_GDB" -eq 0 ]; then
    echo "==> QEMU paused (pid $QEMU_PID). Attach with: just gdb"
    echo "    (Ctrl-C here will terminate QEMU.)"
    wait "$QEMU_PID"
    exit 0
fi

# Give QEMU a moment to open the gdb socket.
sleep 1

# Boot-path breakpoints (see NOTES.md). pvh_start32/kstart are plain symbols; the
# Rust entry points must be addressed by their full path (bare `start`/`kmain` do
# not resolve in GDB).
#   pvh_start32                          - PVH stub entered (32-bit)
#   kstart                               - Rust entry after the stub
#   kernel::arch::x86_shared::start::start - args + serial + paging
#   kernel::startup::kmain               - BSP init complete (direct-boot skips userspace bootstrap)
echo "==> Attaching GDB"
exec gdb \
    -ex "symbol-file $SYMS" \
    -ex "set language rust" \
    -ex "target remote localhost:$GDB_PORT" \
    -ex "set pagination off" \
    -ex "set confirm off" \
    -ex "break pvh_start32" \
    -ex "break kstart" \
    -ex "break kernel::arch::x86_shared::start::start" \
    -ex "break kernel::startup::kmain" \
    -ex "echo \n[debug.sh] breakpoints set: pvh_start32, kstart, start(), kmain(). 'continue' to run.\n"
