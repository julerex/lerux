#!/usr/bin/env bash
# Build the Multiboot 2 UEFI trampoline (i386 ELF linked at 32 MiB).
set -euo pipefail
root="$(cd "$(dirname "$0")" && pwd)"
out="${1:-"$root/mb2-stub.elf"}"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

CC="${CC:-clang}"
# Multiboot 2 enters in 32-bit protected mode with CR4 = 0, so SSE is disabled.
# Clang -Os turns the 8 KiB clear of new_mbi into xorps. That is #UD, and
# Limine's IDT is still the 1 KiB real-mode table, so the #UD triple-faults
# before the stub can write COM1. -mgeneral-regs-only keeps the clear on GPRs.
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
    -mgeneral-regs-only
)

"$CC" "${CFLAGS[@]}" -c -o "$tmpdir/stub.o" "$root/stub.c"
"$CC" "${CFLAGS[@]}" -c -o "$tmpdir/entry.o" "$root/entry.S"
"$CC" "${CFLAGS[@]}" -nostdlib -static \
    -Wl,-T,"$root/stub.ld" \
    -Wl,-m,elf_i386 \
    -Wl,--build-id=none \
    -o "$out" \
    "$tmpdir/entry.o" "$tmpdir/stub.o"

# xorps here is a triple fault: see the CFLAGS comment above.
if objdump -d "$out" | grep -Eq '[[:space:]](xorps|movaps|movups|movdqa|movapd|pxor|addps)[[:space:]]'; then
    echo "mb2-stub contains an SSE opcode; Multiboot 2 enters with CR4=0" >&2
    exit 1
fi

echo "built $out"
readelf -l "$out" | head -20
