# Which graphics device does the desk scan out through?

Type: grilling
Label: wayfinder:grilling
Status: resolved

## Question

[Which scanout path does the spec commit to?](05-scanout-path.md) uses the VESA linear framebuffer on the legacy Limine entry and reads the mode-info block from the firmware's primary VGA option ROM. That primary device was the Sapphire RX 560, and the monitor was on its HDMI port. The Intel HD Graphics 4600 was out of scope because nothing was plugged into it.

The cable is now on the motherboard HDMI port. Linux shows the Intel connector connected at 1920 by 1080, and the RX 560 connector disconnected. Firmware still reports the RX 560 as the boot VGA device.

Which device does the spec scan out through?

## Answer

The desk browser scans out through the Intel HD Graphics 4600 on the motherboard HDMI port. [ADR-013](../../../docs/decisions/013-desk-intel-scanout.md) records why. The VESA linear framebuffer on the legacy Limine entry stays, as recorded in [Which scanout path does the spec commit to?](05-scanout-path.md). In firmware setup, Peripherals → Initial Display Output is `IGFX`, so Limine's Video BIOS call reaches the Intel option ROM. Intel Processor Graphics stays enabled. `browser-ui` maps the linear direct-color mode that option ROM reports. The firmware sets the mode. The RX 560 stays in the machine and stays unused. Once `IGFX` is the primary device, the VGA text page at `0xb8000` is scanned out on this same HDMI port.

The address, pitch, and color masks still come from a boot that reads the Intel option ROM. The PCI regions at `0xf7800000` and `0xd0000000` are not those fields. [Read this card's VESA mode-info block](09-mode-info.md) records them. A mode shorter than 16 rows, or no linear direct-color mode, means this path failed. The spec does not then program the Intel display engine or the Polaris display engine.
