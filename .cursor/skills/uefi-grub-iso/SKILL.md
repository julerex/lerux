---
name: uefi-grub-iso
description: >
  Use when changing the x86 ISO, GRUB menu, FAT ESP, mb2-stub, or the Swift
  panel mode line. Covers a GRUB warning, a lone underscore on the Acer, a
  framebuffer above 4 GiB, and a QEMU hello line that does not prove the panel.
---

# UEFI GRUB ISO

Rules from the GRUB ISO and the Acer Swift X boot. The QEMU smoke skill still applies to the serial log.

## What GRUB loads

GRUB Multiboot 2-loads `support/mb2-stub` (`/boot/mb2-stub.elf`). Module tag 0 is `sel4.elf`. Module tag 1 is `loader.img`. The stub treats that order as identity.

`sel4.elf` stays linked at 1 MiB. GRUB loads the stub, which is linked at 32 MiB. After `ExitBootServices` the stub copies the kernel to 1 MiB. [ADR-014](../../../docs/decisions/014-uefi-mb2-stub.md) records Limine's check, which runs before `ExitBootServices` and panics with "Could not find viable load address". It does not record a GRUB measurement. Upstream Microkit loads `sel4.elf` with GRUB, and GRUB's relocator copies into that address after boot services exit. This tree keeps the stub because it paints the GOP mode line and writes the mode record before seL4. A direct GRUB load of `sel4.elf` was not tried on the Acer.

`just iso` packs the Swift image. `just iso pc_z97_d3h` packs the desk. The stub and `grub.cfg` are the same bytes on every board. UEFI only. There is no BIOS El Torito entry.

A missing `grub-mkimage` fails before an older ISO is reused. The recipe stamp covers the menu and the module list. An ISO that is only newer than its payloads can still be the previous bootloader.

## Panel symptoms

`WARNING: no console will be available to OS` means the stub header does not request a framebuffer. GRUB's default console is EGA text. EFI cannot set that mode, so GRUB passes no framebuffer tag and the panel stays on the warning. `gfxpayload=keep` does not fix it. The header tag is type 5, flags 0, size 20, with width, height, and depth 0. GRUB then replaces `gfxpayload` with `auto` and keeps the firmware GOP mode.

A lone underscore is the firmware text cursor. GRUB has left that console, and the mode line was not written to the buffer the panel scans. On the Swift X that buffer is above 4 GiB. Multiboot enters with CR0.PG clear. Restoring the original CR0 leaves paging off, and the stores hit the 256 MiB window. Turn paging on for that paint, run it on a stack inside the identity map, mark those pages uncached, and `wbinvd` before seL4 starts. GRUB's stack is not in that map.

The first metal success is the blue bar and the `fb 0x…` line. The `1 browser` / `2 maths` menu stays off until that address is the page the console driver maps. The mapped page stays `0x01FD0000` until the line is copied into the board.

## What QEMU does not prove

OVMF's GOP buffer on `x86_64_generic` is below 4 GiB. The recorded address is `0x80000000`. `Got framebuffer info in multiboot2` and `lerux: Hello from Rust on seL4 Microkit!` prove the tag and COM1. They do not prove the high paint or the Swift panel. The metal boot of `just iso` is that proof. Read `build/smoke-logs/x86_64_generic.serial.log` for the QEMU half.

## Header, FAT, and the 1 MiB modules

GRUB scans 8192 bytes for the Multiboot magic. A 4096-byte alignment on the mode page pushes the magic to offset 8192, one byte past the window, and GRUB reports a premature end of file. Link with `-Wl,-z,max-page-size=8` and leave the mode page unaligned in C. Load addresses stay equal to the virtual addresses. `AT()` sets `p_paddr` to the load address. GRUB loads `p_paddr`.

GRUB treats `num_clusters <= 4087` as FAT12. A volume of 4085 data clusters is FAT16 in the specification and FAT12 to GRUB, and the chain walk short-reads the stub. `MIN_CLUSTERS` is 4086.

GRUB places both module files at 1 MiB. That is seL4's first load address. Zeroing that segment's BSS erases the rest of the file, including the higher segments, before they are copied. Park the loader and the kernel above `kernel_image_end` before `elf_load_kernel`.

## Host tools

Packing needs `grub-mkimage` and `xorriso`. Arch packages are `grub` and `libisoburn`. Debian packages are `grub-efi-amd64-bin` and `xorriso`. Arch OVMF is `/usr/share/edk2/x64/OVMF.4m.fd`.

xorriso warns that `/EFI/BOOT` is missing from the ISO9660 tree. The firmware boots `boot/esp.img`. The payloads live in that FAT.
