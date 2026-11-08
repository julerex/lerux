# Linux probe of the Swift MT7921

One USB stick holds the official Arch install image and `capture.sh`. Boot it on the Acer, with the card's driver held until the script loads it. The output is the Phase 4 record in [docs/plan-swift-wifi.md](../../docs/plan-swift-wifi.md): `lspci -vvv` for the MT7921 and its root port, the `dmesg` lines matching `mt7921|AMD-Vi`, and the `mt76:reg_rr` / `mt76:reg_wr` trace of that probe.

Arch is enough. The install image already contains `linux-firmware` (the MT7961 patch and RAM image), `pciutils`, and `kmod`. It has no network until this driver comes up, so the script does not install `trace-cmd`. It reads the same two events from the kernel tracer.

`just iso` is a different stick. That one is the lerux image. This recipe does not build lerux.

## Make the stick

Plug in the USB stick and run:

```bash
just iso-arch-and-capture
```

The command waits if the stick is not in yet. It then lists the USB sticks and asks for a number. The list is whole disks whose transport is `usb`. On this machine `sda` and `sdb` are internal SATA disks. The stick needs room for the Arch image (about 1.3 GiB) plus 1 GiB for the capture.

The command then:

1. Downloads the current Arch x86_64 image into `build/archiso/`, checks its sha256 against the published list, and checks the signature with the Arch keyring.
2. Asks for your password, prints the stick, and waits until you type the device path back. Any other answer stops before writing.
3. Writes the image, then adds a FAT partition named `CAPTURE` in the free space and copies `capture.sh`, this file, and `BOOT.txt` onto it.

`just iso-arch-and-capture /dev/sdX` uses that stick directly. The same checks apply.

A second run uses the cached image when the published checksum still matches. The stick is erased either way, including an earlier `swift-linux-out`.

Unplug the stick when the recipe prints `wrote CAPTURE`.

## On the Acer

Shut the laptop down and leave it off for a few seconds. A restart out of a system that already loaded the card can leave the firmware running, and the semaphore then answers that the patch is already present.

Plug the stick in. Power on and open the firmware boot menu the same way you boot the lerux stick. Choose the USB entry. If the firmware refuses the stick, turn secure boot off.

At the Arch menu, highlight the install-medium entry and press `e`. Add this to the end of the line that contains `archisobasedir`:

```text
modprobe.blacklist=mt7921e
```

Press Enter to boot. That stops the automatic load and still allows the script's `modprobe`. `module_blacklist=mt7921e` is a different parameter: the kernel then refuses the manual load as well.

The console is already root. Leave the stick plugged in. The Arch files stay on the stick, and `CAPTURE` is a separate partition:

```bash
mkdir -p /mnt/stick
mount /dev/disk/by-label/CAPTURE /mnt/stick
bash /mnt/stick/capture.sh
```

If the label is missing, `lsblk -f` shows it as `CAPTURE`. That partition is the large FAT one. The 109 MiB partition is the installer's EFI system partition.

The script waits up to two minutes. Linux retries a semaphore timeout and may reset the chip before `Firmware init done`, so the first `Failed to get patch semaphore` line is not the end. A line of progress prints every few seconds. The last line is the verdict:

| Verdict | Meaning |
| --- | --- |
| `probe ok` | Linux finished firmware init on this unit |
| `probe failed: patch semaphore` | Linux stopped in the same place lerux does |
| `probe failed: chip reset` | The retries ended in a failed reset |
| `probe unfinished` | Two minutes passed with neither result |
| `already loaded, no trace` | The driver loaded at boot. Power off, and boot again with the blacklist |

When the script prints `wrote /mnt/stick/swift-linux-out`, shut down:

```bash
poweroff
```

`already loaded, no trace` means this boot did not record the register trace. Power the machine off, then boot the stick again and add `modprobe.blacklist=mt7921e` before the installer starts.

## Bring the results back

Plug the stick into this machine. Mount `CAPTURE` and copy the directory into the tree. The destination is gitignored because the files name the laptop.

```bash
mkdir -p /mnt/stick support/swift-linux/out
sudo mount /dev/disk/by-label/CAPTURE /mnt/stick
cp -r --no-preserve=ownership,mode /mnt/stick/swift-linux-out/. support/swift-linux/out/
sudo umount /mnt/stick
```

`regs.txt` is one `rr` or `wr` per line, then the BAR offset, then the value. `mt76` traces the access after remap, so `0x7c060010` shows up as the BAR offset `map_reg` produces (`0xe0010` for that register), not as the logical address. Lost events, if the trace buffer wrapped, are counted in `regs-note.txt`.

`lspci-nvme-xxxx-before.txt` and `lspci-nvme-port-xxxx-before.txt` are the disk endpoint and its root port. The panel photograph did not resolve the endpoint, and these files are that comparison.
