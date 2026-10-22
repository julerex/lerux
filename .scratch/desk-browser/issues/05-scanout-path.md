# Which scanout path does the spec commit to?

Type: grilling
Label: wayfinder:grilling
Status: resolved
Blocked by: 01

## Question

[What can legacy boot scan out on this RX 560?](01-rx560-scanout.md) found three paths. The legacy Limine entry leaves Video Graphics Array text at `0xb8000`. A linear framebuffer on that same Compatibility Support Module entry needs a Multiboot 2 framebuffer tag so Limine calls the VESA BIOS Extensions (`int 10h` `AX=4F02h` with the linear bit). The address and pixel masks are unknown until a boot reads the mode-info block, and nothing yet proves this option ROM has a linear mode. The Unified Extensible Firmware Interface Graphics Output Protocol path does not run: Limine rejects `sel4_32.elf` at 1 MB before it modesets. If the option ROM has no linear mode, the smallest driver that can put pixels on this HDMI is a Polaris display-engine modeset.

Which path does the spec require so `display-server` presents the browser bitmap on this desk's HDMI? Commit to the VESA linear framebuffer on the legacy entry before a boot has seen it, refuse to commit until that boot, or commit to a Polaris modeset.

The spec may amend ADR-009's QEMU-only framebuffer ceiling for this desk. It does not add a window system, and it does not move the cable to the Intel graphics ports.

## Answer

The spec uses the VESA linear framebuffer on the legacy Limine entry. Limine asks for 1024 by 768 at 32 bits per pixel and sets the mode before the kernel starts. The spec uses the linear direct-color mode the mode-info block reports, including a different size or depth. `browser-ui` receives that block's address, pitch, width, height, and pixel masks in a region it maps. It does not call the firmware.

`browser-ui` maps the framebuffer. It clears the framebuffer, paints the address line in the top 16 rows, and copies the page into the top-left of the rows beneath. Each row advances by the mode-info pitch, and each pixel uses that block's masks. `web-content` paints a shared bitmap of 1024 by 752, which is not the framebuffer, and `web-content` does not map the framebuffer. A larger mode does not grow that bitmap. Pixels that do not fit are not copied, and the rest of a larger mode stays clear. `display-server` stays on the QEMU path.

A mode shorter than 16 rows, or no linear direct-color mode, means this path failed. The spec does not then commit to a Polaris modeset. The address, pitch, and masks are recorded from a boot of this card.
