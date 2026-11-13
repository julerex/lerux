# PLAN — Arch parity on the real boards

Last updated: 2026-11-14.

This is the living checklist for the three machines that already have a lerux image. It does not replace the historical phases. Phases 50–60 stay in [`plan-arch.md`](plan-arch.md). The LionsOS rewrite stays in [`plan-lionsos.md`](plan-lionsos.md). The Ladybird engine stays in [`../lerux-ladybird/PLAN.md`](../lerux-ladybird/PLAN.md). The Swift radio sequence stays in [`plan-swift-wifi.md`](plan-swift-wifi.md).

Related: [`plan.md`](plan.md), [`context.md`](context.md), [`boards.md`](boards.md), [`platforms.md`](platforms.md), [ADR-009](decisions/009-interactive-surface.md), [ADR-011](decisions/011-lionsos-structures.md), [ADR-012](decisions/012-ordinary-webassembly.md).

## What parity means

Arch’s workflow on these machines, reimplemented as static Microkit protection domains. A finished board can be flashed, boots to a console, reads and writes a hierarchical filesystem on its own disk, gets an address on its real network device, resolves a name, fetches and edits a file, shows time and logs, takes a host `lerux package` pin plus rebuild, and opens the Rust browser on that board’s pixels.

That is the bar. A QEMU line does not close a metal milestone. A panel photo or a hardware serial log does.

“Arch-like” in [`context.md`](context.md) already means workflow, not binary compatibility. This plan keeps that ceiling. Unmodified `bash`, `pacman`, and `firefox` are not milestones.

### The three boards

- **Acer Swift X** (`swift_sfx14_41g`) is the main target. The image is `just iso`. There is no QEMU model and no COM port. The console is the firmware GOP panel. The keyboard is the built-in i8042.
- **Gigabyte Z97-D3H** (`pc_z97_d3h`) is the desk. The image is `just iso pc_z97_d3h`. QEMU covers the VGA menu (`just test-x86-console`). The onboard Ethernet port is the I217-V. The RTL8852BE card (`pc_z97_d3h_rtl8852be`) stays a configuration-space probe.
- **Raspberry Pi 4** (`rpi4b_4gb`, workstation profile `rpi4b_4gb_workstation`) is secondary. It stays behind the two x86 boards. No second single-board computer is added until this one is signed off.

### Status words

- **not started.** No exit evidence for this plan.
- **in review.** A pull request exists and the exit is not on `main`.
- **done.** The exit line in this file was met, and the proof is named.

Milestones 1–10, 12, and 14 start as **not started**. Milestone 11 starts as **not started** for the metal browser. Ladybird milestones through the request server are already merged. Navigables is **in review** and is not a metal exit. Milestone 13 is stretch and is **not started**. It is not part of the bar. Milestone 14 is part of the bar: a lerux command for every rescue and diagnostic tool on the Arch live installer.

## Already true

Do not redo this work.

QEMU already has the sDDF serial path, the FAT filesystem image, virtio net, fetch, and a small browser. LionsOS milestones 1–21 and 18 are done. x86 QEMU has an AHCI disk and an e1000e NIC:

- `just test-x86-ahci` prints `ahci: port 0`, then `lerux shell ready`, then the scripted shell lines. The controller it accepts is class `0x010601`, and on that QEMU it is `8086:2922`.
- `just test-x86-e1000e` prints `e1000e: ready`, then `ip 10.0.2.15`, then `ping ok`.
- `just test-x86-fetch-native` prints `ahci: port 0`, `e1000e: ready`, and `fetch ok`, and the saved file contains `lerux-http-fixture`.

The Swift panel has shown the mode line, the menu, and the six PCI lines (LionsOS milestones 24–26). On 2026-10-29 the mode line was `fb 0x000000fc90000000 pitch 7680 1920x1080 r16 g8 b0`. On 2026-10-30 the menu showed `hello lerux`, `1 browser`, and `2 maths`. The same day the probe painted `01:00.0 10de:1f9d`, `02:00.0 1c5c:174a`, `03:00.0 14c3:7961`, `04:00.0 1002:164c`, `04:00.3 1022:1639`, and `04:00.4 1022:1638`. `browser` on that menu still printed `browser unavailable` in the LionsOS 25 write-up. A later Lenna page paints into the firmware framebuffer when the menu choice is browser. That local paint is not the fetched parity page in Milestone 11.

The Z97 QEMU console runs the menu and `maths` (`just test-x86-console`). Host profiles and `lerux package search|install|remove|upgrade` already exist from phases 55 and 60. Image digests (`loader.img.sha256`, `lerux digest`, `lerux verify-image`) already exist as a host-side track.

The live filesystem is still the LionsOS FAT server, not the deleted postcard `fs-server`. A path is one 8.3 component in the root, or an optional leading slash and exactly two 8.3 components opened read-only. A nested create or write is rejected. One file is open. A write starts at offset 0 and is at most 512 bytes. After format, `fatfs` creates `CONFIG/HOSTNAME` with the bytes `lerux` when that file is missing. That is not a home directory.

## Shared role set

Every board image, once its drivers exist, composes the same roles:

- console (panel pixels on the Swift, VGA text on the Z97, serial on the Pi)
- block driver
- filesystem server
- network driver
- network copier and virtualisers, where the sDDF net path is used
- shell

Device protection domains are the only board-specific programs. The shell, the filesystem server, and the network client do not grow a second copy per board. A board feature flag selects the driver and the system template. Channel numbers still come from the profile manifest.

```
Milestone 1 filesystem
        │
        ├── Milestone 2 shell ── Milestone 3 config, time, logs
        │                              │
        │                              └── Milestone 4 host rolling image
        │
        ├── Z97: 5 SATA ── 6 I217 ── 7 desk session
        │
        ├── Swift: 8 Wi-Fi ── 9 read-only disk ── 10 console session
        │
        ├── Milestone 11 browser (QEMU, then Z97, then Swift)
        │
        ├── Milestone 12 Pi lab (after the x86 sessions)
        │
        └── Milestone 14 rescue and diagnostic commands
```

