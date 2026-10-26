# ADR-014: UEFI metal boot via Multiboot 2 trampoline

## Status

Accepted

## Context

`sel4.elf` (64-bit) from the Microkit SDK is linked at physical 1 MiB. Limine's Multiboot 2 loader checks that the ELF load address is free **before** `ExitBootServices`. Under UEFI that range is still owned by firmware, so Limine panics with "Could not find viable load address". Relinking seL4 would require a patched SDK rebuild and would break the stock `fetch-sdk` path. The metal path uses the 64-bit kernel only; `sel4_32.elf` remains solely for QEMU `-kernel` (which rejects ELF64).

## Decision

Ship a small i386 Multiboot 2 trampoline (`support/mb2-stub/`) linked at 32 MiB. Limine loads that stub under UEFI. After Limine has exited boot services, the stub ELF-loads `sel4.elf` to 1 MiB, parks `loader.img` above the kernel, rebuilds a Multiboot 2 info block, and jumps to the Multiboot entry (still 32-bit machine state). The ISO and deploy Limine stanzas use:

- kernel: `mb2-stub.elf`
- module 0: `sel4.elf`
- module 1: `loader.img`

UEFI is the documented metal path.

## Consequences

- `clang` is required to build the stub when packing an ISO or deploying to FAT.
- QEMU `-kernel` smokes still use `sel4_32.elf`; Limine ISO/FAT media use `sel4.elf`.
- Local `iso --boot-test` boots the ISO under OVMF (UEFI) only; it matches the metal USB entry.
