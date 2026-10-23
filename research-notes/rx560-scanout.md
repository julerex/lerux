# What legacy boot can scan out on this RX 560

## Answer

The legacy Limine entry leaves Video Graphics Array text mode, not a linear pixel framebuffer. The scanned memory is the 80 by 25 text page at physical `0xb8000` (two bytes per cell: character, then attribute). The Sapphire RX 560 (`1002:67ff` at `01:00.0`) is the firmware boot Video Graphics Array device, so those writes appear on the High-Definition Multimedia Interface port the firmware already lit. Cursor placement is the CRT controller ports `0x3d4` and `0x3d5`. That is the mode `pc_z97_d3h` paints, and it does not require lerux to program the Polaris display engine.

A linear direct-color framebuffer can be programmed on that same Compatibility Support Module entry, and only if `sel4_32.elf` carries a Multiboot 2 framebuffer header tag (type 5). Limine 12.9.0 ignores `resolution:` unless that tag is present. With the tag, the BIOS build calls `init_vbe`, which uses interrupt `10h`:

- `AX=4F00h` reads the VESA BIOS Extensions info block.
- `AX=4F01h` reads each mode.
- `AX=4F02h` sets the mode with `BX` bit 14 set (`set_vbe_mode`), which asks for the linear framebuffer.

Limine keeps a mode only when the mode-info memory model is `0x06` (direct color) and attribute bit 7 is set (linear). The physical address is the 32-bit `framebuffer_addr` in that block. Pitch and the red, green, and blue mask sizes and shifts come from the linear fields when the VESA version major is at least 3, and from the banked fields otherwise. Limine then publishes them in the Multiboot 2 framebuffer tag as type RGB. None of those values are known for this card. The fallback sizes Limine tries are 1024×768, 800×600, and 640×480, at 32, then 24, then 16 bits per pixel, after an EDID preferred mode at 32 bits per pixel.

This image's kernel header does not request that mode. seL4 15.0.0 (`deps/versions.toml`) puts only the magic and the end tag in the Multiboot 2 header (`src/arch/x86/multiboot.S`, `CONFIG_MULTIBOOT2_HEADER`). The ISO stanza is `protocol: multiboot2` (`limine_conf` in `tools/lerux-cli/src/iso.rs`) and has no `resolution` key. On that path Limine takes `vga_textmode_init`, and if the current mode is not `0x03` it sets interrupt `10h` `AX=0003h`. The inherited framebuffer tag is EGA text: address `0xb8000`, pitch 160, 80×25, 16 bits per pixel, type EGA text.

That linear mode does not need the Unified Extensible Firmware Interface entry. The image cannot use that entry today: Limine panics with "Could not find viable load address for executable" while placing `sel4_32.elf` (linked at 1 MB) and only then builds the framebuffer tag. The Graphics Output Protocol path in `fb_init` therefore does not run. The legacy entry is the one this image already boots. What it lacks is the header tag.

If this option ROM has no linear direct-color mode, `init_vbe` fails and Limine stays on the text page. Nothing in the repository, in Limine 12.9.0, or in a read of this machine while Ubuntu is running proves that the Sapphire ROM implements such a mode. The smallest driver that would then put pixels on this HDMI is a modesetting driver for the Polaris display engine: program the display clock, the display controller, and the HDMI encoder, and aim them at a buffer in the video-memory window. `display-server` does not do that. ADR-009 gives it the QEMU software framebuffer only.

## Evidence

- Desk and the text path: `docs/boards.md` section "VGA text on the RX 560"; `docs/context.md` "Z97 monitor (phase 82)"; `userspace/pds/console-driver/src/device.rs` (`vga`, CRTC ports); `userspace/pds/hello/src/vga.rs` (`paint_cells`, attribute `0x1f00`). ADR-009 (`docs/decisions/009-interactive-surface.md`) and `docs/plan-interactive.md` keep the software framebuffer on QEMU ramfb.
- This boot: `limine_conf` is Multiboot 2 with no `resolution`. `docs/boards.md` and `tools/lerux-cli/src/iso.rs` record that the Unified Extensible Firmware Interface entry stops because `sel4_32.elf` is linked at 1 MB. A metal boot of this image has not been recorded.
- Limine 12.9.0, tag used by `deps/versions.toml`:
  - `CONFIG.md` multiboot2 `resolution`: used only when the executable requests a graphical framebuffer. `textmode: yes` forces text on BIOS.
  - `common/protos/multiboot2.c`: `fbtag` is set only for header tag `MULTIBOOT_HEADER_TAG_FRAMEBUFFER`. Without it, BIOS jumps to `textmode`. `fb_init` runs before the kernel, and the load-address panic is earlier in `multiboot2_load`.
  - `common/lib/fb.c`: BIOS `fb_init` calls `init_vbe`. Unified Extensible Firmware Interface calls `init_gop`.
  - `common/drivers/vbe.c`: `get_vbe_info`, `get_vbe_mode_info`, `set_vbe_mode`, memory model `0x06`, linear attribute bit 7.
  - `common/drivers/vga_textmode.c`: `video_mem` at `0xb8000`, mode `0x03`.
- seL4 15.0.0 `src/arch/x86/multiboot.S`: Multiboot 2 header is magic `0xe85250d6`, architecture 0, checksum, and an end tag. The graphics width, height, and depth fields exist only on the Multiboot 1 header, which this stanza does not use.
- Live PCI, read-only, no mode set: `01:00.0` class VGA, `boot_vga` 1, kernel driver `amdgpu`. Region 0 is a 64-bit prefetchable window at `0xe0000000`, size 256 MiB. Region 2 at `0xf0000000` is 2 MiB. Region 5 at `0xf7d00000` is 256 KiB. Expansion ROM at `0xc0000` is 128 KiB and disabled. `00:02.0` (`8086:0412`) has `boot_vga` 0. `card2-HDMI-A-3` is connected and enabled at 1920×1080 under `amdgpu`. That is the Linux driver scanout, not a firmware framebuffer. The option ROM was not mapped, because enabling it while `amdgpu` owns the card can hang the display.

## Unknowns

A later legacy boot has to observe the mode. Do not treat `0xe0000000` as the framebuffer address.

1. Boot the existing Compatibility Support Module entry with a Multiboot 2 framebuffer header tag added to `sel4_32.elf`, or run a real-mode probe before the kernel. Record interrupt `10h` `AX=4F00h`: the `"VESA"` signature and the version.
2. For the mode Limine actually sets, record `AX=4F01h`: attributes, memory model, `framebuffer_addr`, width, height, bits per pixel, linear bytes per scan line, and the linear red, green, and blue mask sizes and shifts. Confirm `AX=4F02h` returns `AL=4Fh` and `AH=00h`.
3. Write a recognizable pattern at that address and confirm the HDMI picture changes. Record the Multiboot 2 framebuffer tag Limine built (address, pitch, size, depth, type, masks).
4. If `4F00h` fails, or no linear direct-color mode sets, the option ROM has no firmware linear framebuffer for this port. The text page at `0xb8000` remains the only scanout this boot path leaves.

Classic Video Graphics Array mode `13h` (320×200, one byte per pixel, palette, linear at `0xa0000`) is also unobserved. Limine's multiboot2 path does not select it.
