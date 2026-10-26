# ADR-014: UEFI metal boot via Multiboot 2 trampoline

## Status

Accepted

## Context

`sel4_32.elf` from the Microkit SDK is linked at physical 1 MiB. Limine's Multiboot 2 loader checks that the ELF load address is free **before** `ExitBootServices`. Under UEFI that range is still owned by firmware, so Limine panics with "Could not find viable load address". The legacy/CSM USB entry worked; the UEFI Limine entry did not. Relinking seL4 would require a patched SDK rebuild and would break the stock `fetch-sdk` path.

## Decision

Ship a small i386 Multiboot 2 trampoline (`support/mb2-stub/`) linked at 32 MiB. Limine loads that stub under both UEFI and BIOS. After Limine has exited boot services, the stub ELF-loads `sel4_32.elf` to 1 MiB, parks `loader.img` above the kernel, rebuilds a Multiboot 2 info block, and jumps to the kernel entry. The ISO and deploy Limine stanzas use:

- kernel: `mb2-stub.elf`
- module 0: `sel4_32.elf`
- module 1: `loader.img`

UEFI is the documented metal path. CSM remains supported with the same media.

## Consequences

- `clang` is required to build the stub when packing an ISO or deploying to FAT.
- QEMU `-kernel sel4_32.elf` smokes are unchanged; only Limine ISO/FAT media use the stub.
- Local `iso --boot-test` runs SeaBIOS and, when OVMF is present, UEFI.
- GRUB Multiboot 2 snippets still point at `sel4_32.elf` directly and are not the UEFI metal path.
