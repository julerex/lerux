# Desk probe

One USB stick holds the official Arch install image and `probe.sh`. Boot it on a machine lerux has not been brought up on. The script records the firmware framebuffer, the PCI windows, the firmware memory map, and the keyboard and serial wiring a desk image hardcodes. It loads no modules and leaves internal disks unmounted.

`just iso` is a different stick. That one is the lerux image. This recipe does not build lerux.

`support/arch-probe/fixture` is the host check for the summary. It is not a dump from a machine.

## Make the stick

Plug in the USB stick and run:

```bash
just burn-arch-probe
```

The command waits if the stick is not in yet. It then lists the USB sticks and asks for a number. The list is whole disks whose transport is `usb`. On this machine `sda` and `sdb` are internal SATA disks. The stick needs room for the Arch image (about 1.3 GiB) plus 1 GiB.

The command then:

1. Downloads the current Arch x86_64 image into `build/archiso/`, checks its sha256 against the published list, and checks the signature with the Arch keyring.
2. Asks for your password, prints the stick, and waits until you type the device path back. Any other answer stops before writing.
3. Writes the image, then adds a FAT partition named `CAPTURE` in the free space and copies `probe.sh`, `summarize.sh`, `capture.sh`, this file, and `BOOT.txt` onto it.

`just burn-arch-probe /dev/sdX` uses that stick directly. The same checks apply.

A second run uses the cached image when the published checksum still matches. The stick is erased either way, including an earlier `arch-probe-out`.

Unplug the stick when the recipe prints `wrote CAPTURE`.

## On the desk

Plug the stick in. Power on and open the firmware boot menu the same way you boot the lerux stick. Choose the USB entry. If the firmware refuses the stick, turn secure boot off.

The install image has two menus. A blue menu titled Arch Linux, whose entry says `(x86_64, BIOS)`, edits with Tab: the kernel line appears along the bottom. A plain text list whose entry says `(x86_64, UEFI)` edits with `e`. Add this to the end of the line that contains `archisobasedir`:

```text
nomodeset
```

Press Enter to boot. That leaves the EFI framebuffer in place, which is the buffer the lerux stub paints. The console is already root. Leave the stick plugged in. The Arch files stay on the stick, and `CAPTURE` is a separate partition:

```bash
mkdir -p /mnt/stick
mount /dev/disk/by-label/CAPTURE /mnt/stick
bash /mnt/stick/probe.sh
```

If the label is missing, `lsblk -f` shows it as `CAPTURE`. That partition is the large FAT one. The 109 MiB partition is the installer's EFI system partition.

The last line is `probe: wrote /mnt/stick/arch-probe-out/<stamp>`. Then run:

```bash
poweroff
```

`board.txt` in that directory is the summary. `framebuffer_source: efifb` means the address came from the firmware line. `framebuffer_source: modeset` means a display driver had already replaced that buffer, which happens when the kernel line did not include `nomodeset`.

The Swift radio trace is a different boot of this same stick. Its steps are [support/swift-linux/README.md](../swift-linux/README.md).

## Bring the results back

Plug the stick into this machine and run:

```bash
just read-arch-probe
```

The command copies each stamp into `support/arch-probe/out/<product>_<stamp>/`. When the desktop has already mounted `CAPTURE`, that mount is used and left in place. Otherwise the partition is mounted read-only and unmounted afterwards. It prints the path of `board.txt`. That directory is gitignored: it is the raw capture, including PCI config space and ACPI tables. A machine kept in the tree is copied by hand into `support/arch-probe/machines/`. The Inspiron N5010 record is [machines/inspiron-n5010.md](machines/inspiron-n5010.md).

`just read-arch-probe /dev/sdX` uses that stick directly. One CAPTURE partition is used without a prompt. Several are listed.

`board.txt` names the machine, the firmware framebuffer (physical address, size, pitch, mode, and colour shifts), the boot VGA device, the i8042, the IRQ 1 override, the serial port, and the IOMMU and MCFG bases when the log printed them. NVMe, AHCI, xHCI, Wi-Fi, and Ethernet each get a line with the BAR and the parent bridge. `e820` lines are the firmware memory map. The other files are the raw record: ACPI tables, PCI config space, `lspci`, and `dmesg`. `host.txt` leaves out serial numbers and UUIDs. Firmware filenames are recorded. The blob files stay on the Arch image.
