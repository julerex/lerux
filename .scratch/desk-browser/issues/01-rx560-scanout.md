# What can legacy boot scan out on this RX 560?

Type: research
Label: wayfinder:research
Status: resolved

## Question

The desk monitor is HDMI on the Sapphire RX 560 (`1002:67ff` at `01:00.0`), the firmware boot VGA device. `pc_z97_d3h` writes the Video Graphics Array text page at `0xb8000` and the cursor ports `0x3d4`/`0x3d5`. The legacy Limine entry (CSM, not UEFI) is what leaves the card in that text mode, and the card scans those writes out on the HDMI the firmware already lit. The UEFI Limine entry stops before the shell because `sel4_32.elf` is linked at 1 MB. Intel HD Graphics 4600 (`00:02.0`) is not the boot VGA device. `display-server` owns QEMU ramfb only. A metal boot of this image has not been recorded. See [docs/boards.md](../../../docs/boards.md) (VGA text on the RX 560) and [ADR-009](../../../docs/decisions/009-interactive-surface.md).

What linear framebuffer, if any, can the firmware and Limine's non-UEFI path leave programmed on this card so that writes to a memory region are scanned out on the existing HDMI, without lerux programming the Polaris display engine? Report the mode-set mechanism and, if known, the framebuffer address and pixel format. Say whether reaching that mode needs a boot path this image cannot use today. If no such firmware framebuffer exists, say so, and name the smallest class of driver that would put pixels on this HDMI. Do not reboot the machine. If the repo and Limine's documentation are not enough, say what a later boot would have to observe.

## Answer

Legacy Limine leaves VGA text at `0xb8000`. A linear direct-color framebuffer on that same CSM entry needs a Multiboot 2 framebuffer header tag so Limine will call VBE (`int 10h` `AX=4F02h` with the linear bit). The address and pixel masks are not known until a boot reads the mode-info block. UEFI GOP is unreachable: Limine rejects the 1 MB link before it modesets. If the option ROM has no linear mode, the smallest driver is a Polaris display-engine modeset.

Findings: https://github.com/julerex/lerux/tree/research/rx560-scanout — `research-notes/rx560-scanout.md`
