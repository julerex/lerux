#!/usr/bin/env bash
# Build the Multiboot 2 UEFI trampoline (i386 ELF linked at 32 MiB).
set -euo pipefail
root="$(cd "$(dirname "$0")" && pwd)"
out="${1:-"$root/mb2-stub.elf"}"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

CC="${CC:-clang}"
CFLAGS=(
    --target=i386-unknown-linux-gnu
    -std=c11
    -ffreestanding
    -fno-pic
    -fno-pie
    -fno-stack-protector
    -fno-builtin
    -Wall
    -Wextra
    -Werror
    -Os
    -m32
)
# Limine enters this stub in 32-bit protected mode with SSE left off
# (CR4.OSFXSR clear). A vector store is #UD, then a triple fault and a
# reset, which is what a laptop shows as a hang after the module lines.
INTEGER_ONLY=(
    -mgeneral-regs-only
    -mno-sse
    -mno-sse2
    -mno-mmx
    -mno-80387
)

"$CC" "${CFLAGS[@]}" "${INTEGER_ONLY[@]}" -c -o "$tmpdir/stub.o" "$root/stub.c"
"$CC" "${CFLAGS[@]}" "${INTEGER_ONLY[@]}" -c -o "$tmpdir/fb_paint.o" "$root/fb_paint.c"
"$CC" "${CFLAGS[@]}" -c -o "$tmpdir/entry.o" "$root/entry.S"
"$CC" "${CFLAGS[@]}" -nostdlib -static \
    -Wl,-T,"$root/stub.ld" \
    -Wl,-m,elf_i386 \
    -Wl,--build-id=none \
    -o "$out" \
    "$tmpdir/entry.o" "$tmpdir/stub.o" "$tmpdir/fb_paint.o"

echo "built $out"
readelf -l "$out" | head -20
