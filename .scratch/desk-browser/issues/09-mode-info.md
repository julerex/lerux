# Read this card's VESA mode-info block

Type: task
Label: wayfinder:task
Status: open

## Question

[Which scanout path does the spec commit to?](05-scanout-path.md) uses the linear direct-color mode this card's option ROM reports. Those fields are unknown until a boot of this desk. QEMU's VGA BIOS is not this option ROM. The legacy Limine entry is the Compatibility Support Module entry, not the Unified Extensible Firmware Interface entry. The image's Multiboot 2 header does not request a framebuffer today. Limine 12.9.0 ignores a resolution request unless the kernel carries framebuffer header tag type 5.

The boot is a boot of this desk. Record the result on this ticket.

1. Add a Multiboot 2 framebuffer header tag to the legacy image, requesting 1024 by 768 at 32 bits per pixel, and boot the Compatibility Support Module entry. Leave the monitor on the RX 560 HDMI port.
2. From the mode Limine sets, record the physical address, pitch, width, height, bits per pixel, and the red, green, and blue mask sizes and shifts. Record the Multiboot 2 framebuffer tag Limine publishes.
3. Write a recognizable pattern at that address and confirm the HDMI picture changes.
4. If there is no linear direct-color mode, or the mode is shorter than 16 rows, record the failure and stop. Do not program the Polaris display engine. Do not treat `0xe0000000` as the framebuffer address.
