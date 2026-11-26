# What fills an Arch live installer

This is the official Arch Linux live image written straight to a [USB](../GLOSSARY.md#usb) stick. The file measured here is `archlinux-2026.11.01-x86_64.iso`, the 1 November 2026 release, Linux 7.2.11. It is 1256.5 MiB, which is 1.32 GB when a GB is 1000 × 1000 × 1000 bytes. The download page rounds that same file to 1.2 GB.

A MiB is 1024 × 1024 bytes. On a larger stick, the spare capacity stays empty.

Words used below are defined in [GLOSSARY.md](../GLOSSARY.md#arch-live-image). This image is the [Arch live image](../GLOSSARY.md#arch-live-image). The lerux [ISO](../GLOSSARY.md#iso) is a different file, the one `just make-lerux-iso-swift` writes for the Swift X.

## What lands on the stick

| Piece | MiB | What it is |
| --- | ---: | --- |
| Live root (`airootfs.sfs`) | 1044.0 | The whole live system, packed as a [squashfs](../GLOSSARY.md#squashfs) with [xz](../GLOSSARY.md#xz) |
| [Initramfs](../GLOSSARY.md#initramfs) | 82.5 | Early userspace, loaded before that root exists |
| Kernel ([vmlinuz](../GLOSSARY.md#vmlinuz)) | 16.4 | Linux 7.2.11 |
| [UEFI](../GLOSSARY.md#uefi) partition | 109.0 | A [FAT](../GLOSSARY.md#fat) filesystem with a second copy of the kernel and the initramfs |
| Boot menu and image metadata | 4.6 | [Syslinux](../GLOSSARY.md#syslinux), memtest86+, the [UEFI](../GLOSSARY.md#uefi) command shell, and the image headers |

## Why the kernel and the initramfs are copied

The stick has to boot two ways.

[BIOS](../GLOSSARY.md#bios) boot starts [syslinux](../GLOSSARY.md#syslinux). Syslinux reads `vmlinuz` (16.4 MiB) and `initramfs-linux.img` (82.5 MiB) out of the image's file tree.

[UEFI](../GLOSSARY.md#uefi) firmware reads files from a [FAT](../GLOSSARY.md#fat) partition. The image appends one, 109.0 MiB. Almost all of it is the same initramfs and the same kernel again, plus [systemd-boot](../GLOSSARY.md#systemd-boot) (0.15 MiB), a 32-bit companion loader, memtest86+, and the UEFI command shell. The two copies of the initramfs are 165 MiB of the stick. The two copies of the kernel that sit outside the live root are 33 MiB.

The live root holds a third kernel. The `linux` package installs `vmlinuz` under `/usr/lib/modules`, and the squashfs keeps that file.

## Kernel modules

The file the bootloader loads is one program: [vmlinuz](../GLOSSARY.md#vmlinuz), 16.4 MiB. It knows how to start the machine, and it knows how to load more of itself from separate files. Those files are [kernel modules](../GLOSSARY.md#kernel-module).

A kernel module is kernel code for one job, stored on its own so the running kernel can load it when the hardware shows up. The file ends in `.ko`. One module drives a disk controller. Another understands a filesystem. Another drives a network card. Arch compresses every module with [zstd](../GLOSSARY.md#zstd), so the stored name is `.ko.zst`. A later [xz](../GLOSSARY.md#xz) pass barely shrinks a file that is already zstd.

The live root carries the full module directory for Linux 7.2.11:

| Piece of the module directory | MiB |
| --- | ---: |
| Modules, already `.ko.zst` | 145 |
| The package's own copy of `vmlinuz` | 16 |
| Indexes (`modules.dep`, `modules.alias`, and the rest) | 8 |

The installer needs the full set. It has to partition a disk, format a filesystem, and bring up a network on whatever machine it finds.

The bootloader loads the initramfs before the squashfs is a filesystem, so the initramfs carries its own modules, the ones needed to find a disk and to mount the squashfs: 42.6 MiB, again already zstd, stored without a second compression layer. Those modules are on the stick twice, once in each copy of the initramfs, and the full set is inside the squashfs as well.

In this repo a [driver](../GLOSSARY.md#driver) is a [protection domain](../GLOSSARY.md#protection-domain). A kernel module is the Linux file that does the corresponding job inside this installer.

## The live root

The 1044.0 MiB file `airootfs.sfs` is a [squashfs](../GLOSSARY.md#squashfs). [xz](../GLOSSARY.md#xz) packed 2146 MiB of files into it.

About 653 MiB of those files are already compressed: device [firmware](../GLOSSARY.md#firmware) blobs ending in `.zst`, the `.ko.zst` modules, and `vmlinuz`. They occupy roughly that much of the 1044 MiB. The other 1493 MiB, programs and libraries and text, compresses to about 391 MiB. That split is estimated from file types. The 1044.0 MiB size of the squashfs file is measured.

## Device firmware

Device [firmware](../GLOSSARY.md#firmware) is a program a chip runs on its own processor. The Linux kernel copies the blob onto the chip before the chip will answer. The live image carries a blob for every common chip, so the installer can boot whatever machine it finds. On this image those blobs add up to 564 MiB before the squashfs compression, and 492 MiB of them are already zstd.

The figures below are rounded to the nearest MiB. They sum to 564.

| Firmware | MiB |
| --- | ---: |
| NVIDIA [GPU](../GLOSSARY.md#gpu), including [GSP](../GLOSSARY.md#gsp) blobs | 104 |
| Intel Wi-Fi | 95 |
| [Marvell](../GLOSSARY.md#marvell) | 77 |
| Intel [SOF](../GLOSSARY.md#sof) audio | 55 |
| Qualcomm Atheros Wi-Fi | 49 |
| [MediaTek](../GLOSSARY.md#mediatek-firmware) | 37 |
| AMD [GPU](../GLOSSARY.md#gpu) | 28 |
| Other Intel device firmware | 27 |
| Intel graphics (`i915` and `xe`) | 13 |
| Intel [CPU microcode](../GLOSSARY.md#cpu-microcode) | 17 |
| Every other vendor | 62 |

The largest single file in the root is one NVIDIA [GSP](../GLOSSARY.md#gsp) blob, 49 MiB, for the GA102 chip. The [GSP](../GLOSSARY.md#gsp) is a processor on the [GPU](../GLOSSARY.md#gpu). The kernel module loads this blob onto it, and the GSP schedules work on the graphics chip. The image also carries a 13 MiB GSP blob for the TU102 chip.

### Marvell

[Marvell](../GLOSSARY.md#marvell) is a chip vendor. The package on the image is `linux-firmware-marvell`. Arch's `linux-firmware` package treats it as optional. The live image lists it by name, so the whole package is included. The `mrvl/` directory is 77 MiB.

About 71 MiB of that directory is five copies of [Prestera](../GLOSSARY.md#prestera) switch firmware: versions 2.0, 3.0, 4.0, 4.1, and a 64-bit Arm build of 4.1. Prestera is Marvell's family of Ethernet switch chips, the kind used in network switches. The Linux `prestera` driver loads one of these images onto the switch. The five images are on the installer because the package ships as one piece.

The remaining few megabytes are older Marvell Wi-Fi:

- Libertas [USB](../GLOSSARY.md#usb), [SDIO](../GLOSSARY.md#sdio), and CompactFlash cards, including the 8388 and 8686, about 1 MiB.
- Images for the 88W8787, 8897, and 8997 Wi-Fi and Bluetooth combo chips, on [SDIO](../GLOSSARY.md#sdio), [PCIe](../GLOSSARY.md#pcie), and [USB](../GLOSSARY.md#usb).
- `mwl8k` cards, the 88W8366, 8687, and 8764, under half a megabyte.
- `mwlwifi` cards, the 88W8864 and 88W8897, under half a megabyte.

### SOF

[SOF](../GLOSSARY.md#sof) is Sound Open Firmware, an open firmware project for an audio [DSP](../GLOSSARY.md#dsp). A DSP is a small processor beside the main processor that runs the audio paths: microphone, speakers, and audio sent to a screen. The Linux driver `snd-sof` copies a firmware image onto that DSP, then loads a topology file. The topology names that machine's wiring: which amplifier, which headset codec, which screen output.

On this image the Intel files are 55 MiB, which is the `sof-firmware` package:

| Directory under `intel/` | MiB | Role |
| --- | ---: | --- |
| `sof-ipc4` | 23 | Firmware for the newer Intel audio DSP |
| `sof-ipc4-tplg` | 14 | Topology files for that newer DSP |
| `sof-tplg` | 9 | Topology files for the earlier DSP |
| `sof` | 9 | Firmware for the earlier DSP |

`sof-ipc4` is the newer command protocol, used by Intel's later audio DSPs. The topology directories are large because each laptop design gets its own file. MediaTek's SOF files, for the audio DSP on the MT8186, MT8188, and MT8195, live in the MediaTek package and are a handful of small files. They are separate from this 55 MiB.

### MediaTek

The 37 MiB `mediatek/` directory is the `linux-firmware-mediatek` package. Arch describes it as firmware for MediaTek and Ralink devices. It is the whole vendor set. The [MT7921](../GLOSSARY.md#mt7921) in the Swift X uses one pair of files inside it, `WIFI_MT7961_patch_mcu` and `WIFI_RAM_CODE_MT7961`, plus the matching Bluetooth image `BT_RAM_CODE_MT7961`.

The package covers four groups of devices:

- **Wi-Fi chips in the [mt76](../GLOSSARY.md#mt76) family.** Older [USB](../GLOSSARY.md#usb) and [PCI](../GLOSSARY.md#pci) adapters such as the MT7601U, MT7610, MT7615, MT7662, MT7663, and MT7668. Wi-Fi 6 chips MT7915 and MT7916. Client combo chips MT7921 (the files are named MT7961), MT7922, and MT7925, each with a Bluetooth firmware image for the radio on the same chip.
- **Router and access-point chips.** Filogic router [SoCs](../GLOSSARY.md#soc) MT7981, MT7986, and MT7988, including firmware for the physical layer of their 2.5 gigabit Ethernet ports. Wi-Fi 7 access-point chips in the MT7996 family.
- **Older [Ralink](../GLOSSARY.md#ralink) Wi-Fi.** MediaTek bought Ralink. The package still carries the [PCI](../GLOSSARY.md#pci) and [USB](../GLOSSARY.md#usb) blobs from that line: RT2860, RT2870, RT3070, RT3290, and the other `rt*.bin` files. Those files sit beside the `mediatek/` directory and are small.
- **Companion processors on MediaTek Chromebook [SoCs](../GLOSSARY.md#soc).** The system control processor on the MT8183, MT8186, MT8188, MT8192, and MT8195. The video processor on the MT8173. The small [SOF](../GLOSSARY.md#sof) set for the MT8186, MT8188, and MT8195 audio [DSPs](../GLOSSARY.md#dsp).

The chip names are the ones in the November 2026 package file list. This image carries `linux-firmware` 20260916, one upload earlier. The directory size, 37 MiB, was measured on the image.

## What compression still shrinks

These are the other files in the live root, measured before xz. Together they are the 1493 MiB that compresses to about 391 MiB. Text shrinks more than a shared library does. The rows are rounded to the nearest MiB.

| Contents | MiB | What it is |
| --- | ---: | --- |
| Shared libraries | 281 | The `.so` files programs load, including the Unicode data library `libicudata` at 32 MiB |
| Programs | 254 | Installer and rescue tools. The Sequoia signature tool `sq` is 21 MiB, and `xgettext` is 13 MiB |
| Data files | 213 | Perl, nmap's probe database, interface description files, espeak speech data, [GRUB](../GLOSSARY.md#grub) files, fonts |
| Python | 155 | The standard library plus `archinstall` and the libraries it imports |
| Translations | 148 | Message catalogs and locale data |
| Static libraries | 82 | `.a` archives, mostly Boost, glibc, and binutils, pulled in by the packages that ship them |
| C headers | 62 | The `.h` files from the same packages |
| Documentation | 47 | `/usr/share/doc` |
| Man pages | 45 | `/usr/share/man` |
| Vim runtime | 39 | Syntax files and help text for the editor on the image |
| Firmware that is not precompressed, module indexes, udev data, and small text | 167 | The rest of the 1493 MiB |

The [GRUB](../GLOSSARY.md#grub) files in that data-file row are the GRUB package inside the live system. The running image boots with [syslinux](../GLOSSARY.md#syslinux) and [systemd-boot](../GLOSSARY.md#systemd-boot). The package is there so the installer can write a bootloader onto the disk it is installing.

## The initramfs

Each copy is 82.5 MiB. The two copies are 165 MiB of the stick.

| Inside one initramfs | MiB on the stick |
| --- | ---: |
| [Kernel modules](../GLOSSARY.md#kernel-module) for disks, filesystems, and a few network cards | 42.6 |
| Intel [CPU microcode](../GLOSSARY.md#cpu-microcode) | 16.9 |
| A few network and storage firmware blobs | 4.2 |
| AMD [CPU microcode](../GLOSSARY.md#cpu-microcode) | 0.3 |
| [xz](../GLOSSARY.md#xz) archive of the rest of early userspace | 18.3 |
| Archive headers | 0.2 |

[CPU microcode](../GLOSSARY.md#cpu-microcode) is a patch the processor applies to itself during boot. Intel's patch is 16.9 MiB and is stored in the clear at the front of the initramfs, then stored again inside the squashfs. AMD's patch is 0.3 MiB.

The xz archive expands to 65 MiB. Libraries are 51 MiB of that. The [udev hardware database](../GLOSSARY.md#udev-hardware-database) is 13.2 MiB of those libraries. The `bcachefs` filesystem tool is 8.9 MiB of the 13 MiB of programs in the same archive. On the stick, that whole expanded tree occupies 18.3 MiB.

### The udev hardware database

udev is the device manager in systemd. When a keyboard, a disk, or a [USB](../GLOSSARY.md#usb) device appears, udev names it and attaches a set of properties: the vendor, the model, a keyboard layout.

It does that lookup in one compiled file, `/usr/lib/udev/hwdb.bin`. The readable sources are the `*.hwdb` text files under `/usr/lib/udev/hwdb.d/`. `systemd-hwdb` compiles them into `hwdb.bin`. The records cover [PCI](../GLOSSARY.md#pci) identifiers, [USB](../GLOSSARY.md#usb) identifiers, and input devices. A PCI id becomes a vendor string. A keyboard's id becomes a layout.

The file on this image is 13.2 MiB. It lives in the compressed half of the initramfs, so its share of the 82.5 MiB file is a slice of the 18.3 MiB xz archive.

## How this was counted

The byte sizes of `airootfs.sfs`, `vmlinuz`, the initramfs, and the FAT partition were read from the 2026.11.01 image. The file is 1,317,568,512 bytes, the same length the Arch mirror published for that release. File sizes inside the squashfs are the uncompressed lengths from a squashfs listing. The initramfs was split into its leading uncompressed archive and the [xz](../GLOSSARY.md#xz) archive that follows it.

The 653 MiB and 391 MiB figures treat every `.zst` file, every `.ko.zst` module, and `vmlinuz` as already at its final size.