Milestone 13 sits after the bar.

## Reuse map

| Area | Path |
|------|------|
| Domain language | [`context.md`](context.md) |
| Board install steps | [`boards.md`](boards.md) |
| Device names | [`platforms.md`](platforms.md), [`../GLOSSARY.md`](../GLOSSARY.md) |
| sDDF shell | `userspace/pds/sddf-shell` (`sddf-shell.elf`) |
| Desk and Swift menu | `userspace/pds/console-shell` (`console-shell.elf`) |
| FAT server | `userspace/pds/sddf-fatfs` |
| Filesystem queue layouts | `userspace/crates/lerux-sddf` (`fs_image`, `blk_image`, `net_image`) |
| File-descriptor library | `userspace/crates/lerux-posix` |
| AHCI | `userspace/pds/sddf-ahci-driver` |
| e1000e / I217 class | `userspace/pds/sddf-e1000e-driver` |
| Swift radio | `userspace/pds/swift-mt7921` |
| Swift disk | `userspace/pds/swift-nvme` |
| Swift PCI probe | `userspace/pds/swift-pci-probe` |
| Swift Bluetooth | `userspace/pds/swift-bt` |
| Pi network and disk | `genet-driver`, `emmc2-driver` |
| Browser engine | `lerux-ladybird`, `userspace/crates/lerux-web`, `userspace/crates/lerux-html` |
| Templates | `userspace/systems/templates/console-swift.system.template`, `console-pc.system.template`, and the fetch / filesystem templates the QEMU images already use |
| Profiles and pins | `support/profiles/`, `support/packages/`, `support/package-pins.toml` |
| Smoke lines | `support/smoke-expects.toml` |
| Host CLI | `tools/lerux-cli/` |

Do not add a postcard message. New queue code uses `lerux-sddf`. Do not invent a LionsOS struct that is missing from the pinned 0.4.0 headers.

## Milestone 1 — Filesystem that can hold a home directory

**Status:** not started.

**Arch capability.** A home directory is a tree of named files larger than one sector. Arch assumes `mkdir`, nested paths, rename, remove, and files big enough for a page or an edit buffer. The current FAT server is a smoke volume: 8.3 names, one open file, and a 512-byte write from offset 0.

**Boards.** All three, after each board has a block driver. The proof is QEMU first. Metal disks are not formatted here.

**Protection domains and templates.** `sddf-fatfs` and the shell that speaks `fs_cmd_t` through `lerux-posix`. The filesystem image template and the x86 AHCI filesystem image. The block queue layout in `lerux-sddf` stays. Do not bring back `lerux-fat` or `lerux-interface-types`.

**Steps.**

- Keep the existing commands: open, close, read, write, stat, rename, remove, mkdir, directory open, directory read, directory close.
- Accept a nested path of more than two components for create, write, and read. `.`, an empty path, and `/` still name the root.
- Allow more than one file open at a time, with a small fixed cap. A second open past the cap still returns `FS_STATUS_TOO_MANY_OPEN_FILES`.
- A write may start at a non-zero offset and may span more than one 512-byte sector, up to a documented cap large enough for a fetched HTML body and an edit buffer. One cluster per file is no longer the cap.
- `rename` and `remove` work on a name that is not in the root. `rm` of a directory that still holds another name still returns `FS_STATUS_NOT_EMPTY`.
- Creating a file still marks its clusters in both tables. A second create of the same name still returns `FS_STATUS_ALREADY_EXISTS`.
- Do not format a disk that still holds another operating system. QEMU uses `support/disk.img`. A metal disk is out of scope until Milestones 5 and 9, and those milestones read before they write.

**QEMU.** `just test-fs-sddf` and `just test-x86-ahci`. Extend the scripted lines so a nested `mkdir`, a write past 512 bytes, a `cat` of that file, and an `rm` of an empty directory all succeed. Golden strings live in `support/smoke-expects.toml`. Keep `lerux shell ready` out of `expect` and `assert` messages.

**Metal evidence.** None. This milestone closes on QEMU.

**Exit.** The shell on the filesystem image and on the AHCI image creates a nested directory, writes a file larger than 512 bytes, reads it back, renames it, and removes it. The old 8.3 root smoke still passes.

## Milestone 2 — Daily shell on the sDDF console

**Status:** not started.

**Arch capability.** Base packages: enough built-ins to administer files, the address, the clock, and logs without knowing a channel id. Phase 53 met that bar on the postcard shell. Milestone 17 deleted that shell. The LionsOS shell has a smaller command set (`mkdir`, `write`, `mv`, `rm`, `ls`, `stat`, `cat`, `config get hostname`, `date`, `ps`, `qos`, `dmesg`, `reboot`, and on the fetch image `fetch`, `calc`, `backup`, `source`, `cert`, `edit`, `chat`).

**Boards.** QEMU filesystem image, QEMU fetch image, and the x86 native fetch image. The Swift and Z97 menus stay as they are until Milestones 7 and 10.

**Protection domains and templates.** `sddf-shell` only. Do not add `lerux-logging` or `lerux-interface-types`. The fetch-image worker stack and heap stay as they are. Assertions stay on the root.

**Steps.**

