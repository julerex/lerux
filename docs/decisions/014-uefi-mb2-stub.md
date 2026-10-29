# ADR-014: UEFI metal boot via Multiboot 2 trampoline

## Status

Accepted. The ISO loader is GRUB. The Limine finding below is unchanged, and it is not a measurement of GRUB.

## Date

2026-10-27. Amended 2026-10-29, after the Acer Swift X showed the framebuffer.

## Context

`sel4.elf` (64-bit) from the Microkit SDK is linked at physical 1 MiB. Limine's Multiboot 2 loader checks that the ELF load address is free **before** `ExitBootServices`. Under UEFI that range is still owned by firmware, so Limine panics with "Could not find viable load address". Relinking seL4 would require a patched SDK rebuild and would break the stock `fetch-sdk` path. The metal and QEMU paths use the 64-bit kernel only. QEMU `-kernel` rejects ELF64, so x86 QEMU smokes boot an ISO under OVMF.

GRUB's relocator copies an ELF into its link address after boot services exit. Upstream Microkit loads `sel4.elf` that way. That is not the check Limine failed, and this record does not claim GRUB fails it. A menu that Multiboot 2-loads `sel4.elf` directly was not tried on the Acer.

The Swift panel needs a mode line before seL4. The stub requests a framebuffer and paints the firmware GOP buffer. seL4 does not paint that panel.

## Decision

Ship a small i386 Multiboot 2 trampoline (`support/mb2-stub/`) linked at 32 MiB. GRUB loads that stub under UEFI. After GRUB has exited boot services, the stub ELF-loads `sel4.elf` to 1 MiB, parks `loader.img` above the kernel, rebuilds a Multiboot 2 info block, and jumps to the Multiboot entry (still 32-bit machine state). The ISO menu is:

- kernel: `/boot/mb2-stub.elf`
- module 0: `/boot/sel4.elf`
- module 1: `/boot/loader.img`

The stub header requests a framebuffer (tag type 5, no preferred size). GRUB keeps the firmware GOP mode and passes the framebuffer tag. The stub paints one mode line into that buffer, including when the buffer sits above 4 GiB, and writes the mode record at `0x01FE0000`.

UEFI is the documented metal path. There is no BIOS El Torito entry. Every x86 QEMU smoke packs the same ISO and boots it under OVMF.

The stub stays under GRUB because of the mode line, not because ADR-014 showed that GRUB cannot place `sel4.elf`.

## Consequences

- `clang` is required to build the stub when packing an ISO or deploying to FAT. The stub is compiled with `-mgeneral-regs-only` (no SSE, MMX, or x87). Multiboot 2 enters it in 32-bit protected mode with `CR4.OSFXSR` clear, so a vector store is an invalid-opcode fault and a triple fault resets the machine. Multiboot also enters with `CR0.PG` clear. A framebuffer above 4 GiB is painted with paging on, on a stack inside the identity map, and the stores are flushed before the jump.
- x86 QEMU smokes need `ovmf`, `xorriso`, and `grub-mkimage`. Arch packages are `libisoburn` and `grub`. Debian packages are `xorriso` and `grub-efi-amd64-bin`. Arch OVMF is `/usr/share/edk2/x64/OVMF.4m.fd`.
- The EFI system partition is a FAT16 volume with `EFI/BOOT/BOOTX64.EFI`, `boot/grub/grub.cfg`, `mb2-stub.elf`, `sel4.elf`, and `loader.img`. The ISO9660 tree carries `boot/esp.img`. xorriso warns that `/EFI/BOOT` is missing from that tree. Firmware boots the FAT.
- GRUB's menu timeout is 0, so QEMU smokes do not wait on a menu.
- The SDK still ships `sel4_32.elf`; lerux does not stage or boot it.
- On 2026-10-29 a metal boot of `just iso` on the Acer Swift X SFX14-41G showed the framebuffer. The `1 browser` / `2 maths` menu stays off until that address is the page the console driver maps.

## References

- [`.cursor/skills/uefi-grub-iso/SKILL.md`](../../.cursor/skills/uefi-grub-iso/SKILL.md)
- [boards.md](../boards.md)
