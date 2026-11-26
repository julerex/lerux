# Boards

Board names are the `BOARD=` value for `just run`, `just test`, and `just build`. Metadata lives in [`support/boards.toml`](../support/boards.toml). `just build` and `just run` default to `qemu_virt_aarch64`. Bare `just test` boots `qemu_virt_aarch64_browser_sddf`. `just make-lerux-iso-swift` packs one image, `swift_sfx14_41g`, that probes the machine.

The metal target being driven is the Acer Swift X SFX14-41G (`swift_sfx14_41g`). That one ISO also boots the Gigabyte Z97-D3H, the Dell Inspiron N5010 (`inspiron_n5010`), and any other x86 PC. The stub selects the Swift GOP panel when the framebuffer is `0xfc90000000` and the VGA text page otherwise. It does not map the Inspiron AHCI disk. A VGA boot does not load, store, or bus-master the NVMe window, and it does not program the IOMMU or the MT7921. The stub reserves the panel and the MT7921 window only when the CPU can address them. `just make-lerux-iso-z97` packs the one Z97 image, the I217 shell with the Arch sign-on (`pc_z97_d3h_i217`), and `just make-lerux-iso-inspiron` packs the Inspiron image. The RPi4 workstation lab (`rpi4b_4gb`) stays secondary.