- One command table, shared by the filesystem image and the fetch image. A command that needs a socket is absent on the filesystem image and prints the command name plus `err` there, rather than a second shell.
- File commands from Milestone 1: `mkdir`, `rm`, `mv`, `stat`, `cat`, `ls`. Add `df` from the volume the filesystem server already knows.
- `ip` prints the address the network client already has. On an image with no network role it prints `ip err`.
- `date` stays the protected timer call where the image has a timer. An image with no timer prints `date err`.
- `dmesg` prints the shell log and does not record the dump.
- `config` is Milestone 3. This milestone only keeps `config get hostname` working.
- `edit` creates a file through `lerux-posix`. Create of an existing name is rejected.
- `reboot` prints `reboot`. Where the board sets `qemu.qmp`, the host still sends `system_reset`.
- A line is at most 80 bytes. A failed command prints its name plus `err`. An unknown or empty command prints `err`.
- Success lines stay the ones the filesystem image already uses (`mkdir ok`, `write ok`, `mv ok`, `rm ok`, `ls ok`, `stat size N`, `stat dir`), plus the new lines this milestone adds. Put those new lines in `support/smoke-expects.toml`.

**QEMU.** `just test-fs-sddf`, `just test-fetch-sddf`, and `just test-x86-fetch-native`. Each new line is a script step after `lerux shell ready`.

**Metal evidence.** None.

**Exit.** A scripted QEMU session on the filesystem image and on the native x86 fetch image runs `mkdir`, `rm`, `mv`, `stat`, `df`, `ip` or `ip err`, `date` or `date err`, `dmesg`, `edit`, and `reboot` without a channel number.

## Milestone 3 — Config, time, and logs on that shell

**Status:** not started.

**Arch capability.** `/etc` plus `timedatectl` plus the journal, at the scale of one operator. Hostname and network mode survive reboot. The clock is the board clock. A failed boot leaves a log that can be read from the shell.

**Boards.** The QEMU filesystem image (timer and FAT) and the x86 native fetch image (disk, network, no promise of a timer). Metal clocks wait until Milestones 7 and 10 compose this shell.

**Protection domains and templates.** `sddf-shell`, `sddf-fatfs`, and the timer driver already on the filesystem image. The shell config page is the serial client page, the filesystem shell page, and, on the fetch image, the network shell page. The filesystem-image feature still appends the timer client page and the schedule page. Do not enable the fetch feature and the filesystem feature in one build.

**Steps.**

- Persist hostname in the file the filesystem server already seeds (`CONFIG/HOSTNAME`, bytes `lerux` when missing). `config get hostname` prints those bytes. `config set hostname NAME` writes them and the next boot prints them again.
- Persist a network mode key beside that file. `config get net.mode` prints `dhcp` or `static`. A static mode stores the address the shell already understands. Applying it uses the network client on the fetch image. An image with no network role prints `config err` for `net.mode`.
- Any other key prints `config err`.
- `date` reads the timer the image has. A reply of 0 nanoseconds prints `date err`. The x86 native image grows a clock only if that image already has a timer protection domain. Do not invent a second timer driver for this milestone.
- `dmesg` prints the 2048-byte shell log. A boot that fails before the shell still needs the one-shot driver lines (`fatfs: serving`, `ahci: port 0`, `e1000e: ready`, and the virtualiser lines). Leave those lines.
- Secrets stay a later stretch. This milestone does not encrypt a file.

**QEMU.** `just test-fs-sddf` for hostname across a reboot the smoke can script, and `just test-x86-fetch-native` for `net.mode`. If the native image has no timer, the expected line is `date err`, and that is a pass.

**Metal evidence.** None.

**Exit.** Hostname set in one boot is the hostname printed after reboot. `net.mode` round-trips on the native fetch image. `date` and `dmesg` match the rules above.

## Milestone 4 — Host rolling image

**Status:** not started.

**Arch capability.** `pacman -Syu` is a rolling update. On lerux the update is a host pin plus a rebuild of `loader.img`, not a package installed inside the guest. Phase 55 and track C of phase 60 built the commands. This milestone makes the three board install pages say the same sequence.

**Boards.** Swift, Z97, and Pi, as documentation and a host command. No guest code is required if the commands already rebuild a profile.

**Protection domains and templates.** None in the guest. `tools/lerux-cli/` (`package`, `image`, `digest`, `verify-image`) and the install sections of [`boards.md`](boards.md).

**Steps.**

- `lerux package install` and `lerux package upgrade` rebuild the selected board profile and print the pin digest.
- The image build writes `loader.img.sha256`. `lerux verify-image` checks it before a flash instruction is printed.
- [`boards.md`](boards.md) states one sequence for each board: build the image, verify the digest, write the media, boot. Swift stays `just iso` and the `dd` line. Z97 stays `just iso pc_z97_d3h` or `just deploy-pc`. Pi stays `just deploy-rpi4`. Do not invent a fourth writer.
- Say, on each of those three pages, that the guest cannot install a package. The operator rebuilds on the host.
- Do not add an on-device package manager.

**QEMU.** `just check`, which already runs the host package tests. Add a host test only if upgrade does not already rebuild a profile and write the digest.

**Metal evidence.** A verified image file on the host. Flashing is the operator’s step and is not this exit.

**Exit.** One documented sequence per board, and a host test or the existing CLI test shows install or upgrade writing a pin and a `loader.img.sha256`.

## Milestone 5 — Z97 Serial ATA identity

**Status:** not started.

**Arch capability.** The desk disk is a real controller, not QEMU `ich9-ahci`. Arch reads that disk. lerux does not know the Z97 vendor and device id yet. [`platforms.md`](platforms.md) records the identifier as not taken.

**Boards.** `pc_z97_d3h` only. QEMU `x86_64_generic_fs_ahci` stays the regression and does not close the milestone.

**Protection domains and templates.** `sddf-ahci-driver`. The desk image template, once the identifier is known. The driver still accepts class `0x010601` and `8086:2922`.

**Steps.**

- Read the desk controller’s configuration space from the metal boot and record vendor, device, class, and location in [`platforms.md`](platforms.md) and [`context.md`](context.md).
- Teach `sddf-ahci-driver` that identifier beside `8086:2922`. Reject anything else.
- Keep one command at a time on port 0. A 4096-byte transfer is still eight 512-byte sectors. A flush still reports success and submits no device command.
- The metal disk is not formatted. The first metal read is a file on a spare partition, after the operator names that partition. The Windows volume is not mounted for write and is not formatted.
- The ready line stays `ahci: port 0`, printed once.

