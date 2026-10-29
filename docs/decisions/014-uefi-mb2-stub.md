# ADR-014: UEFI metal boot via Multiboot 2 trampoline

## Status

Accepted

## Context

`sel4.elf` (64-bit) from the Microkit SDK is linked at physical 1 MiB. Limine's Multiboot 2 loader checks that the ELF load address is free **before** `ExitBootServices`. Under UEFI that range is still owned by firmware, so Limine panics with "Could not find viable load address". Relinking seL4 would require a patched SDK rebuild and would break the stock `fetch-sdk` path. The metal and QEMU paths use the 64-bit kernel only. QEMU `-kernel` rejects ELF64, so x86 QEMU smokes boot the Limine ISO under OVMF instead.

## Decision

Ship a small i386 Multiboot 2 trampoline (`support/mb2-stub/`) linked at 32 MiB. Limine loads that stub under UEFI. After Limine has exited boot services, the stub ELF-loads `sel4.elf` to 1 MiB, parks `loader.img` above the kernel, rebuilds a Multiboot 2 info block, and jumps to the Multiboot entry (still 32-bit machine state). The ISO and deploy Limine stanzas use:

- kernel: `mb2-stub.elf`
- module 0: `sel4.elf`
- module 1: `loader.img`

UEFI is the documented metal path. Every x86 QEMU smoke packs the same ISO and boots it under OVMF (`ovmf` package or `OVMF_PATH`).

## Consequences

- `clang` is required to build the stub when packing an ISO or deploying to FAT. The stub is compiled with `-mgeneral-regs-only` (no SSE, MMX, or x87). Limine enters it in 32-bit protected mode with `CR4.OSFXSR` clear, so a vector store is an invalid-opcode fault and a triple fault resets the machine.
- `ovmf` and `xorriso` are required for x86 QEMU smokes (and for `iso --boot-test`). `xorriso` is the system package (`libisoburn` on Arch).
- The hybrid ISO's EFI system partition is a FAT16 volume with `BOOTX64.EFI`, `limine.conf`, `mb2-stub.elf`, `sel4.elf`, and `loader.img`. UEFI firmware that boots that partition reads the modules from it. The ISO9660 tree keeps the same files for the BIOS El Torito entry.
- The SDK still ships `sel4_32.elf`; lerux does not stage or boot it.
- Limine `timeout: 0` so QEMU smokes do not wait on the menu.
