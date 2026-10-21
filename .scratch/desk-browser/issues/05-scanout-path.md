# Which scanout path does the spec commit to?

Type: grilling
Label: wayfinder:grilling
Status: open
Blocked by: 01

## Question

[What can legacy boot scan out on this RX 560?](01-rx560-scanout.md) found three paths. The legacy Limine entry leaves Video Graphics Array text at `0xb8000`. A linear framebuffer on that same Compatibility Support Module entry needs a Multiboot 2 framebuffer tag so Limine calls the VESA BIOS Extensions (`int 10h` `AX=4F02h` with the linear bit). The address and pixel masks are unknown until a boot reads the mode-info block, and nothing yet proves this option ROM has a linear mode. The Unified Extensible Firmware Interface Graphics Output Protocol path does not run: Limine rejects `sel4_32.elf` at 1 MB before it modesets. If the option ROM has no linear mode, the smallest driver that can put pixels on this HDMI is a Polaris display-engine modeset.

Which path does the spec require so `display-server` presents the browser bitmap on this desk's HDMI? Commit to the VESA linear framebuffer on the legacy entry before a boot has seen it, refuse to commit until that boot, or commit to a Polaris modeset.

The spec may amend ADR-009's QEMU-only framebuffer ceiling for this desk. It does not add a window system, and it does not move the cable to the Intel graphics ports.
