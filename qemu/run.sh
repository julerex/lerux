#!/usr/bin/env bash
#
# QEMU bring-up launcher for lerux (Only Rust Redox)
#
# Usage:
#   ./run.sh                    # normal run
#   ./run.sh -s -S              # run with GDB stub
#   KERNEL_FEATURES=... ./run.sh
#
# The script focuses on building the loader reliably.
# Kernel building is optional (see KERNEL_BUILD below).
#
set -euo pipefail

# ---------- Colors ----------
if [ -t 1 ]; then
    GREEN="\033[0;32m"
    YELLOW="\033[1;33m"
    RED="\033[0;31m"
    BLUE="\033[0;34m"
    NC="\033[0m"
else
    GREEN=""; YELLOW=""; RED=""; BLUE=""; NC=""
fi

info()  { echo -e "${BLUE}==>${NC} $1"; }
ok()    { echo -e "${GREEN}✓${NC}  $1"; }
warn()  { echo -e "${YELLOW}!${NC}  $1"; }
error() { echo -e "${RED}✗${NC}  $1" >&2; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# KERNEL_DIR no longer needed for make; Makefile is now at repo root

# ---------- Configuration ----------
QEMU="${QEMU:-qemu-system-x86_64}"
MEMORY="${MEMORY:-512M}"
SMP="${SMP:-1}"
SERIAL="${SERIAL:-stdio}"
ENABLE_KVM="${ENABLE_KVM:-no}"

# Set to "yes" to also try building the kernel inside this script
BUILD_KERNEL="${BUILD_KERNEL:-no}"
KERNEL_FEATURES="${KERNEL_FEATURES:-serial_debug}"

LOADER_ASM="$SCRIPT_DIR/loader.asm"
LOADER_S="$SCRIPT_DIR/loader.S"
LOADER_LD="$SCRIPT_DIR/loader.ld"
LOADER_BIN="$SCRIPT_DIR/loader.bin"
LOADER_ELF="$SCRIPT_DIR/loader.elf"

KERNEL_BUILD_DIR="${KERNEL_BUILD_DIR:-$SCRIPT_DIR/build}"
KERNEL_BIN="$KERNEL_BUILD_DIR/kernel"

# ---------- Banner ----------
echo
echo "╔════════════════════════════════════════════════════════════╗"
echo "║            lerux QEMU Bring-up Launcher                    ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo

# ---------- Tool checks ----------
info "Checking required tools..."

MISSING=0

if command -v "$QEMU" >/dev/null 2>&1; then
    ok "Found qemu: $($QEMU --version | head -n1)"
else
    error "qemu-system-x86_64 not found in PATH"
    MISSING=1
fi

if command -v nasm >/dev/null 2>&1; then
    ok "Found nasm: $(nasm -v | head -n1)"
    USE_NASM=1
else
    warn "nasm not found — will use GNU as (loader.S)"
    USE_NASM=0
fi

if command -v ld >/dev/null 2>&1; then
    ok "Found ld (binutils)"
else
    error "ld not found (install binutils)"
    MISSING=1
fi

if command -v objcopy >/dev/null 2>&1; then
    ok "Found objcopy (binutils)"
else
    error "objcopy not found (install binutils)"
    MISSING=1
fi

if [ $MISSING -eq 1 ]; then
    error "Missing required tools. Please install them and try again."
    exit 1
fi

echo

# ---------- Optional kernel build ----------
if [ "$BUILD_KERNEL" = "yes" ]; then
    info "Building kernel (features=$KERNEL_FEATURES) ..."
    mkdir -p "$KERNEL_BUILD_DIR"

    if make -C "$REPO_ROOT" \
            BUILD="$KERNEL_BUILD_DIR" \
            KERNEL_CARGO_FEATURES="$KERNEL_FEATURES" \
            all; then
        ok "Kernel built successfully"
    else
        error "Kernel build failed (this is common — the kernel needs a specific nightly toolchain)"
        warn "You can build the kernel manually with:"
        echo "    make -C .. BUILD=qemu/build KERNEL_CARGO_FEATURES=serial_debug all"
        exit 1
    fi
else
    if [ -f "$KERNEL_BIN" ]; then
        ok "Using existing kernel at $KERNEL_BIN ($(stat -c%s "$KERNEL_BIN") bytes)"
    else
        warn "No kernel binary found at $KERNEL_BIN"
        warn "The loader will currently just say 'N' (no kernel module)."
        warn "Build the kernel first with:"
        echo "    make -C .. BUILD=qemu/build KERNEL_CARGO_FEATURES=serial_debug all"
    fi
fi

echo

# ---------- Build loader ----------
info "Building loader..."

rm -f "$SCRIPT_DIR"/loader.o "$LOADER_ELF" "$LOADER_BIN"

if [ "$USE_NASM" = "1" ]; then
    info "Using NASM → loader.asm"
    nasm -f elf64 -o "$SCRIPT_DIR/loader.o" "$LOADER_ASM"
else
    info "Using GNU as → loader.S"
    as --64 -o "$SCRIPT_DIR/loader.o" "$LOADER_S"
fi

ok "Assembled loader object"

ld -T "$LOADER_LD" -o "$LOADER_ELF" "$SCRIPT_DIR/loader.o"
objcopy -O binary "$LOADER_ELF" "$LOADER_BIN"

LOADER_SIZE=$(stat -c%s "$LOADER_BIN")
ok "Loader ready: $LOADER_BIN (${LOADER_SIZE} bytes)"

echo

# ---------- Prepare QEMU command ----------
QEMU_ARGS=(
    -m "$MEMORY"
    -smp "$SMP"
    -serial "$SERIAL"
    -no-reboot
    -no-shutdown
    -display none
    -device "loader,file=$LOADER_BIN,addr=0x100000"
)

if [ -f "$KERNEL_BIN" ]; then
    QEMU_ARGS+=(-device "loader,file=$KERNEL_BIN,addr=0x200000")
fi

if [ "$ENABLE_KVM" = "yes" ] && [ -e /dev/kvm ]; then
    QEMU_ARGS+=(-enable-kvm -cpu host)
else
    QEMU_ARGS+=(-cpu qemu64,+invtsc)
fi

# Extra args from the user
QEMU_ARGS+=("$@")

# ---------- Launch ----------
echo "────────────────────────────────────────────────────────────"
info "Launching QEMU"
echo
echo "Command:"
echo "  $QEMU ${QEMU_ARGS[*]}"
echo "────────────────────────────────────────────────────────────"
echo

# Helpful hints
echo -e "${YELLOW}Tips:${NC}"
echo "  • Press Ctrl+A then C to enter the QEMU monitor"
echo "  • In monitor: 'info registers' or 'quit'"
echo "  • For GDB: add -s -S when running this script"
echo

exec "$QEMU" "${QEMU_ARGS[@]}"
