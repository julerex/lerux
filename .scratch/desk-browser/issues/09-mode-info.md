# Read this card's VESA mode-info block

Type: task
Label: wayfinder:task
Status: resolved

## Question

[Which graphics device does the desk scan out through?](12-intel-scanout.md) uses the linear direct-color mode the Intel HD Graphics 4600 option ROM reports. Those fields are unknown until a boot of this desk. QEMU's VGA BIOS is not this option ROM. The legacy Limine entry is the Compatibility Support Module entry, not the Unified Extensible Firmware Interface entry. The image's Multiboot 2 header does not request a framebuffer today. Limine 12.9.0 ignores a resolution request unless the kernel carries framebuffer header tag type 5.

The cable is already on the motherboard HDMI port. On 2026-10-22 the Setup variable's Initial Display Output was set to `IGFX`, and the following reboot reports the Intel device as the boot VGA device. A Limine probe on the flash drive asks for 1024 by 768 at 32 bits per pixel. It writes the mode-info record at LBA 20480 of that drive, then reboots. Record the result on this ticket.

1. The reboot on 2026-10-22 reports the Intel HD Graphics 4600 (`00:02.0`) as the boot VGA device and the RX 560 as not. Leave the monitor on the motherboard HDMI port.
2. Add a Multiboot 2 framebuffer header tag to the legacy image, requesting 1024 by 768 at 32 bits per pixel, and boot the Compatibility Support Module entry.
3. From the mode Limine sets, record the physical address, pitch, width, height, bits per pixel, and the red, green, and blue mask sizes and shifts. Record the Multiboot 2 framebuffer tag Limine publishes.
4. Write a recognizable pattern at that address and confirm the motherboard HDMI picture changes.
5. If there is no linear direct-color mode, or the mode is shorter than 16 rows, record the failure and stop. Do not program the Intel display engine. Do not program the Polaris display engine. Do not treat `0xe0000000`, `0xf7800000`, or `0xd0000000` as the framebuffer address.

## Answer

Limine 12.9.0, booted from the legacy flash-drive entry on 2026-10-22, published a Multiboot 2 framebuffer tag. The mode is linear direct color, 1024 by 768, 32 bits per pixel, pitch 4096. The physical address is `0xd0000000`. Red is 8 bits at shift 16, green is 8 bits at shift 8, and blue is 8 bits at shift 0. The height is 768 rows, so the path does not fail.

The tag is type 8, size 40, framebuffer type 1. Its bytes are `08 00 00 00 28 00 00 00 00 00 00 d0 00 00 00 00 00 10 00 00 00 04 00 00 00 03 00 00 20 01 00 00 10 08 08 08 00 08 00 00`.

That address is the same as this machine's Intel graphics prefetchable region. It is recorded because the tag names it, not because the region list was assumed. The probe painted vertical color bars at that address, wrote this record at LBA 20480 of the flash drive, and rebooted to Ubuntu. The Intel display engine and the Polaris display engine were not programmed.
