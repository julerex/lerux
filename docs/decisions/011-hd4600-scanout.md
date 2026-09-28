# ADR-011: Intel HD Graphics 4600 scanout domain

## Status

Accepted

## Date

2026-09-28

## Context

The console image is the Limine ISO for the Gigabyte Z97-D3H. Its session screen is an 800×600 XRGB8888 linear framebuffer that Limine sets before the kernel. The shell prompt and the `grok` transcript are drawn into that bitmap. The VGA text page is not the session screen.

[ADR-009](009-interactive-surface.md) allows a QEMU software framebuffer owned by `display-server` (ramfb). ramfb is not a device on this board. The graphics device in the i5-4690K is Intel HD Graphics 4600. A driver for that chip is in scope for scanout only. GPU compositors, 3D, and a discrete PCIe card stay out.

`display-server` already owns the shared bitmap. On QEMU it also programs ramfb. The console image puts the chip in its own protection domain instead.

## Decision

1. **`hd4600-driver` is a separate protection domain.** It is the only domain that programs the Intel HD Graphics 4600. It scans out the 800×600 bitmap. It does not do 3D, video decode, or a second output.

2. **`display-server` is its only client.** Apps still present through the shared bitmap and `DisplayRequest`. They never map the GPU registers. `display-server` does not program this chip.

3. **QEMU ramfb is unchanged.** `qemu_virt_aarch64_interactive` and the other ramfb boards keep `display-server` as the device owner. CI does not require the physical GPU.

4. **ADR-009's graphics ceiling changes only here.** A GPU compositor, Wayland, virtio-gpu 3D, and RPi4 HDMI stay out. A discrete card in the PCIe slot is not this driver.

## Alternatives considered

### `display-server` programs the Intel GPU

One trusted domain would own the bitmap and the chip, which is how ramfb works. Rejected for this board: the chip is a separate driver domain so device programming is not folded into the bitmap service.

### Limine's framebuffer only, with no chip driver

The bootloader mode would be the whole scanout path. Rejected because the image then depends on firmware state that a later modeset, or a reboot into a different firmware mode, can drop.

### A 3D driver or a discrete GPU

That is a compositor-sized project and a different device. Rejected. ADR-009 still forbids the compositor.

## Consequences

- The console image gains `hd4600-driver` beside `display-server`.
- Metal scanout can be tested only on this iGPU. QEMU coverage of the bitmap path stays on ramfb or the Limine framebuffer, not on an emulated HD Graphics 4600.
- Glossary: [`context.md`](../context.md) (`hd4600-driver`, console image).