**QEMU.** `just test-x86-ahci` still passes with `8086:2922`.

**Metal evidence.** The desk serial log or the VGA console shows `ahci: port 0`, then a shell read of one named file on the spare partition. A boot that only matches class `0x010601` on QEMU is not this evidence.

**Exit.** The driver accepts the recorded Z97 identifier, the QEMU smoke still passes, and the desk reads one file from a spare partition without writing the Windows volume.

## Milestone 6 — Z97 I217-V

**Status:** not started.

**Arch capability.** The onboard Ethernet port gets an address from the router. The QEMU 82574 (`8086:10d3`) is a different physical-layer path. [`context.md`](context.md) already records the I217-V as `8086:153b` at `00:19.0`, revision `0`, subsystem `1458:e000`, memory `0xf7e00000` (128 KiB) and `0xf7e3c000` (4 KiB), I/O `0xf080`.

**Boards.** `pc_z97_d3h`. The RTL8852BE probe board is not this milestone.

**Protection domains and templates.** `sddf-e1000e-driver` in the network-driver role. The desk image composes it the way `x86_64_generic_net_e1000e` composes the QEMU device. The client address on QEMU stays `52:54:00:12:34:56`. The metal address is whatever DHCP grants.

**Steps.**

- Accept `8086:153b` at `00:19.0` with the recorded subsystem. Keep the QEMU `8086:10d3` path.
- Program the same rings the QEMU driver programs. Do not copy a Linux driver.
- DHCP, then one ping of the gateway. The success lines are an address line and `ping ok`. A miss prints `ip err` or `ping err`.
- Leave the RTL8852BE as a configuration-space read. Do not map its BAR in this milestone.

**QEMU.** `just test-x86-e1000e` still prints `e1000e: ready`, `ip 10.0.2.15`, and `ping ok`. That smoke does not close the milestone.

**Metal evidence.** The desk console or COM1 shows the DHCP address and `ping ok` on the onboard port.

**Exit.** The driver accepts `8086:153b`, the QEMU smoke still passes, and the desk prints a DHCP address and `ping ok`.

## Milestone 7 — Z97 desk session

**Status:** not started.

**Arch capability.** One installed system: local console, disk, and network, with the shell from Milestones 2 and 3. Arch does not ask the operator to swap images to get a shell.

**Boards.** `pc_z97_d3h`. Depends on Milestones 1–3, 5, and 6.

**Protection domains and templates.** `console-pc.system.template` grows the AHCI driver, the e1000e driver, `sddf-fatfs`, and `sddf-shell` (or the console shell learns the same commands, one of the two, not both). The VGA menu stays. COM1 stays. Priorities stay in the LionsOS bands: drivers above the filesystem server, shell at the bottom. The menu program and the filesystem shell are not two copies of `mkdir`.

**Steps.**

- One image boots the VGA menu, the AHCI driver, the I217 driver, and the shell.
- The menu still offers `1 browser` and `2 maths`. `maths` stays the `lerux-calc` session. `browser` keeps printing `browser unavailable` until Milestone 11 is composed onto this image. The menu stays honest until that day.
- The shell from Milestones 2 and 3 is reachable without a second flash. `df`, `ip`, `config`, `edit`, and `dmesg` use the desk disk and the I217.
- Do not attach virtio. virtio-pci workstation images still do not boot this motherboard.
- Do not format the Windows volume.

**QEMU.** `just test-x86-console` still expects `1 browser` and `maths 3`. `just test-x86-ahci`, `just test-x86-e1000e`, and `just test-x86-fetch-native` stay green. QEMU cannot close the combined desk image.

**Metal evidence.** One boot on the desk shows the menu, `ahci: port 0`, the I217 address, `ping ok`, and a shell `edit` that reads back on the spare partition.

**Exit.** That single metal boot. `browser unavailable` remains until Milestone 11.

## Milestone 8 — Swift Wi-Fi association

**Status:** not started.

**Arch capability.** A laptop joins the house network. The Swift has no Ethernet port in this image. The radio is the network device.

**Boards.** `swift_sfx14_41g`. The sequence already written in [`plan-swift-wifi.md`](plan-swift-wifi.md) is the path. This milestone is the exit of that path plus LionsOS milestone 27. Survey counters in review do not close it.

**Protection domains and templates.** `swift-mt7921` owns `03:00.0` (`14c3:7961` or `14c3:0608`). `console-shell` owns `3 wifi`. The image maps physical `0xfc82000000` at virtual `0x82000000`. Firmware is `support/mt7921.bin`, gitignored. The service set and passphrase are `support/swift-wifi.txt`, gitignored.

**Steps.**

- Follow [`plan-swift-wifi.md`](plan-swift-wifi.md) phases 2–8. Do not skip a phase because a host test passed.
- The menu command `3` or `wifi`, once the radio line is `wait`, asks the radio which names it can hear. The shell prints each as `N name` in byte order, at most 16, then `ssid>`. A printed row number selects that row. Any other listed name, or any other line of 1 to 32 bytes, is the name. `ssid ok` follows, then `psk>`. `psk ok` is a passphrase of 8 to 63 bytes. `wifi err` is the rejection, including a survey the radio did not answer.
- Empty `ssid>` means the scan command was posted and zero names were stored. It does not mean a beacon was rejected. The next boot that is diagnosing this prints the survey counters (`rx`, `ok`, `done`, and `fc` when frames arrived and none were kept). Those counters are a means, not the exit.
- After a name is chosen, the stage line is `scan`, `auth`, `assoc`, `eapol`, `dhcp`, then `ping`. A not-ready notify repaints the current stage and keeps no key for the next boot.
- Association is PBKDF2-HMAC-SHA1 and an HMAC-MD5 MIC. Frames the radio accepts are enqueued on an sDDF transmit queue.
- The host test drives a scripted radio to `ip 192.168.1.20` and `ping ok`. That test stays. It does not close the milestone.

