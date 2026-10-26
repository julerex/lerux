# ADR-013: Desk scanout through the Intel graphics

## Status

Accepted

## Date

2026-10-22

## Context

[ADR-009](009-interactive-surface.md) allows a QEMU software framebuffer owned by `display-server` and leaves a metal display driver out. Phase 82 paints VGA text at `0xb8000` through whichever device the firmware marked as the boot VGA device. On this desk that device was the Sapphire RX 560, because Peripherals → Initial Display Output defaults to the PCIEX16 slot, and the monitor was on that card's HDMI port.

The desk-browser spec needs the linear framebuffer Limine can set on the legacy entry. Limine makes that request by calling the primary VGA option ROM. The RX 560 option ROM's address, pitch, and color masks are not published, and the monitor cable is now on the motherboard HDMI port. The Intel HD Graphics 4600 (`00:02.0`) is the device on that port. Firmware still reports the RX 560 as the boot VGA device until Initial Display Output is `IGFX`.

## Decision

This desk scans out through the Intel HD Graphics 4600 on the motherboard HDMI port. Firmware Peripherals → Initial Display Output is `IGFX`, and Intel Processor Graphics stays enabled. Limine asks that option ROM for 1024 by 768 at 32 bits per pixel and sets the mode before the kernel starts. The spec uses the linear direct-color mode the mode-info block reports, including a different size or depth. `browser-ui` maps that block's address, pitch, width, height, and pixel masks. The firmware sets the mode. lerux does not program the Intel display engine or the Polaris display engine. The RX 560 stays in the machine and stays unused.

This amends [ADR-009](009-interactive-surface.md) for `pc_z97_d3h` only. QEMU images still present through `display-server`. A GPU compositor, Wayland, and a display-engine driver stay out.

## Alternatives considered

### Keep the VESA path on the RX 560

The option ROM fields are not a property of the RX 560 model, so a published page cannot supply them. The monitor is no longer on that card. Rejected.

### Program the Polaris display engine

That is the driver the scanout decision refused. The firmware can set a linear mode without it. Rejected.

### Program the Intel display engine

That is the same class of driver, aimed at the other device. The Intel option ROM is the firmware path already used for the text page at `0xb8000`. Rejected.

### Use the Unified Extensible Firmware Interface graphics output protocol

`sel4_32.elf` is linked at 1 MB. Limine alone rejects that load on the firmware entry before it sets a mode. The metal path uses `mb2-stub` ([ADR-014](014-uefi-mb2-stub.md)) so UEFI Limine can boot; GOP modeset for the desk browser remains a separate concern from this VGA-text shell image.

## Consequences

- A Video BIOS call reaches the Intel option ROM only after Initial Display Output is `IGFX`. On 2026-10-22 the Setup variable's byte at offset 3 was changed from 1 (PCIEX16 slot) to 0 (`IGFX`). Intel Processor Graphics was already enabled. After the reboot, firmware reports the Intel device as the boot VGA device and the RX 560 as not.
- The address, pitch, and color masks come from a boot that reads the Intel mode-info block. On 2026-10-22 that boot reported `0xd0000000`, pitch 4096, 1024 by 768, and 32 bits per pixel, with red at shift 16, green at shift 8, and blue at shift 0. The address matches the Intel prefetchable region. It is used because the Multiboot 2 tag names it.
- Once `IGFX` is the primary device, the VGA text page at `0xb8000` is scanned out on the motherboard HDMI port.
- A mode shorter than 16 rows, or no linear direct-color mode, fails this path. The spec does not then start a display-engine driver.
- [`context.md`](../context.md) and [`boards.md`](../boards.md) carry the desk terms.

## References

- [ADR-009](009-interactive-surface.md)
- [`boards.md`](../boards.md#vga-text-on-the-rx-560)
- [`context.md`](../context.md)
