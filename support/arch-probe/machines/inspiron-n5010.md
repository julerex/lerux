# Dell Inspiron N5010

Probed 2026-11-14 from the Arch stick (`archlinux-2026.11.01-x86_64.iso`), BIOS boot, with `nomodeset` on the kernel line. There is no lerux image for this machine. The retrieval copy under `support/arch-probe/out/` stays gitignored.

| | |
| --- | --- |
| Product | Dell Inc. Inspiron N5010 |
| Board | Dell Inc. 0WXY9J |
| BIOS | A15, 07/19/2011 |
| CPU | Intel Core i5-460M, family 6, model 37, stepping 5 |
| Chipset | Intel HM57 (`8086:3b0b` at `00:1f.0`) |
| PCI subsystem | `1028:0447` on the chipset functions |
| Secure Boot | not published (this boot was BIOS) |
| Console | VGA text, 80 by 25. The log has no `efifb` or `vesafb` line |
| Boot VGA | `00:02.0` `8086:0046` |
| Keyboard | i8042 at `0x60`,`0x64`, IRQ 1. No IRQ 1 override |
| Serial | none |
| IOMMU | the log disables the graphics IOMMU and prints no register base |
| MCFG | not printed |
| Disk | WDC WD5000BEVT-75A0RT0, 465.8G, SATA |
| Optical | TSSTcorp DVD+/-RW TS-L633C |

The TSC calibration in that boot was 2526.993 MHz. HPET base `0xfed00000`. The I/O APIC is at `0xfec00000`, GSI 0–23. ACPI overrides bus IRQ 0 to global IRQ 2, and bus IRQ 9 to global IRQ 9.

## Devices

Addresses are the firmware assignments from this boot.

| Device | Location | Registers |
| --- | --- | --- |
| Integrated graphics, `8086:0046` | `00:02.0`, IRQ 11 | memory `0xfa400000` size `0x400000`; aperture `0xc0000000` size `0x10000000`; I/O `0xf080` size 8 |
| AHCI, `8086:3b2f` | `00:1f.2`, MSI IRQ 28 | memory `0xfbd06000` size `0x800`. Legacy I/O ports sit at `0xf020` (32 bytes) and `0xf040`–`0xf070` |
| EHCI, `8086:3b3c` | `00:1a.0`, IRQ 16 | memory `0xfbd08000` size `0x400` |
| EHCI, `8086:3b34` | `00:1d.0`, IRQ 23 | memory `0xfbd07000` size `0x400` |
| HDA, `8086:3b56` | `00:1b.0`, IRQ 30 | memory `0xfbd00000` size `0x4000` |
| BCM4313, `14e4:4727` | `12:00.0`, parent `00:1c.1`, IRQ 17 | memory `0xfbc00000` size `0x4000`. Subsystem `1028:0010` |
| RTL810xE, `10ec:8136` | `13:00.0`, parent `00:1c.2`, IRQ 18 | I/O `0xe000` size `0x100`; memory `0xd0b00000` size `0x10000` and `0xd0b10000` size `0x1000` |

Root ports `00:1c.0` and `00:1c.4` had no endpoint. There is no xHCI controller and no NVMe controller.

The built-in keyboard is the i8042. The touchpad is a Synaptics device on the same controller's AUX port, IRQ 12. This boot also had a USB HID keyboard `413c:8161` and mouse `413c:8162` on EHCI `00:1d.0`.

## Memory map

```text
System RAM       0x000000000-0x00009d3ff
device reserved  0x00009d400-0x00009ffff
gap              0x0000a0000-0x0000dffff
device reserved  0x0000e0000-0x0000fffff
System RAM       0x000100000-0x0bb5a5fff
ACPI NVS         0x0bb5a6000-0x0bb5edfff
ACPI data        0x0bb5ee000-0x0bb5f8fff
ACPI NVS         0x0bb5f9000-0x0bb5fbfff
device reserved  0x0bb5fc000-0x0bb61ffff
ACPI NVS         0x0bb620000-0x0bb628fff
device reserved  0x0bb629000-0x0bb62bfff
ACPI NVS         0x0bb62c000-0x0bb630fff
device reserved  0x0bb631000-0x0bb650fff
ACPI NVS         0x0bb651000-0x0bb693fff
System RAM       0x0bb694000-0x0bb7fffff
gap              0x0bb800000-0x0bddfffff
device reserved  0x0bde00000-0x0bfffffff
gap              0x0c0000000-0x0dfffffff
device reserved  0x0e0000000-0x0efffffff
gap              0x0f0000000-0x0fed1bfff
device reserved  0x0fed1c000-0x0fed1ffff
gap              0x0fed20000-0x0feffffff
device reserved  0x0ff000000-0x0ffffffff
System RAM       0x100000000-0x137ffffff
```

The VGA text page and the video ROM sit in the gap at `0xa0000`. The graphics aperture at `0xc0000000` sits in the gap at `0xc0000000`.