**QEMU.** None for the radio. The host scripted-radio test is the regression. Phase 6 of the Wi-Fi plan, the application path on QEMU, uses the existing fetch client once a net driver is standing in. It is not a Swift boot.

**Metal evidence.** The panel prints a DHCP address and `ping ok` on the WPA-Personal network the operator typed. `mt7921: firmware err`, `fw`, `own`, `mac`, or `scan` with no address is not the exit.

**Exit.** LionsOS milestone 27: the panel prints a DHCP address, then `ping ok`.

## Milestone 9 — Swift disk as a filesystem

**Status:** not started.

**Arch capability.** The internal SSD holds the files the operator reads. Arch would mount it read-write. This milestone mounts it read-only. A write to a dedicated partition is a later milestone, spelled below, and is not this exit.

**Boards.** `swift_sfx14_41g`. Depends on Milestone 1 for the filesystem that mounts the volume. Depends on LionsOS milestone 28 for the first named-file read.

**Protection domains and templates.** `swift-nvme` at `02:00.0`, class `0x010802`, and `sddf-fatfs` once the read path returns sectors. Queue pages stay `0x01f00000`. IO write `0x01` and admin format `0x80` stay rejected through this milestone. The volume is not formatted.

**Steps.**

- LionsOS milestone 28 first: the shell reads one named file (`LERUX.TXT`) from a FAT root or an NTFS resident record and prints its bytes. Sector 0 of a GUID partition table is a protective master boot record, type `0xEE`. The reader tries the partition named by the header at LBA 1. `nvme: raw` means sector 0 came back and was not an NTFS or FAT boot sector. That line is not the exit.
- After that read works, mount the same volume through Milestone 1’s filesystem in read-only mode. `cat` and `ls` use the shell. `mkdir`, `write`, and `rm` print `err` on this image until the follow-on below.
- Do not format the SSD. Do not enable bus-master writes that the driver currently rejects.

**Follow-on, not this exit.** A later milestone adds a write, only to a partition the operator dedicated to lerux. The Windows or other operating-system partitions stay read-only. That follow-on is not required for Milestone 10.

**QEMU.** None for this NVMe. Milestone 1’s `just test-fs-sddf` and `just test-x86-ahci` stay the filesystem regression.

**Metal evidence.** The panel prints the bytes of `LERUX.TXT`, then `ls` of that volume through the filesystem server, with writes still rejected.

**Exit.** One named file is printed, and a read-only `ls` goes through Milestone 1’s server. The SSD is not formatted.

## Milestone 10 — Swift console session

**Status:** not started.

**Arch capability.** One laptop image: panel, keyboard, Wi-Fi, read-only disk, and the daily shell. The operator does not chain a debug probe image and a shell image.

**Boards.** `swift_sfx14_41g`. Depends on Milestones 1–3, 8, and 9.

**Protection domains and templates.** `console-swift.system.template`. Roles: `console_driver`, `console-shell`, `swift-pci-probe`, `swift-mt7921`, `swift-nvme`, `sddf-fatfs`, and the network virtualisers and copier once Milestone 8 speaks sDDF queues. There is no `serial-driver`. The keyboard interrupt stays channel 0 on `console_driver` (I/O APIC pin 1, vector 49).

**Steps.**

- One ISO: `just iso`. The panel still shows the mode line’s geometry (1920 by 1080, pitch 7680, framebuffer `0xfc90000000`) and the menu `1 browser`, `2 maths`, `3 wifi`.
- `2` or `maths` still runs `lerux-calc`. The proof on metal is `maths 3` from the built-in keyboard. LionsOS milestone 25 recorded the menu without that line.
- `3 wifi` is Milestone 8. After `ping ok`, `ip` in the shell prints the same address.
- The read-only disk is Milestone 9. `ls` and `cat` work. Writes print `err`.
- `config`, `date`, and `dmesg` are Milestone 3. `date err` is acceptable until the Swift image has a timer the shell can call. Do not pretend the TSC was read if it was not.
- `browser` prints `browser unavailable` until Milestone 11 is on this image.
- Probe rows and radio rows stay where [`docs/swift-boot-screen.md`](swift-boot-screen.md) puts them. A full console clear is `Damage::All` and erases rows the console cell buffer does not own. Do not move those rows in this milestone.
- No COM port. Do not add `serial-driver` to this image.

**QEMU.** None. `just check` and `just check-pd` still run. They do not close the milestone.

**Metal evidence.** One panel boot shows the menu, `maths 3`, a Wi-Fi address, `ping ok`, and `ls` of the read-only volume.

**Exit.** That boot, from one ISO, with writes still rejected and `browser unavailable` still honest.

## Milestone 11 — Browser on QEMU pixels, then on the desk and the panel

**Status:** not started for the metal browser. Ladybird milestones 1–10 are done (tree, URL, bitmap, DOM, CSS, cold style, layout, paint, web content, request server). Ladybird milestone 11 (navigables) is **in review** and is not a metal exit.

**Arch capability.** A browser opens a page: a heading, an image, a link, and a form. Firefox is not the implementation. The engine stays the Rust rewrite shaped like Ladybird, in [`../lerux-ladybird/PLAN.md`](../lerux-ladybird/PLAN.md). Both “milestone 17” headings stay: LionsOS “Remove the old model” and Ladybird “Forms”. This plan’s Milestone 11 is the whole remaining browser track. It does not renumber Ladybird.

