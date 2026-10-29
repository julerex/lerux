---
name: uefi-grub-iso
description: >
  Use when changing the x86 ISO, GRUB menu, FAT ESP, mb2-stub, or the Swift
  panel mode line. Covers a GRUB warning, a lone underscore on the Acer, an
  underscore under the mode line, a framebuffer above 4 GiB, a newer ISO that
  still maps the spare page, and a QEMU hello line that does not prove the panel.
---

# UEFI GRUB ISO

Rules from the GRUB ISO and the Acer Swift X boot. The QEMU smoke skill still applies to the serial log.

## What GRUB loads

GRUB Multiboot 2-loads `support/mb2-stub` (`/boot/mb2-stub.elf`). Module tag 0 is `sel4.elf`. Module tag 1 is `loader.img`. The stub treats that order as identity.

`sel4.elf` stays linked at 1 MiB. GRUB loads the stub, which is linked at 32 MiB. After `ExitBootServices` the stub copies the kernel to 1 MiB. [ADR-014](../../../docs/decisions/014-uefi-mb2-stub.md) records Limine's check, which runs before `ExitBootServices` and panics with "Could not find viable load address". It does not record a GRUB measurement. Upstream Microkit loads `sel4.elf` with GRUB, and GRUB's relocator copies into that address after boot services exit. This tree keeps the stub because it paints the GOP mode line and writes the mode record before seL4. A direct GRUB load of `sel4.elf` was not tried on the Acer.

`just iso` rebuilds the Swift image, then packs it. `just iso pc_z97_d3h` does the same for the desk. `--no-build` packs the `loader.img` already on disk. `just image` follows `BOARD`, which defaults to `qemu_virt_aarch64`, so it leaves the Swift loader untouched. The stub and `grub.cfg` are the same bytes on every board. UEFI only. There is no BIOS El Torito entry.

A missing `grub-mkimage` fails before an older ISO is reused. The recipe stamp covers the menu and the module list. An ISO that is only newer than its payloads can still be the previous bootloader.

## Panel symptoms

`WARNING: no console will be available to OS` means the stub header does not request a framebuffer. GRUB's default console is EGA text. EFI cannot set that mode, so GRUB passes no framebuffer tag and the panel stays on the warning. `gfxpayload=keep` does not fix it. The header tag is type 5, flags 0, size 20, with width, height, and depth 0. GRUB then replaces `gfxpayload` with `auto` and keeps the firmware GOP mode.

A lone underscore is the firmware text cursor. GRUB has left that console, and the mode line was not written to the buffer the panel scans. On the Swift X that buffer is above 4 GiB. Multiboot enters with CR0.PG clear. Restoring the original CR0 leaves paging off, and the stores hit the 256 MiB window. Turn paging on for that paint, run it on a stack inside the identity map, mark those pages uncached, and `wbinvd` before seL4 starts. GRUB's stack is not in that map.

The first metal success is the blue bar and the mode line. On 2026-10-29 the Swift X showed `fb 0x000000fc90000000 pitch 7680 1920x1080 r16 g8 b0`. The image maps `0xfc90000000` for `0x7E9000` bytes. The `1 browser` / `2 maths` menu paints when the mode record names that address. A boot that prints a different address leaves the menu off.

An underscore under the first glyph is the firmware text cursor in the rows just below that 16-pixel bar. The glyph is 16 pixels and the clear covers 32, so a stub built after that clear leaves those rows blue. Seeing the underscore under the “f” means the USB still has the earlier stub.

## What counts as a new image

The file to flash is `build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso`. Its mtime is not the check. Read `build/swift_sfx14_41g/system.system`. The framebuffer region is `phys_addr="0xfc90000000"`, `size="0x7e9000"`, and `page_size="0x1000"`. On 2026-10-29 a 22:42 ISO was packed from a 16:29 loader that still had `phys_addr="0x1fd0000"` and `size="0x1000"`. The panel showed the stub's mode line and the menu stayed off.

The address is 2 MiB-aligned. `0x7e9000` is 2025 pages of 4 KiB and is not a multiple of 2 MiB. The template sets `page_size="0x1000"` so Microkit keeps 4 KiB pages. A Microkit assemble that accepts this region does not prove seL4 has a device untyped for it. The menu on the metal panel is the proof.

`FRAMEBUFFER_PHYS` is `0x00FC_9000_0000`. That value equals `0xfc90000000` and the mode line prints `fb 0x000000fc90000000`. `0xFC90_0000_0000` is a different address and prints `fb 0x0000fc9000000000`. Clippy `unusual_byte_groupings` rejects uneven groups.

seL4 clears the RAM frame at `0x01FE0000` when it retypes that page, so the mode record is gone by the time the console starts. A cleared record paints the recorded geometry and fills the mapped buffer with the console blue. A record that still names a different address leaves the menu off. The Swift console path does not program a VGA cursor.

When that `system.system` is the one inside the ISO and the panel still shows only the mode line, the protection domain did not run. Read the hex on the line. A different address means the mapped buffer is not the one the panel scans.

## What QEMU does not prove

OVMF's GOP buffer on `x86_64_generic` is below 4 GiB. The recorded address is `0x80000000`. `Got framebuffer info in multiboot2` and `lerux: Hello from Rust on seL4 Microkit!` prove the tag and COM1. They do not prove the high paint or the Swift panel. The metal boot of `just iso` is that proof. Read `build/smoke-logs/x86_64_generic.serial.log` for the QEMU half.

## Header, FAT, and the 1 MiB modules

GRUB scans 8192 bytes for the Multiboot magic. A 4096-byte alignment on the mode page pushes the magic to offset 8192, one byte past the window, and GRUB reports a premature end of file. Link with `-Wl,-z,max-page-size=8` and leave the mode page unaligned in C. Load addresses stay equal to the virtual addresses. `AT()` sets `p_paddr` to the load address. GRUB loads `p_paddr`.

GRUB treats `num_clusters <= 4087` as FAT12. A volume of 4085 data clusters is FAT16 in the specification and FAT12 to GRUB, and the chain walk short-reads the stub. `MIN_CLUSTERS` is 4086.

GRUB places both module files at 1 MiB. That is seL4's first load address. Zeroing that segment's BSS erases the rest of the file, including the higher segments, before they are copied. Park the loader and the kernel above `kernel_image_end` before `elf_load_kernel`.

## Host tools

Packing needs `grub-mkimage` and `xorriso`. Arch packages are `grub` and `libisoburn`. Debian packages are `grub-efi-amd64-bin` and `xorriso`. Arch OVMF is `/usr/share/edk2/x64/OVMF.4m.fd`.

xorriso warns that `/EFI/BOOT` is missing from the ISO9660 tree. The firmware boots `boot/esp.img`. The payloads live in that FAT.
