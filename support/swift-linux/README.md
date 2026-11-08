# Linux probe of the Swift MT7921

One boot of the official Arch install image, with the card's driver held until `capture.sh` loads it. The output is the Phase 4 record in [docs/plan-swift-wifi.md](../../docs/plan-swift-wifi.md): `lspci -vvv` for the MT7921 and its root port, the `dmesg` lines for `mt7921` and `AMD-Vi`, and the `mt76:reg_rr` / `mt76:reg_wr` trace of that probe.

Arch is enough. The releng image already contains `linux-firmware` (the MT7961 patch and RAM image), `pciutils`, and `kmod`. It has no network until this driver comes up, so the script does not install `trace-cmd`. It reads the same two events from the kernel tracer.

## What to take to the laptop

1. A USB stick with the current x86_64 Arch install image from <https://archlinux.org/download/>.
2. A second stick, formatted FAT or exFAT, holding `capture.sh` (and this file, if you want it on the machine):

```bash
cp support/swift-linux/capture.sh support/swift-linux/README.md /run/media/$USER/<stick>/
```

Ventoy works as one stick: copy the Arch ISO and `capture.sh` onto the Ventoy data partition, boot the ISO, then mount that partition and run the script from it.

Write the Arch image with the usual care. `lsblk` first, then `dd` only to that stick:

```bash
sudo dd if=archlinux-x86_64.iso of=/dev/sdX bs=4M status=progress conv=fsync
```

## Boot

Shut the laptop down and leave it off for a few seconds. A reboot out of a system that already loaded the card can leave the firmware running, and the semaphore then answers that the patch is already present.

In the firmware boot menu, choose the Arch stick. At the systemd-boot menu, highlight the install-medium entry and press `e`. At the end of the kernel line, add:

```text
modprobe.blacklist=mt7921e
```

Press Enter to boot. That stops the automatic load and still allows the script's `modprobe`. Do not use `module_blacklist=mt7921e`: the kernel then refuses the manual load as well.

If the firmware refuses the stick, turn secure boot off. The lerux stick already boots that way.

The console logs in as root.

## Capture

Mount the stick that holds the script and run it:

```bash
lsblk -f
mkdir -p /mnt/stick
mount /dev/sdY1 /mnt/stick
bash /mnt/stick/capture.sh
```

The script waits up to two minutes. Linux retries a semaphore timeout and may reset the chip before `Firmware init done`, so the first `Failed to get patch semaphore` line is not the end. The last line on the console is the verdict:

| Verdict | Meaning |
| --- | --- |
| `probe ok` | Linux finished firmware init on this unit |
| `probe failed: patch semaphore` | Linux stopped in the same place lerux does |
| `probe failed: chip reset` | The retries ended in a failed reset |
| `probe unfinished` | Two minutes passed with neither result |
| `already loaded, no trace` | The driver loaded at boot. Power off and reboot with the blacklist |

When the script says `wrote /mnt/stick/swift-linux-out`, the stick is safe to remove. If it instead points at `/run/swift-linux`, that copy is RAM: copy it onto the stick before rebooting.

## Bring back

Copy `swift-linux-out/` to `support/swift-linux/out/` in this tree. That directory is gitignored; the files name the machine. The diff against `device.rs` is what gets written into the plan.

`regs.txt` is one `rr` or `wr` per line, then the BAR offset, then the value. `mt76` traces the access after remap, so `0x7c060010` shows up as the BAR offset `map_reg` produces (`0xe0010` for that register), not as the logical address. Lost events, if the trace buffer wrapped, are counted in `regs-note.txt`.

`lspci-nvme-xxxx-before.txt` and `lspci-nvme-port-xxxx-before.txt` are the disk endpoint and its root port. The panel photograph did not resolve the endpoint, and these files are that comparison.