**Boards.** QEMU first (`qemu_virt_aarch64_browser_sddf` and the Ladybird QEMU board). Then the Z97 HDMI path, using the VGA text console until a pixel path exists, then the Swift panel. The GTX 1650 (`01:00.0`, `10de:1f9d`) and the RX 560 stay unused. The Swift panel is the Radeon GOP framebuffer the firmware already handed over (`04:00.0`), not a new display engine. The Pi browser is not this milestone.

**Protection domains and templates.** The Ladybird crates and the browser shell already on `qemu_virt_aarch64_browser_sddf` (`gpu_driver`, `gpu_virt`, `shell`). The desk image from Milestone 7 and the Swift image from Milestone 10 gain that client only in the desk-seam step. Do not enable `board-qemu_virt_aarch64_browser_sddf` together with the fetch, web, or filesystem board features.

**Steps.** Continue Ladybird milestones in order. Do not start a later one before the earlier exit is on `main`.

1. **Navigables** (Ladybird 11, in review). A host test loads a document, follows one link to a second document, and follows a fragment without a second load. `just test-ladybird` prints `title lerux`, `pixel 000000ff`, then `title frame` and `pixel ff0000ff` from the iframe. A failed iframe does not drop the parent that already loaded.
2. **ImageDecoder** (Ladybird 12). The Lenna page is the local check: heading `Lenna` and the 512 by 512 image. PNG decode stays in-tree. The sampled pixels already recorded for that image stay the regression.
3. **Script engine** (Ladybird 13) and **classic scripts** (Ladybird 14). JavaScript is approved. The engine is the one in the Ladybird plan, not a second runtime.
4. **DOM bindings** (Ladybird 15), **chrome and keys** (Ladybird 16), **forms** (Ladybird 17). A form posts the way the desk-browser note in [`boards.md`](boards.md) already describes for the QEMU preview (`q=hi`).
5. **WebAssembly** (Ladybird 18). Ordinary WebAssembly stays `wasmi` in the one runtime protection domain ([ADR-012](decisions/012-ordinary-webassembly.md)). `wasmtime` and the WebAssembly Micro Runtime stay out. Do not confuse this with LionsOS milestone 14, which is the program image and is already done.
6. **One worker** (Ladybird 19).
7. **Desk seam** (Ladybird 20). This is the first metal browser. Compose the client onto the Milestone 7 Z97 image. The menu’s `browser` choice stops printing `browser unavailable` and paints the parity page on the HDMI output the firmware is using. Then compose the same client onto the Milestone 10 Swift image and paint on the GOP framebuffer. Lenna remains the local paint check. The parity page is a fetched page with a heading, one image, a link, and a form.

Each Ladybird step has a host test and either `just test-ladybird` or `just test-browser-sddf`. The existing browser smoke still expects `1 browser`, then `pixel 000000ff`, then `maths 9`. Keep those strings out of `expect` and `assert` messages. The frame stays 320 by 64 on the QEMU graphics client until a metal seam names a larger framebuffer. The Swift panel is 1920 by 1080. The seam scales or letters the page. It does not change the QEMU smoke frame as a side effect.

**QEMU.** `just test-browser-sddf` and `just test-ladybird` for every step above. `just check` and `just check-pd` before a step is called done.

**Metal evidence.** Only the desk-seam step. The Z97 shows the parity page on the HDMI output. The Swift shows it on the panel. A QEMU `pixel` line does not close either metal boot. The guest proof must say whether the paint path used the display interrupt or only a polled flush.

**Exit.** The parity page is fetched and painted on QEMU, then on the Z97, then on the Swift. `browser unavailable` is gone on those two menus. The Pi menu is unchanged.

## Milestone 12 — Raspberry Pi 4 lab

**Status:** not started.

**Arch capability.** The same console session on the Pi: serial shell, eMMC filesystem, and the GENET port with DHCP, DNS, and TCP. The Pi is secondary. It does not block Milestones 5–11.

**Boards.** `rpi4b_4gb_workstation`. The hello board `rpi4b_4gb` is not the workstation.

**Protection domains and templates.** Either port `genet-driver` and `emmc2-driver` onto the sDDF block and net queues, or leave the existing workstation profile (supervisor, the pre-LionsOS servers, native drivers) as the Pi image and say so at the top of the Pi section in [`boards.md`](boards.md). Do not leave the reader unsure which image is the install. One of those two choices is the first step. A half port is not an image.

**Steps.**