aarch64 init and composed images need the patched SP804 QEMU in [`ci.md`](ci.md#patched-qemu); RISC-V init uses stock QEMU with the Goldfish RTC and `rdtime`, and x86 init uses stock QEMU with the CMOS RTC and TSC.

**System profiles** (Phase 35) live in `support/profiles/*.toml` (e.g. `workstation`, `minimal`). They declare the PD list, template, and channel manifest. Use `lerux profile build <name>` (it resolves to a board via `default_board` or `--board`). Board entries in `boards.toml` still define hardware details + qemu config; profile tooling selects compositions on top of boards.

## Reference

| Board | Arch | Smoke command | PDs (summary) |
|-------|------|---------------|---------------|
| `qemu_virt_aarch64` | aarch64 | `BOARD=qemu_virt_aarch64 just test` | hello + serial |
| `qemu_virt_aarch64_debug` | aarch64 | `just test-debug` | debug-handler (parent) + crash-demo (child fault) |
| `qemu_virt_aarch64_serial_sddf` | aarch64 | `just test-serial-sddf` | `serial_driver`, `serial_virt_tx`, `serial_virt_rx`, `serial_client` |
| `qemu_virt_aarch64_cothread` | aarch64 | `just test-cothread` | `cothread_client`, `cothread_peer`, `serial_driver` |
| `qemu_virt_aarch64_fs_sddf` | aarch64 | `just test-fs-sddf` | `blk_driver`, `serial_driver`, `blk_virt`, `serial_virt_tx`, `serial_virt_rx`, `fatfs`, `timer_driver`, `shell` |
| `qemu_virt_aarch64_net_sddf` | aarch64 | `just test-net-sddf` | `net_driver`, `serial_driver`, `net_virt_tx`, `net_virt_rx`, `serial_virt_tx`, `serial_virt_rx`, `net_copy`, `net_client` |
| `qemu_virt_aarch64_fetch_sddf` | aarch64 | `just test-fetch-sddf` | `blk_driver`, `net_driver`, `serial_driver`, `blk_virt`, `net_virt_tx`, `net_virt_rx`, `serial_virt_tx`, `serial_virt_rx`, `fatfs`, `net_copy`, `shell` |
| `qemu_virt_aarch64_web_sddf` | aarch64 | `just test-web-sddf` | `blk_driver`, `net_driver`, `serial_driver`, `blk_virt`, `net_virt_tx`, `net_virt_rx`, `serial_virt_tx`, `serial_virt_rx`, `fatfs`, `net_copy`, `shell` |
| `qemu_virt_aarch64_browser_sddf` | aarch64 | `just test` | `blk_driver`, `net_driver`, `gpu_driver`, `serial_driver`, `blk_virt`, `net_virt_tx`, `net_virt_rx`, `serial_virt_tx`, `serial_virt_rx`, `gpu_virt`, `fatfs`, `net_copy`, `shell` |
| `qemu_virt_aarch64_agent_sddf` | aarch64 | `just test-agent-sddf` | `blk_driver`, `net_driver`, `serial_driver`, `blk_virt`, `net_virt_tx`, `net_virt_rx`, `serial_virt_tx`, `serial_virt_rx`, `fatfs`, `net_copy`, shell running `sddf-grok.elf`, agent running `sddf-agent.elf` |
| `qemu_virt_aarch64_program_sddf` | aarch64 | `just test-program-sddf` | fetch roles with `program` (`sddf-program.elf`) in place of the shell |
| `qemu_virt_aarch64_html` | aarch64 | `just test-html` | Phase 74: html-demo + lerux-html (baked-in fixture) |
| `qemu_virt_aarch64_virtio` | aarch64 | `just test-virtio` | hello + serial + virtio blk/net |
| `qemu_virt_aarch64_http` | aarch64 | `just test-http` | serial + virtio-net + http-server |
| `qemu_virt_riscv64` | riscv64 | `just test-riscv` | hello + serial (MMIO UART) |
| `qemu_virt_riscv64_virtio` | riscv64 | `just test-riscv-virtio` | hello + serial + virtio |
| `qemu_virt_riscv64_http` | riscv64 | `just test-riscv-http` | serial + virtio-net + http-server |
| `x86_64_generic` | x86_64 | `BOARD=x86_64_generic just test` | hello + serial (COM1) |
| `x86_64_generic_console` | x86_64 | `just test-x86-console` | `serial_driver`, `console_driver`, `shell` (`console-shell.elf`) |
| `pc_z97_d3h_i217` | x86_64 | `just make-lerux-iso-z97` | Arch sign-on and `ping` over the I217-V, 80 by 25 cells in the Intel framebuffer (hardware only) |
| `swift_sfx14_41g` | x86_64 | `just make-lerux-iso-swift` | One ISO: Swift panel when the GOP buffer is `0xfc90000000`, VGA text on any other x86 PC (hardware only) |
| `inspiron_n5010` | x86_64 | `just make-lerux-iso-inspiron` | VGA-only image. The default ISO also boots this machine on the VGA text page (hardware only) |
| `x86_64_generic_virtio` | x86_64 | `just test-x86-virtio` | hello + serial + virtio-pci blk/net |
| `x86_64_generic_http` | x86_64 | `just test-x86-http` | serial + virtio-pci net + http-server |
| `rpi4b_4gb` | aarch64 | `BOARD=rpi4b_4gb just image` | hello + serial (PL011; hardware only) |

Milestone 17 stays in [plan-lionsos.md](plan-lionsos.md). It removed the postcard guest model and did not remove the Ladybird-shaped domains or the postcard servers the browser specs still name. The live browser smoke is `just test-browser-sddf`.

## SDK boards

`just build-sdk` compiles kernel + loader for Microkit board names (not always identical to lerux `BOARD`):

```bash
MICROKIT_BOARDS=qemu_virt_aarch64,x86_64_generic,qemu_virt_riscv64,rpi4b_4gb just build-sdk
```

RPi4 workstation images require `rpi4b_4gb` in `MICROKIT_BOARDS` (e.g. `MICROKIT_BOARDS=qemu_virt_aarch64,rpi4b_4gb just build-sdk`).

CI sets this via `MICROKIT_BOARDS` in the workflow env.

## Hardware boards (Phase 37+)

Real (non-QEMU) boards have no `qemu` field. ARM/RISC-V produce `loader.img`; x86 stages `sel4.elf` (64-bit) for the GRUB Multiboot 2 menu. QEMU x86 smokes pack that kernel into a GRUB UEFI ISO and boot under OVMF.
Use `lerux image --board <name>` (or `BOARD=<name> just image`).

- `rpi4b_4gb`: Raspberry Pi 4 Model B (4 GB). Requires U-Boot on SD card. See seL4 docs for initial bring-up and `fatload` / `go`.
- Serial: PL011 at 0xfe201000 (GPIO 14/15). Update IRQ in `boards.toml` if the platform IRQ mapping differs.
- Full workstation (FS + net) on hardware requires native (non-virtio) block and network drivers; see `support/profiles/hardware-rpi4.toml`.
- `swift_sfx14_41g`: Acer Swift X SFX14-41G. `just make-lerux-iso-swift` packs the UEFI image. See [Acer Swift X install path](#acer-swift-x-install-path).
- `inspiron_n5010`: Dell Inspiron N5010. `just make-lerux-iso-inspiron` packs the UEFI image. See [Dell Inspiron N5010 install path](#dell-inspiron-n5010-install-path).
- `pc_z97_d3h_i217`: Gigabyte GA-Z97-D3H (Haswell), the one Z97 image. Microkit still uses `x86_64_generic`; deploy writes GRUB's `EFI/BOOT/BOOTX64.EFI`, `/boot/grub/grub.cfg`, and **`boot/mb2-stub.elf` + `boot/sel4.elf` + `boot/loader.img`** for UEFI Multiboot 2. The image paints the 80 by 25 sign-on into the recorded firmware buffer. The desk-browser spec makes the scanout card the Intel HD Graphics 4600 on the motherboard HDMI port, with Initial Display Output set to `IGFX`. After the reboot on 2026-10-22, firmware reports that device as the boot VGA device. See [Gigabyte Z97-D3H install path](#gigabyte-z97-d3h-install-path) and [VGA text on the RX 560](#vga-text-on-the-rx-560).

`just run` on hardware boards builds the image then prints deployment instructions (no QEMU).

`just test` on hardware boards builds the image; without a serial device it stops there (manual gate). **Phase 47 golden path** for automated boot smoke:

```bash
BOARD=rpi4b_4gb_workstation LERUX_HW_SERIAL=/dev/ttyUSB0 just test-hw
# equivalent: lerux test --board rpi4b_4gb_workstation --mode hw-serial
```

Expects come from [`support/smoke-expects.toml`](../support/smoke-expects.toml) (shared with QEMU smokes). Modes and lock env: [`ci.md` — hardware serial](ci.md#hardware-serial-smoke-phase-47).

### RPi4 workstation install path (Phase 52)

Board `rpi4b_4gb_workstation` (profile `workstation-rpi4`) is the **install-media → boot → shell** path on real metal: supervisor, fs-server, net-server, shell, log, config, edit/chat/http-fs over native `genet-driver` + `emmc2-driver`. There is no QEMU profile for this board.

**One-command deploy** (after mounting the SD FAT boot partition on the host):

Build the image, verify the digest, write the media, and boot. `just deploy-rpi4` copies `build/rpi4b_4gb/loader.img`. `lerux verify-image` prints that flash line after the digest matches.

```bash
BOARD=rpi4b_4gb just image
lerux verify-image --board rpi4b_4gb
DEST=/media/$USER/boot just deploy-rpi4
# equivalent: cargo run -p lerux-cli -- deploy --board rpi4b_4gb --dest /media/$USER/boot
# Boot: fatload mmc 0 0x10000000 loader.img && go 0x10000000
```

The guest cannot install a package. The operator rebuilds on the host with `lerux package install --board rpi4b_4gb` or `lerux package upgrade --board rpi4b_4gb`. That command prints the pin digest and writes `loader.img.sha256`.

**Full path**

| Step | Command / action |
|------|------------------|
| 1. SDK | `MICROKIT_BOARDS=…,rpi4b_4gb just build-sdk` or `just fetch-sdk` |
| 2. Image | `BOARD=rpi4b_4gb just image`, then `lerux verify-image --board rpi4b_4gb` |
| 3. Flash | `DEST=/path/to/sd-boot just deploy-rpi4` |
| 4. Boot | U-Boot: `fatload mmc 0 0x10000000 loader.img` then `go 0x10000000` |
| 5. Host smoke | `LERUX_HW_SERIAL=/dev/ttyUSB0 just test-hw` (boot expects + scripted `ls`/`pwd`/`ip`) |
| 6. Manual REPL | At `lerux>`: `ls`, `cat /boot.log`, `ip`, `fetch`, `edit /test.txt` |

**Prerequisites**

- Raspberry Pi 4 Model B (4 GB) with U-Boot on the SD FAT boot partition
- USB-serial adapter on GPIO UART (PL011, 115200 8N1)
- Ethernet on `192.168.1.0/24` (static fallback guest `192.168.1.10`; DHCP used when a server answers; `fetch` UDP demo targets `192.168.1.1:12345`)

**First-boot disk story**

Empty eMMC/block device is formatted as **LERUXFS2** on first FS access. Supervisor then:

1. `mkdir /config` (idempotent)
2. Seeds config keys via `config-server`: `net.ip`, `net.gateway`, `net.dns`, `hostname`
3. Logs `lerux-supervisor: first-boot seed ok`
4. Writes `/boot.log` from the log ring

Reboots refresh the seed keys (overwrite) and re-write `/boot.log`.

**Automated boot + REPL smoke (Phase 47/52)**

Connect serial **before** starting the host command; boot (or reset) the Pi so logs stream while the reader runs:

```bash
BOARD=rpi4b_4gb_workstation LERUX_HW_SERIAL=/dev/ttyUSB0 just test-hw
```

1. Unordered boot expects from `support/smoke-expects.toml` (supervisor/fs/net/shell/seed/…).
2. **Scripted REPL** (same TTY): host sends `ls`, `pwd`, `ip` and waits for `boot.log`, `/`, `inet`.

Success prints `==> hardware serial smoke passed` and holds a board lock under `$TMPDIR/lerux-hw-locks/`.

Without `LERUX_HW_SERIAL`, `just test` only verifies the image build.

Living work list (what still needs a Pi): [`plan-arch.md` — Physical RPi4 lab](plan-arch.md#physical-rpi4-lab-hardware-gated). This page is the procedure and result grid.

**Manual REPL checklist** (record pass/fail for the lab gate)

| Command | Pass criteria | Result |
|---------|---------------|--------|
| `ls` | Lists `boot.log` / dirs; no `ls: error` | |
| `cat /boot.log` | File contents printed | |
| `ip` | Shows `inet …` (dhcp or static) | |
| `fetch` | `fetch: demo udp sent` (UDP demo, not HTTP) | |
| `edit /test.txt` | Edit TUI; Ctrl-S save, Ctrl-Q quit | |
| `pwd` / `cd` / `mkdir` | Hierarchical FS ops work | |

**Likely failure modes**

| Symptom | Likely cause |
|---------|----------------|
| No `lerux-fs: ready` / seed never appears | `emmc2` SDHCI init / block probe |
| `ls: error` / no `boot.log` | FS format failed or wrong channel wiring |
| `ip: unavailable` / no `lerux-net: ready` | genet bring-up; check PHY/link |
| `fetch` prints ok but nothing on LAN | UDP demo is best-effort; check static IP/gateway |
| `test-hw` script step timeout | Shell not at prompt; serial held by another process; increase `script_timeout_secs` |

**QEMU dev substitute:** `just disk-img && just test-workstation` exercises the same REPL stack with virtio (not a substitute for the RPi4 gate).

**Self-hosted CI:** optional workflow [`.github/workflows/hw-serial.yml`](../.github/workflows/hw-serial.yml); see [`ci.md`](ci.md#hardware-serial-smoke-phase-47).

### Acer Swift X install path

A first boot, from the checkout through Wi-Fi, is [setup.md](setup.md).

Board `swift_sfx14_41g` is the one x86 ISO. On the Acer Swift X SFX14-41G (Ryzen 5 5500U) it paints the GOP panel. On any other x86 PC, including the Z97 desk and the Inspiron, it paints the VGA text page. On a UEFI boot of the Z97 that page is not the picture on the motherboard HDMI port. It reuses Microkit `x86_64_generic` and `console-swift.system.template`. Names for this machine and its devices are in [GLOSSARY.md](../GLOSSARY.md#swift-x). The keyboard is the built-in i8042 (ports `0x60` and `0x64`, I/O APIC pin 1, vector 49). The Swift has no COM port and no `serial-driver`. This image has no QEMU model of the panel. Device roles are in [platforms.md](platforms.md#swift-x-devices). `just make-lerux-iso-z97` packs the separate [Gigabyte Z97-D3H](#gigabyte-z97-d3h-install-path) image.

**USB ISO** (`just make-lerux-iso-swift` packs `build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso`). Build the image, verify the digest, write the media, and boot. `lerux verify-image` prints `lerux iso --board swift_sfx14_41g --no-build` and the `dd` line after the digest matches. That packs the checked `loader.img`. It does not run `just make-lerux-iso-swift`.

```bash
just fetch
just fetch-sdk
just make-lerux-iso-swift
lerux verify-image --board swift_sfx14_41g
lsblk
sudo dd if=build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso of=/dev/sdX bs=4M status=progress conv=fsync
```

The guest cannot install a package. The operator rebuilds on the host with `lerux package install --board swift_sfx14_41g` or `lerux package upgrade --board swift_sfx14_41g`. That command prints the pin digest and writes `loader.img.sha256`.

Replace `sdX` with the flash drive from `lsblk`. Do not write the internal NVMe.

`just burn-lerux-iso-swift` builds the one x86 image, then lists the USB sticks and asks which one to erase. `just burn-lerux-iso-z97` and `just burn-lerux-iso-inspiron` do the same for the Z97 I217 shell and the Inspiron image. None of these recipes takes an argument. The command refuses the host OS disk. On the Linux host that wrote the stick, `just boot-lerux-usb` sets UEFI `BootNext` for one reboot and leaves the lasting boot order alone, so Omarchy (or the previous default) returns on the next power-on. Otherwise reboot and open the firmware boot menu. Choose the USB entry that says UEFI (GRUB `EFI/BOOT/BOOTX64.EFI`). GRUB boots immediately. It Multiboot 2-loads `/boot/mb2-stub.elf` with modules `/boot/sel4.elf` and `/boot/loader.img`. The stub places the 64-bit kernel at 1 MiB after `ExitBootServices`.

On 2026-10-29 the panel showed `fb 0x000000fc90000000 pitch 7680 1920x1080 r16 g8 b0`. The shell band is 240 columns by 40 rows of the VGA 8×16 font, light gray on black, with an underline at the cursor. The sign-on is `Lerux` and the short git describe, then `lerux login: root (automatic login)`, the archiso message of the day, and `[root@lerux ~]# `. The prompt runs `sddf-shell`, including `setup-wifi` and the file commands. Those file commands use `sddf-fatfs` on a RAM volume. The NVMe disk is not formatted. Wi-Fi, the disk, Bluetooth, and the PCI names paint below that band in the same font. The built-in keyboard types the line.

The ISO is UEFI only. Packing needs `xorriso` and `grub-mkimage` (Arch: `sudo pacman -S libisoburn grub`. Debian: `xorriso` and `grub-efi-amd64-bin`).

### Gigabyte Z97-D3H install path

Part names for this desk and for QEMU q35 are in [GLOSSARY.md](../GLOSSARY.md). The driver tables are in [platforms.md](platforms.md#x86-device-map).

Board `pc_z97_d3h_i217` is the one Z97 image. It reuses Microkit `x86_64_generic` (same kernel as QEMU x86). The system description is the Rust image renderer, not a template file. virtio-pci workstation images will **not** boot this motherboard. QEMU coverage of the VGA menu is `x86_64_generic_console` (`just test-x86-console`, PS/2), which runs `console-shell`, not this image's shell.

The image drives the onboard I217-V (`8086:153b` at `00:19.0`, subsystem `1458:e000`) and paints the Arch sign-on on the Intel GOP buffer. A UEFI boot leaves the motherboard HDMI port scanning that buffer, so the stub's blue mode line is the picture until the console paints. The mapped window is the first 16 MiB of the HD Graphics 4600 aperture at `0xd0000000`. The console paints the 80 by 25 shell there when the mode record names an address inside that window. `sddf-e1000e-driver` is the network driver, with the virtualisers, the copier, `console-driver`, and `sddf-shell`. There is no `serial-driver`. The register window is `0xf7e00000` (128 KiB). The console owns port `0xcf8`, arms the USB keyboard, and writes the I217 window. The I/O APIC pin is 11 and the vector is 51, from the interrupt line register on this desk. The shell note says to type `ping`. Success on that screen is `ip` and the address, then `ping ok`. A miss is `ip err` or `ping err`. Any other line prints `err`. This image leaves the Windows disk alone. Build and write it with `just burn-lerux-iso-z97`. `just burn-lerux-iso-swift` writes the Swift image. `just test-x86-e1000e` is the QEMU 82574 image.

The image maps the xHCI window so the desk dongle can type. The separate QEMU USB-keyboard board is not in this tree. The desk keyboard is the USB HID boot dongle on the Intel controller at `00:14.0`, or a PS/2 keyboard in the rear combo port. Keyboards behind a hub are not seen.

This machine is also the Ubuntu build host. Booting lerux **reboots Linux**. Keep Ubuntu as the default EFI entry. Do **not** format or overwrite `sda`.

#### VGA text on the RX 560

Checked 2026-10-03 while Ubuntu was driving the monitor through the RX 560. The cable was HDMI on a Sapphire Radeon RX 560 (Polaris 11 / Baffin):

| Field | This desk |
|-------|-----------|
| PCI | `01:00.0` |
| ID | `1002:67ff` rev cf, Sapphire subsystem `1da2:e348`, 4 GB |
| Linux | `amdgpu`, DRM `card2-HDMI-A-3`, 1920×1080, Wayland |
| Firmware | boot VGA (`boot_vga=1`) |

Checked 2026-10-22. The HDMI cable is on the motherboard port. Linux shows the Intel HD Graphics 4600 (`00:02.0`, `8086:0412`, `i915`, DRM `card1-HDMI-A-1`) connected at 1920×1080. The RX 560 HDMI connector is disconnected. On 2026-10-22 the Setup variable's Initial Display Output was set to `IGFX` (data offset 3, value 0). Intel Processor Graphics was already enabled, and CSM Support was already Always. After the reboot, firmware reports the Intel device as the boot VGA device (`boot_vga=1`) and the RX 560 as `boot_vga=0`. A later legacy Limine boot the same day published a linear framebuffer at `0xd0000000`, pitch 4096, 1024 by 768, 32 bits per pixel. Red is 8 bits at shift 16, green is 8 bits at shift 8, and blue is 8 bits at shift 0.

The desk-browser spec ([`desk-browser.md`](desk-browser.md)) keeps the cable on the motherboard HDMI port ([ADR-013](decisions/013-desk-intel-scanout.md)). Firmware Peripherals → Initial Display Output is `IGFX`, and Intel Processor Graphics stays enabled. `console-driver` paints the classic 80 by 25 cells into the firmware buffer the mode record names inside the `0xd0000000` aperture (the recorded Limine boot was pitch 4096, 1024 by 768) and draws the underline in those pixels. After `IGFX` is set, that buffer is the picture on the motherboard HDMI port. lerux does not program the Intel display engine, `amdgpu`, or the Polaris display engine. The RX 560 stays unused.

The desk-browser spec names `console-driver`, `serial-driver`, `log-server`, `browser-ui`, one `web-content`, `request-server`, `tls-proxy`, `net-server`, and a new onboard Ethernet driver as the image this desk boots. Those named protection domains are the spec. The crates are not in this tree. On that image `console-driver` forwards key-downs and does not paint the text page. COM1 stays the log. The shell and the filesystem are absent. `x86_64_generic_console` remains the QEMU console test.

Before a metal boot, a separate QEMU image runs those browser protection domains with the existing x86 virtio network driver. It loads `https://host:8443/desk.html` from `lerux https-one`, follows the link to `https://host:8443/desk-link.html`, and submits `q=hi` to `https://host:8443/desk-form.html?q=hi`. COM1 records each address and the first non-white pixel. That boot is not the metal product.

Use the USB boot entry that says UEFI (GRUB `EFI/BOOT/BOOTX64.EFI`). `sel4.elf` (64-bit) remains linked at 1 MiB; GRUB loads `/boot/mb2-stub.elf` and the stub places the kernel after `ExitBootServices`. The documented result is 80 by 25 cells in the firmware buffer, light gray on black, with the Arch sign-on and `[root@lerux ~]# `. The cursor is an underline. A boot of this paint on the Z97 has not been confirmed. If only the blue mode line shows, the hex address after `fb 0x` says whether the firmware buffer is inside the mapped window. The QEMU proof of the GRUB path is `cargo run -p lerux-cli -- iso --board pc_z97_d3h_i217 --boot-test` (OVMF UEFI with `sel4.elf`).

An AMD modesetting driver would matter only if lerux itself programmed this card (resolution, HDMI, a linear framebuffer) or had to bring the picture up when the firmware had not left VGA text mode. That work is still deferred. The QEMU `ramfb` display server does not drive this card.

**USB ISO** (writes `build/pc_z97_d3h_i217/lerux_pc_z97_d3h_i217.iso` and does not touch a disk). `just burn-lerux-iso-z97` builds the ISO, lists the USB sticks, asks which one to erase, and writes it. The manual sequence is build the image, verify the digest, write the media, and boot. `lerux verify-image` prints the flash line after the digest matches. The ISO path is `lerux iso --board pc_z97_d3h_i217 --no-build` and the `dd` line. The other path is `just deploy-pc` onto a mounted FAT volume. The printed FAT line is a comment, so pasting the flash block writes only the ISO.

```bash
just make-lerux-iso-z97
lerux verify-image --board pc_z97_d3h_i217
lsblk
sudo dd if=build/pc_z97_d3h_i217/lerux_pc_z97_d3h_i217.iso of=/dev/sdX bs=4M status=progress conv=fsync
```

The guest cannot install a package. The operator rebuilds on the host with `lerux package install --board pc_z97_d3h_i217` or `lerux package upgrade --board pc_z97_d3h_i217`. That command prints the pin digest and writes `loader.img.sha256`.

Replace `sdX` with the flash drive from `lsblk`. Do not use `sda` (Ubuntu SSD) or `sdb` (data disk). Reboot and open the firmware boot menu (F12). Choose the USB entry that says UEFI (GRUB `EFI/BOOT/BOOTX64.EFI`). GRUB boots immediately (`timeout=0`). The image paints 80 by 25 cells in the firmware buffer, light gray on black, with the Arch sign-on and `[root@lerux ~]# `. Type with a PS/2 keyboard in the rear combo port, or a USB HID boot keyboard on a root xHCI port (the Microsoft Wireless Desktop 900 dongle on this machine is one). Keyboards behind a hub are not seen. `ping` brings up DHCP on the onboard port and pings the gateway. Reboot and choose Ubuntu to return.

The ISO is a UEFI image. GRUB Multiboot 2-boots `/boot/mb2-stub.elf` (linked at 32 MiB) with modules `/boot/sel4.elf` and `/boot/loader.img`. The stub ELF-loads the 64-bit kernel to 1 MiB after `ExitBootServices`. Every x86 QEMU smoke packs that ISO and boots it under **OVMF** (needs the `ovmf` package or `OVMF_PATH`, plus `xorriso` and `grub-mkimage`. Arch: `sudo pacman -S libisoburn grub`. Debian: `xorriso` and `grub-efi-amd64-bin`). `cargo run -p lerux-cli -- iso --board pc_z97_d3h_i217 --boot-test` is the same path without QMP keys.

**FAT volume that already has a bootloader** (after mounting it; not the Ubuntu ESP):

```bash
lerux verify-image --board pc_z97_d3h_i217
DEST=/media/$USER/boot just deploy-pc
# equivalent: cargo run -p lerux-cli -- deploy --board pc_z97_d3h_i217 --dest /media/$USER/boot
```

That writes GRUB's `EFI/BOOT/BOOTX64.EFI`, `/boot/grub/grub.cfg`, and `boot/mb2-stub.elf`, `boot/sel4.elf`, `boot/loader.img`, plus SHA-256 sidecars beside the kernel and the loader. It removes a leftover root `lerux-limine.conf`, `lerux-grub.cfg`, `mb2-stub.elf`, `sel4.elf`, or `loader.img`. Do not replace the Ubuntu GRUB menu on `sda`.

**Full path**

| Step | Command / action |
|------|------------------|
| 1. SDK | `MICROKIT_BOARDS=…,x86_64_generic just build-sdk` or `just fetch-sdk` (already required for QEMU x86) |
| 2. Serial hardware (optional, kernel output only) | Gigabyte **COMA** 9-pin header → DB9 cable → laptop USB-serial at **115200 8N1**. There is no rear DB9. Prove the link from Linux (`/dev/ttyS0` is `0x3F8` IRQ 4) before seL4. |
| 3. Image | `BOARD=pc_z97_d3h_i217 just image`, then `lerux verify-image --board pc_z97_d3h_i217` |
| 4. Media | `lerux iso --board pc_z97_d3h_i217 --no-build`, then `dd` that ISO onto the flash drive (not `sda` or `sdb`). Or `DEST=/path/to/usb-fat just deploy-pc` to write the GRUB layout onto a mounted volume. |
| 5. Boot | Firmware boot menu (F12) → the USB entry that says UEFI (GRUB `EFI/BOOT/BOOTX64.EFI`). GRUB is Multiboot 2: kernel `/boot/mb2-stub.elf`, modules `/boot/sel4.elf` and `/boot/loader.img`. |
| 6. Back to Ubuntu | Reboot and pick the default Ubuntu EFI entry. |

**Prerequisites**

- Gigabyte GA-Z97-D3H, i5-4690K (or another PC99 box with the same COM1 + CPU flags: `pat`, `xsave`, `fpu`, `sse`, `pdpe1gb`, `fsgsbase`)
- PS/2 keyboard in the rear combo port, or a USB HID boot keyboard on a root xHCI port
- Monitor cable on the motherboard HDMI port. Set Peripherals → Initial Display Output to `IGFX` before a boot that should appear there. See [VGA text on the RX 560](#vga-text-on-the-rx-560).
- USB stick (or a unused FAT partition). Not the Ubuntu root disk.
- Ethernet cable from the onboard port to a router that hands out DHCP leases

**Pass criteria**

| Check | Pass |
|-------|------|
| Screen | Gray on black, the Arch sign-on, then `[root@lerux ~]# ` |
| Keyboard and network | `ping` then Enter prints `ip` and the address, then `ping ok` |
| Ubuntu | `sda` still boots after the experiment |

**Likely failure modes**

| Symptom | Likely cause |
|---------|----------------|
| Blue mode line, then no sign-on | The console did not paint. Read the hex address after `fb 0x`: the console paints only inside the 16 MiB window at `0xd0000000`. Kernel output is still COM1 (COMA header, 115200 8N1). |
| Prompt is visible, keys do nothing | The combo port has no PS/2 keyboard and the USB keyboard was not armed. A boot keyboard on a hub is not seen. |
| Boot stops while loading `sel4.elf` at 1 MiB | An old ISO that Multiboot 2-boots the kernel ELF directly. Rebuild with `just make-lerux-iso-z97` so the menu loads `/boot/mb2-stub.elf`, then modules `/boot/sel4.elf` and `/boot/loader.img`. |
| Boot stops loading the `sel4.elf` module | The module was only in the ISO9660 session. Rebuild with `just make-lerux-iso-z97` and rewrite the USB stick. The EFI system partition is `boot/esp.img` and holds `mb2-stub.elf`, `sel4.elf`, and `loader.img`. |
| HDMI picture never changes | The cable is not on the firmware primary VGA device, or this was the UEFI entry. Until Initial Display Output is `IGFX`, that device is still the RX 560. This image does not modeset. See [VGA text on the RX 560](#vga-text-on-the-rx-560). |
| `ip err` | No DHCP lease on the onboard port. Check the cable and the router. |
| Ubuntu gone | You deployed onto `sda` — don't. Use USB. |

This board has no native disk driver. QEMU `workstation-x86` is virtio-pci only. The native plan is [Milestones 19–22](plan-lionsos.md#milestone-19--advanced-host-controller-interface-disk).

### Dell Inspiron N5010 install path

Board `inspiron_n5010` is the VGA-only image for the Dell Inspiron N5010 (Core i5-460M, HM57, BIOS A15). The one ISO from `just make-lerux-iso-swift` also boots this machine: the stub does not see the Swift panel, so it paints this same VGA text page and does not map the AHCI disk. The board image itself reuses Microkit `x86_64_generic` and `console-inspiron.system.template`. The probe is [support/arch-probe/machines/inspiron-n5010.md](../support/arch-probe/machines/inspiron-n5010.md). Device roles are in [platforms.md](platforms.md#inspiron-n5010). This image has no QEMU model.

The 2026-11-14 boot was legacy BIOS with `nomodeset`. The console was the VGA text page, 80 by 25, at `0xb8000`, scanned by the integrated graphics `8086:0046` at `00:02.0`. The log has no `efifb` or `vesafb` line, so this image does not map a GOP buffer. The keyboard is the built-in i8042 at `0x60` and `0x64`, I/O APIC pin 1, vector 49. The probe recorded no IRQ 1 override. There is no serial port and no xHCI controller. The shell is `console-shell`. The page is light gray on black. The sign-on ends at `[root@lerux ~]# `. `1` or `browser` prints `browser unavailable`, and `2` or `maths` is the `lerux-calc` session.

The disk is the WDC WD5000BEVT-75A0RT0 on AHCI `8086:3b2f` at `00:1f.2`. This image does not map that controller and does not format the volume. Write the ISO only to a USB flash drive.

**USB ISO** (writes `build/inspiron_n5010/lerux_inspiron_n5010.iso` and does not touch a disk):

```bash
just make-lerux-iso-inspiron
lsblk
sudo dd if=build/inspiron_n5010/lerux_inspiron_n5010.iso of=/dev/sdX bs=4M status=progress conv=fsync
```

Replace `sdX` with the flash drive from `lsblk`. Do not write the internal SATA disk. The ISO is UEFI only, the same GRUB Multiboot 2 path as the Swift X and the Z97 desk: `EFI/BOOT/BOOTX64.EFI` loads `/boot/mb2-stub.elf` with modules `/boot/sel4.elf` and `/boot/loader.img`. The recorded boot did not publish a UEFI entry. A metal boot has to show whether firmware offers a USB entry that says UEFI. There is no BIOS El Torito entry.

The page, if the integrated graphics still scans VGA text, is light gray on black, with the sign-on and `[root@lerux ~]# `. Type on the built-in keyboard. This laptop has no COM port. `just test-x86-console` boots a different machine (QEMU q35, COM1, and an xHCI window) and does not close this boot.

## QEMU profiles

| `qemu` field | Used by | Extra QEMU args |
|--------------|---------|-----------------|
| `aarch64` | hello, echo | stock `qemu-system-aarch64` virt |
| `aarch64_init` | init | patched SP804 QEMU |
| `aarch64_virtio` | virtio | virtio-net + virtio-blk + `disk.img` |
| `aarch64_blk` | blk | virtio-blk + `disk.img` (read-write) |
| `aarch64_blk_composed` | blk-composed | patched SP804 QEMU + virtio-blk + `disk.img` (read-write) |
| `aarch64_net` | net | virtio-net only (no `disk.img`) |
| `aarch64_fetch` | fetch | virtio-net only; host HTTP on `127.0.0.1:8081` for guest `10.0.2.2:8081` |
| `aarch64_net_composed` | net-composed | patched SP804 QEMU + virtio-net |
| `aarch64_ipc_composed` | ipc-composed | patched SP804 QEMU + virtio-blk/net + `disk.img` (read-write) |
| `aarch64_composed` | composed | patched SP804 QEMU + virtio + `disk.img` |
| `aarch64_http` | http | virtio-net + `hostfwd=tcp::18080-:8080` |
| `aarch64_http_composed` | http-composed | patched SP804 QEMU + virtio-net + `hostfwd` |
| `riscv64` | riscv hello/echo | `-kernel loader.img` |
| `riscv64_virtio` | riscv virtio | MMIO virtio buses + `disk.img` |
| `riscv64_blk` | riscv blk | MMIO virtio-blk bus.0 + `disk.img` |
| `riscv64_net` | riscv net | MMIO virtio-net bus.1 (no `disk.img`) |
| `riscv64_http` | riscv http | MMIO virtio-net bus.1 + `hostfwd=tcp::18080-:8080` |
| `x86_64` | x86 hello/echo | `-machine q35` + OVMF + GRUB ISO (`sel4.elf` via mb2-stub) |
| `x86_64_virtio` | x86 virtio | q35 + PCI virtio-blk/net + `disk.img` |
| `x86_64_blk` | x86 blk | q35 + PCI virtio-blk + `disk.img` |
| `x86_64_net` | x86 net | q35 + PCI virtio-net (no `disk.img`) |
| `x86_64_http` | x86 http | q35 + PCI virtio-net + `hostfwd=tcp::18080-:8080` |

## HTTP boards

`qemu_virt_aarch64_http` serves `GET /` on guest port **8080** (`10.0.2.15`). QEMU user netdev forwards host `127.0.0.1:18080` → guest `:8080`; smoke uses `curl` after serial shows `lerux-http: listening`.

`x86_64_generic_http` uses the same HTTP PD and hostfwd layout on QEMU **q35** with PCI virtio-net via `virtio-pci-driver` (net-only). See [plan.md](plan.md) Phase 19.

`qemu_virt_riscv64_http` serves HTTP over MMIO virtio-net on `virtio-mmio-bus.1` (same layout as riscv virtio hello). See [plan.md](plan.md) Phase 22.

## HTML and program boards

`qemu_virt_aarch64_html` (Phase 74) runs `html-demo`, which parses the baked-in `support/browser/fixture.html` with `lerux-html` and logs `lerux-html: nodes=8`. No NIC or filesystem.

`qemu_virt_aarch64_program_sddf` (Milestone 14) is the fetch image with `program` in place of the shell. The module reads `/config/hostname` and allocates a socket through `lerux-posix`. A bad signature prints `program: rejected` and is not instantiated. The signed module then prints `lerux-wasm: ran`. See [ADR-012](decisions/012-ordinary-webassembly.md).

Milestone 17 stays. It removed `lerux-interface-types` with the postcard guest model. It did not remove `net-server`, `request-server`, `tls-proxy`, or `display-server` from the browser specs. The live browser image is `qemu_virt_aarch64_browser_sddf`.

### x86 HTTP inbound (operational notes)

On x86, `http-server` returns from `init()` after printing `lerux-http: listening` and handles inbound `GET /` via virtio-pci-driver notifications (same model as aarch64 HTTP).

**Automated smoke (preferred):**

```bash
just test-x86-http
```

`lerux test` retries HTTP checks for up to 30s and always terminates QEMU on exit (avoids orphan instances on port 18080).

**Interactive QEMU:**

```bash
BOARD=x86_64_generic_http just qemu-x86_64-http
# other terminal, after "listening" (brief pause or retry helps):
sleep 1 && curl http://127.0.0.1:18080/
```

**Port 18080 — one listener at a time.** Host port 18080 is shared by:

| Consumer | Command / context |
|----------|-------------------|
| x86/aarch64/riscv HTTP hostfwd | `just test-x86-http`, `just test-http`, `just test-riscv-http` |
| TCP echo (virtio outbound tests) | `just test-x86-virtio`, `cargo run -p lerux-cli -- tcp-echo 18080` |

Do **not** run background QEMU and `just test-x86-http` concurrently. A stale QEMU or leftover `tcp-echo-server` on 18080 makes `curl` hit the wrong endpoint and time out even when the new guest has reached `listening`.

**Cleanup before retry:**

```bash
pkill -f 'tcp-echo 18080'
pkill -f 'qemu-system-x86_64.*hostfwd=tcp::18080-:8080'
just test-x86-http
```

`just qemu-x86_64-http` and the `x86_64_http` smoke recipe run the same `pkill` patterns before starting QEMU.