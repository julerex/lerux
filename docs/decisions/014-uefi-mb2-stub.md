# ADR-014: UEFI metal boot via Multiboot 2 trampoline

## Status

Accepted

## Context

`sel4.elf` (64-bit) from the Microkit SDK is linked at physical 1 MiB. Limine's Multiboot 2 loader checks that the ELF load address is free **before** `ExitBootServices`. Under UEFI that range is still owned by firmware, so Limine panics with "Could not find viable load address". Relinking seL4 would require a patched SDK rebuild and would break the stock `fetch-sdk` path. QEMU `-kernel` rejects ELF64, so x86 QEMU smokes boot a Limine ISO under OVMF instead. The desk ISO uses the 32-bit kernel on the BIOS/CSM entry.

## Decision

Ship a small i386 Multiboot 2 trampoline (`support/mb2-stub/`) linked at 32 MiB. Limine loads that stub under UEFI. After Limine has exited boot services, the stub ELF-loads `sel4.elf` to 1 MiB, parks `loader.img` above the kernel, rebuilds a Multiboot 2 info block, and jumps to the Multiboot entry (still 32-bit machine state). `just iso` does not use the stub. It Multiboot 2-boots `sel4_32.elf` with module `loader.img` on the BIOS/CSM entry. QEMU x86 smokes and `lerux deploy` use:

- kernel: `mb2-stub.elf`
- module 0: `sel4.elf`
- module 1: `loader.img`

Every x86 QEMU smoke packs `lerux-uefi.iso` and boots it under OVMF (`ovmf` package or `OVMF_PATH`).

## Consequences

- `clang` is required to build the stub when packing the QEMU UEFI ISO or deploying to FAT.
- A combined OVMF firmware and `xorriso` are required for x86 QEMU smokes. Arch installs that packer with `libisoburn`. Debian installs it with `xorriso`. `just iso --boot-test` uses SeaBIOS.
- `just iso` stages and boots `sel4_32.elf` on the BIOS/CSM Limine entry. QEMU x86 smokes pack `lerux-uefi.iso` and still boot `sel4.elf` through this stub under OVMF. `lerux deploy` still copies the stub and `sel4.elf`.
- Limine `timeout: 10` keeps the metal menu on screen. QEMU smokes wait that long before the entry boots.