- Pick the image, in the first change, and write that choice into [`boards.md`](boards.md) and [`platforms.md`](platforms.md).
- If the port is chosen, the Pi shell speaks the same commands as Milestones 2 and 3, and the filesystem is Milestone 1. The GENET driver is the network-driver role. Unified-dma for GENET is part of this port. The x86 PCI unified-dma work is not a substitute.
- If the old profile is kept, do not pretend it is an sDDF image. The metal checks below still apply.
- Fill the result grid in [`boards.md`](boards.md#rpi4-workstation-install-path-phase-52). The open list in [`plan-arch.md`](plan-arch.md#physical-rpi4-lab-hardware-gated) is the list: scripted `ls`, `pwd`, and `ip`; a serial REPL `ls`, `cat /boot.log`, `fetch`, and `edit`; GENET TCP, DNS, and DHCP. `fetch` is UDP-demo-only on hardware today.
- `just test-hw` with `LERUX_HW_SERIAL` and `BOARD=rpi4b_4gb_workstation`.
- No second aarch64 board until this checklist is filled and the command is green.
- The Pi browser waits until this console session matches Milestone 10’s bar: shell, disk, and a real address. There is no Pi HDMI milestone in this plan.

**QEMU.** There is no QEMU profile for `rpi4b_4gb_workstation`. Host `just check` and `just check-pd` cover a driver change. They do not close the lab.

**Metal evidence.** The serial log from the Pi shows the scripted lines and a TCP `fetch`. Record pass or fail in the boards.md grid, including a failure.

**Exit.** The grid is filled and `just test-hw` is green for `ls`, `pwd`, `ip`, and a TCP fetch. Optional self-hosted CI stays opt-in, as [`ci.md`](ci.md) already says.

## Milestone 13 — Stretch, after the bar

**Status:** not started. Not part of the parity bar. Start a stretch item only after Milestones 1–11 are done. Milestone 12 may still be open. The Pi does not gate these.

**Swift Bluetooth.** LionsOS milestone 29. `swift-bt` selects the HCI interface on `04:00.3` or `04:00.4` and prints `bt: reset ok` after the Bluetooth firmware load. The firmware file is `support/mt7921-bt.bin`, not the Wi-Fi blob. The personal-area-network profile is later and does not join the WPA-Personal access point. After `bt: reset ok`, a HID keyboard or mouse is the useful stretch. PAN is named here and is not required for parity, because Milestone 8 is the network.

**USB Ethernet.** Only if Milestone 8 stays blocked after the Wi-Fi plan’s own phases, including its phase 9. It is not a second network stack beside a working radio.

**Sound.** Not started. No protection domain is named here, because none exists. A later note can pick the Swift HD Audio function or the Z97 codec. Do not start it from this sentence.

**Suspend.** Out of the bar. No S3 milestone until a board already does Milestones 7 or 10 and a note names the firmware tables to read.

**Measured boot.** Host digests are Milestone 4. A metal root of trust is stretch and needs the board. Do not claim it from `loader.img.sha256`.

**RISC-V high virtio map.** LionsOS milestone 23. It waits on Microkit. The identity map stays. It is not a real-board milestone and does not move in this file.

## Milestone 14 — Rescue and diagnostic commands

**Status:** not started.

**Arch capability.** The Arch installer USB is a temporary system. It contains the `base` metapackage and a separate rescue list, `configs/releng/packages.x86_64`. `pacstrap base` does not copy that rescue list onto the new disk. This milestone puts a lerux command for every one of those tools onto the Swift, Z97, and Pi images. The command is a protection domain or a shell built-in. It is not the Arch binary, and it does not link glibc.

The names below are the ones named for that installer image. A tool already covered by an earlier milestone stays that command. It is listed so the set is complete.

**Boards.** All three, on the console each board already has. A command that needs a device the board does not have prints the command name plus `err`.

**Protection domains and templates.** `sddf-shell` and `console-shell`, plus one small protection domain only when the command needs its own device queue. No new postcard message. Channel numbers come from the profile. Do not vendor `smartmontools`, `nvme-cli`, `testdisk`, or any other Arch package.

**Everyday diagnostics, from `base`.** These are on the live image because the live image contains `base`.

- `ps`, `top`. The filesystem shell already prints `ps` and `qos`. `top` is the same table with a repeating read. `free` and `vmstat` print the protection-domain memory and fault counts the supervisor already knows. There is no `/proc`.
- `pstree` prints the static protection-domain graph. There is no process tree to walk.
- `fuser` prints which protection domain holds a file the filesystem server has open.
- `lsblk` prints the block drivers that booted. `lscpu` prints the architecture and the core count the kernel reported. `dmesg` is Milestone 2. `findmnt` prints the one mounted volume. `fdisk` prints the partition table and does not write it.
- `lspci` prints the configuration-space lines the board probe already has. On the Swift that is the six lines from LionsOS milestone 26. On the Z97 it is the devices Milestones 5 and 6 accepted.
- `ip` and `ping` are Milestones 2 and 6 or 8. `ss` prints the open sockets the network client holds. A board with no network role prints `ss err`.
- `journalctl` is `dmesg` with the same filters Milestone 2 already named (`--pd`, `-l`). `systemctl` is `ps` plus the ready or failed line for one protection domain. `systemd-analyze` prints the boot timestamps the shell already recorded. There is no systemd.

**Disk health.**

- `smartctl` reads the SMART page the block driver can return. The AHCI and NVMe drivers grow that read. A controller that has no SMART page prints `smartctl err`.
- `nvme` reads identify and the log pages `swift-nvme` already uses. Admin format stays rejected. IO write stays rejected until the Milestone 9 follow-on.
- `hdparm` and `sdparm` print the identify data for the Serial ATA disk. They do not change the write cache.
- The SCSI inquiry in `sg3_utils` is `lsscsi` below, for a disk that speaks SCSI. The Z97 and the Swift are not SCSI disks. The command prints `err` there.

**Hardware listing.**

- `dmidecode` prints the firmware tables the boot stub already saved, or `dmidecode err` when the board has none.
- `lsusb` prints the xHCI ports the image already enumerates. On the Swift that is the Bluetooth controller’s port line. On the Z97 it is the USB HID keyboard.
- `lsscsi` prints `err` on these three boards. None of them boots from a SCSI HBA.
- `ethtool` prints link, speed, and the hardware address for the I217 or the MT7921. It does not change offload flags in this milestone.

**Partition and copy.** These commands read unless the operator names a dedicated lerux partition. They do not format a volume that still holds another operating system. That rule is the same one Milestones 5 and 9 already use.

- `parted` and `gdisk` print the GUID partition table. `gpart` guesses a table when the header is damaged and prints the guess. It does not write the guess back.
- `testdisk` walks a damaged table and prints the partitions it can still name. It does not write them.
- `ddrescue` copies one named partition to another named partition on a second disk, and keeps a map of the sectors it could not read. The source is read-only.
- `clonezilla`, `fsarchiver`, `partclone`, and `partimage` are one command, `partclone`. It copies a partition image to a file on the dedicated lerux partition, or back from that file. It does not become a second distribution.

**Filesystems and volumes.** Read-only checkers first. A repair that writes is refused unless the operator names the dedicated lerux partition.

- FAT is the filesystem Milestone 1 already serves. `dosfstools` is that server’s check command, `fsck`.
- `e2fsprogs`, `btrfs-progs`, `xfsprogs`, and `ntfsprogs` are read-only identify and list commands for a partition whose boot sector says that type. The Swift named-file read already accepts a FAT root or an NTFS resident record. This milestone adds the type name and a directory list. It does not mount ext4, Btrfs, or XFS as the system volume.
- `lvm2` and `mdadm` print `err` unless the partition table names an LVM physical volume or a Linux RAID member. These boards’ install images do not create either.
- `cryptsetup` prints `err`. Disk encryption is not part of the bar. The command exists so the operator can see that the volume is locked, and it does not unlock it.

**Memory.** `memtest86+` on Arch runs from the USB boot menu, before the installed system. The lerux ISO boot menu gains a `memtest` entry that writes a pattern over a reserved RAM window and prints pass or fail on the panel or VGA console, then returns to the menu. It does not start seL4.

**Network traces.**

- `ping` is already required on the I217 and on the MT7921.
- `tcpdump` prints the headers of frames the network client already copied, up to a fixed cap. It does not put the NIC in promiscuous mode in this milestone.
- `nmap` probes one named host and port and prints open or closed. It does not scan a range wider than that one target.
- `dig` is the DNS lookup the fetch path already does, printed as the answer. A miss prints `dig err`.

**Steps.**

- Add each command to the shell command table from Milestone 2. A missing device prints the command name plus `err`. A successful read prints one line the smoke can match.
- Host tests cover the parsers: a SMART page, a GUID header, a FAT boot sector, an NTFS boot sector, and a DNS answer. Golden lines go in `support/smoke-expects.toml`.
- QEMU `just test-fs-sddf` and `just test-x86-fetch-native` run `lsblk`, `fdisk` (print only), `fsck`, `ss` or `ss err`, and `dig` or `dig err`.
- The metal exits are `smartctl` and `lspci` on the Z97, `nvme` and `lspci` on the Swift, and `lsblk` on the Pi. Each prints the device that board already has.

**QEMU.** `just test-fs-sddf` and `just test-x86-fetch-native` for the commands that do not need the real controller. `just check` and `just check-pd` for the shell change.

**Metal evidence.** The Z97 shows `lspci` including `8086:153b` and a `smartctl` line from the Serial ATA disk. The Swift shows `lspci` including `14c3:7961` and `1c5c:174a`, and `nvme` identify. The Pi shows `lsblk` for the eMMC volume. A command that only passes on QEMU does not close the metal line.

**Exit.** Every name in this milestone is a shell command on the three board images. The QEMU script covers the parsers. The three metal lines above have been seen. No Arch binary is linked, and no foreign operating-system volume is written.

## Order

1. Milestone 1, then 2, then 3. They are QEMU and they unblock every board.
2. Milestone 4 can run beside 1–3. It is host-only.
3. Milestones 5 and 6 can run beside each other. Milestone 7 waits for both, and for 1–3.
4. Milestones 8 and 9 can run beside each other. Milestone 10 waits for both, and for 1–3.
5. Milestone 11’s QEMU Ladybird steps can run beside 5–10. The desk seam waits for Milestone 7. The Swift seam waits for Milestone 10.
6. Milestone 12 waits for nothing in 5–11 except operator time. It stays secondary.
7. Milestone 13 waits for 1–11.
8. Milestone 14 waits for Milestone 2. The disk and PCI lines wait for the board session that owns that device (Milestone 7, 10, or 12).

A metal boot is the exit only where the section says so. Milestones 1–4 close without one.

## Verification

| Gate | When |
|------|------|
| `just check` | Any host or guest change, before the milestone is called done |
| `just check-pd` | Any protection-domain or shared userspace change. Needs the SDK. Do not run it in the same invocation as `just check` |
| `just test-fs-sddf` | Milestones 1–3 |
| `just test-x86-ahci` | Milestones 1 and 5 |
| `just test-x86-e1000e` | Milestone 6 |
| `just test-x86-fetch-native` | Milestones 2 and 3 |
| `just test-x86-console` | Milestones 7 and 11, as a regression only |
| `just test-browser-sddf`, `just test-ladybird` | Milestone 11 |
| Host scripted radio | Milestone 8, regression only |
| Panel or COM1 | The metal exit named on that milestone |
| `just test-hw` on the Pi | Milestone 12 |
| `just test-fs-sddf`, `just test-x86-fetch-native` | Milestone 14 parsers |

Update the status word on a milestone in this file when its exit is met. Do not mark LionsOS 22–29 or Ladybird 11–20 done from this file. Those plans keep their own checkboxes.

## Explicit non-goals

- Unmodified Arch packages, `pacman` inside the guest, glibc or musl userspace. Milestone 14 ports the command names. It does not link the Arch binaries.
- A POSIX ABI, `fork`, `exec`, or a Linux root filesystem as the primary interface
- Guest Linux or libvmm
- Wayland, a compositor, or a driver for the GTX 1650 or the RX 560
- Using the RTL8852BE as the Z97 network device
- Formatting a disk that still holds another operating system
- Printing, multi-user accounts, and suspend, as part of the bar
- Sound and Bluetooth PAN, except as Milestone 13 stretch
- A second single-board computer before the Pi lab is signed off
- Vendoring LionsOS or sDDF C trees, or inventing a missing header
- Formal verification of these protection domains

## Summary

Milestones 1–4 rebuild the daily filesystem, shell, config, and host update on the LionsOS queues. Milestones 5–7 make the Z97 one session on its own Serial ATA controller and its I217. Milestones 8–10 make the Swift one session on its radio and a read-only SSD. Milestone 11 puts the Rust browser on QEMU and then on those two panels. Milestone 12 is the Pi lab, still secondary. Milestone 13 is what Arch also has and this bar does not require yet. Milestone 14 is a lerux command for every rescue and diagnostic tool on the Arch live installer.
