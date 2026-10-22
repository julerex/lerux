# Read this card's VESA mode-info block

Type: task
Label: wayfinder:task
Status: open

## Question

[Which graphics device does the desk scan out through?](12-intel-scanout.md) uses the linear direct-color mode the Intel HD Graphics 4600 option ROM reports. Those fields are unknown until a boot of this desk. QEMU's VGA BIOS is not this option ROM. The legacy Limine entry is the Compatibility Support Module entry, not the Unified Extensible Firmware Interface entry. The image's Multiboot 2 header does not request a framebuffer today. Limine 12.9.0 ignores a resolution request unless the kernel carries framebuffer header tag type 5.

The cable is already on the motherboard HDMI port. On 2026-10-22 the Setup variable's Initial Display Output was set to `IGFX`. The boot that follows that write is a boot of this desk. Record the result on this ticket.

1. The reboot on 2026-10-22 reports the Intel HD Graphics 4600 (`00:02.0`) as the boot VGA device and the RX 560 as not. Leave the monitor on the motherboard HDMI port.
2. Add a Multiboot 2 framebuffer header tag to the legacy image, requesting 1024 by 768 at 32 bits per pixel, and boot the Compatibility Support Module entry.
3. From the mode Limine sets, record the physical address, pitch, width, height, bits per pixel, and the red, green, and blue mask sizes and shifts. Record the Multiboot 2 framebuffer tag Limine publishes.
4. Write a recognizable pattern at that address and confirm the motherboard HDMI picture changes.
5. If there is no linear direct-color mode, or the mode is shorter than 16 rows, record the failure and stop. Do not program the Intel display engine. Do not program the Polaris display engine. Do not treat `0xe0000000`, `0xf7800000`, or `0xd0000000` as the framebuffer address.
