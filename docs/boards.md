# Boards

Board names are the `BOARD=` value for `just run`, `just test`, and `just build`. Metadata lives in [`support/boards.toml`](../support/boards.toml).

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
| `pc_z97_d3h` | x86_64 | `BOARD=pc_z97_d3h just image` | same menu on the desk VGA text page (hardware only) |
| `x86_64_generic_virtio` | x86_64 | `just test-x86-virtio` | hello + serial + virtio-pci blk/net |
| `x86_64_generic_http` | x86_64 | `just test-x86-http` | serial + virtio-pci net + http-server |
| `rpi4b_4gb` | aarch64 | `BOARD=rpi4b_4gb just image` | hello + serial (PL011; hardware only) |
| `pc_z97_d3h_rtl8852be` | x86_64 | `BOARD=pc_z97_d3h_rtl8852be just image` | COM1 plus a config-space read of the RTL8852BE (hardware only) |

Milestone 17 deleted the postcard boards. Their rows, including `qemu_virt_aarch64_browser` and `just test-browser`, are gone. The live browser smoke is `just test-browser-sddf`.

## SDK boards

`just build-sdk` compiles kernel + loader for Microkit board names (not always identical to lerux `BOARD`):

```bash
MICROKIT_BOARDS=qemu_virt_aarch64,x86_64_generic,qemu_virt_riscv64,rpi4b_4gb just build-sdk
```

RPi4 workstation images require `rpi4b_4gb` in `MICROKIT_BOARDS` (e.g. `MICROKIT_BOARDS=qemu_virt_aarch64,rpi4b_4gb just build-sdk`).

CI sets this via `MICROKIT_BOARDS` in the workflow env.

## Hardware boards (Phase 37+)

Real (non-QEMU) boards have no `qemu` field. ARM/RISC-V produce `loader.img`; x86 also stages `sel4_32.elf` beside it for Multiboot 2.
Use `lerux image --board <name>` (or `BOARD=<name> just image`).

- `rpi4b_4gb`: Raspberry Pi 4 Model B (4 GB). Requires U-Boot on SD card. See seL4 docs for initial bring-up and `fatload` / `go`.
- Serial: PL011 at 0xfe201000 (GPIO 14/15). Update IRQ in `boards.toml` if the platform IRQ mapping differs.
- Full workstation (FS + net) on hardware requires native (non-virtio) block and network drivers; see `support/profiles/hardware-rpi4.toml`.
- `pc_z97_d3h`: Gigabyte GA-Z97-D3H (Haswell). Microkit still uses `x86_64_generic`; deploy copies **`sel4_32.elf` + `loader.img`** for Multiboot 2. The monitor shows the shell as VGA text on the firmware boot VGA card. The desk-browser spec makes that card the Intel HD Graphics 4600 on the motherboard HDMI port, with Initial Display Output set to `IGFX`. After the reboot on 2026-10-22, firmware reports that device as the boot VGA device. See [Gigabyte Z97-D3H install path](#gigabyte-z97-d3h-install-path) and [VGA text on the RX 560](#vga-text-on-the-rx-560).
- `pc_z97_d3h_rtl8852be`: same motherboard, COM1 only. The probe reads root port `00:1c.6`, takes that port's secondary bus, and logs the endpoint vendor, device, and assigned BAR. It does not map the BAR, and it is not the shell ISO.

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

```bash
# Build (if needed) + verify loader.img.sha256 + copy image/sidecar + write lerux-uboot.txt
DEST=/media/$USER/boot just deploy-rpi4
# equivalent: cargo run -p lerux-cli -- deploy --board rpi4b_4gb_workstation --dest /media/$USER/boot
# Skip integrity check only if needed: lerux deploy … --no-verify
# Manual: lerux digest / lerux verify-image  (Phase 60 Track C; docs/security.md)
```

**Full path**

| Step | Command / action |
|------|------------------|
| 1. SDK | `MICROKIT_BOARDS=…,rpi4b_4gb just build-sdk` or `just fetch-sdk` |
| 2. Image | `BOARD=rpi4b_4gb_workstation just image` (or deploy builds it) |
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

### Gigabyte Z97-D3H install path

Part names for this desk and for QEMU q35 are in [GLOSSARY.md](../GLOSSARY.md). The driver tables are in [platforms.md](platforms.md#x86-device-map).

Board `pc_z97_d3h` is the **on-screen menu** on this Haswell desktop. It reuses Microkit `x86_64_generic` (same kernel as QEMU x86) and `console-pc.system.template` (COM1 `0x3f8`, IOAPIC pin 4, the VGA text page at `0xb8000`, a PS/2 keyboard, and a USB HID boot keyboard on the xHCI controller at PCI `00:14.0`). QEMU coverage of that image is `x86_64_generic_console` (`just test-x86-console`, PS/2). virtio-pci workstation images will **not** boot this motherboard.

The template maps the xHCI window so the desk dongle can type. The separate QEMU USB-keyboard board is not in this tree. The desk keyboard is the USB HID boot dongle on the Intel controller at `00:14.0`, or a PS/2 keyboard in the rear combo port. Keyboards behind a hub are not seen.

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

The desk-browser spec ([`desk-browser.md`](desk-browser.md)) keeps the cable on the motherboard HDMI port ([ADR-013](decisions/013-desk-intel-scanout.md)). Firmware Peripherals → Initial Display Output is `IGFX`, and Intel Processor Graphics stays enabled. `console-driver` writes the classic 80×25 color text page at physical `0xb8000` and moves the cursor through the VGA CRTC ports `0x3d4`/`0x3d5`. The legacy Limine entry leaves the primary VGA device in that text mode. After `IGFX` is set, those writes are scanned out on the motherboard HDMI port. The spec's linear framebuffer is the VESA mode Limine sets on that same entry. lerux does not program the Intel display engine, `amdgpu`, or the Polaris display engine. The RX 560 stays unused.

The desk-browser spec names `console-driver`, `serial-driver`, `log-server`, `browser-ui`, one `web-content`, `request-server`, `tls-proxy`, `net-server`, and a new onboard Ethernet driver as the image this desk boots. Those named protection domains are the spec. The crates are not in this tree. On that image `console-driver` forwards key-downs and does not paint the text page. COM1 stays the log. The shell and the filesystem are absent. The Phase 82 shell image remains the QEMU console test.

Before a metal boot, a separate QEMU image runs those browser protection domains with the existing x86 virtio network driver. It loads `https://host:8443/desk.html` from `lerux https-one`, follows the link to `https://host:8443/desk-link.html`, and submits `q=hi` to `https://host:8443/desk-form.html?q=hi`. COM1 records each address and the first non-white pixel. That boot is not the metal product.

Use the USB boot entry that says UEFI (Limine `BOOTX64.EFI`). `sel4_32.elf` remains linked at 1 MiB; Limine loads `mb2-stub.elf` at 32 MiB and the stub places the kernel after `ExitBootServices`. The documented result is a blue screen with `hello lerux` on the first line and the menu `1 browser` / `2 maths` under it. A legacy/CSM USB entry still works with the same ISO. A metal UEFI boot of this image has not been recorded on this machine yet. The QEMU proofs are `just test-x86-console` (SeaBIOS) and `just iso --boot-test` (SeaBIOS plus OVMF when available).

An AMD modesetting driver would matter only if lerux itself programmed this card (resolution, HDMI, a linear framebuffer) or had to bring the picture up when the firmware had not left VGA text mode. That work is still deferred. The QEMU `ramfb` display server does not drive this card.

**USB ISO** (writes `build/pc_z97_d3h/lerux.iso` and does not touch a disk):

```bash
just iso
lsblk
sudo dd if=build/pc_z97_d3h/lerux.iso of=/dev/sdX bs=4M status=progress conv=fsync
```

Replace `sdX` with the flash drive from `lsblk`. Do not use `sda` (Ubuntu SSD) or `sdb` (data disk). Reboot and open the firmware boot menu (F12). Choose the USB entry that says UEFI. The Limine menu counts down, then the screen is blue with `hello lerux` on the first line and the menu `1 browser` / `2 maths` under it. Type with a PS/2 keyboard in the rear combo port, or a USB HID boot keyboard on a root xHCI port (the Microsoft Wireless Desktop 900 dongle on this machine is one). Keyboards behind a hub are not seen. `1` or `browser` prints `browser unavailable` (this image has no network and no page painter). `2` or `maths` reads integer expressions until `quit`. The serial console is still COM1 at 115200 8N1 on the COMA header and shows the same menu. `console-driver: usb keyboard` means the USB boot keyboard was armed. Reboot and choose Ubuntu to return.

The ISO is a hybrid image. Limine Multiboot 2-boots `/boot/mb2-stub.elf` (linked at 32 MiB) with modules `/boot/sel4_32.elf` and `/boot/loader.img`. The stub ELF-loads the kernel to 1 MiB after Limine has called `ExitBootServices`, so the UEFI entry works; the same path works under CSM/SeaBIOS. `cargo run -p lerux-cli -- iso --board pc_z97_d3h --boot-test` boots that file in QEMU as a raw disk under SeaBIOS and, when OVMF is installed, under UEFI (local check, not CI).

**FAT volume that already has a bootloader** (after mounting it; not the Ubuntu ESP):

```bash
DEST=/media/$USER/boot just deploy-pc
# equivalent: cargo run -p lerux-cli -- deploy --board pc_z97_d3h --dest /media/$USER/boot
```

That copies `sel4_32.elf` + `loader.img` (and SHA-256 sidecars) and writes `lerux-limine.conf` / `lerux-grub.cfg` snippets. Paste a snippet into that volume's bootloader. Do not replace the Ubuntu GRUB menu on `sda`.

**Full path**

| Step | Command / action |
|------|------------------|
| 1. SDK | `MICROKIT_BOARDS=…,x86_64_generic just build-sdk` or `just fetch-sdk` (already required for QEMU x86) |
| 2. Serial hardware | Gigabyte **COMA** 9-pin header → DB9 cable → laptop USB-serial at **115200 8N1**. There is no rear DB9. Prove the link from Linux (`/dev/ttyS0` is `0x3F8` IRQ 4) before seL4. |
| 3. Image | `BOARD=pc_z97_d3h just image` (or deploy builds it) |
| 4. Media | `just iso`, then `dd` the ISO onto the flash drive (not `sda` or `sdb`). Or `DEST=/path/to/usb-fat just deploy-pc` when that volume already has a bootloader. |
| 5. Boot | Firmware boot menu (F12) → the USB entry that says UEFI. Limine is Multiboot 2: kernel `/boot/mb2-stub.elf`, modules `/boot/sel4_32.elf` and `/boot/loader.img`. CSM/legacy still works with the same media. |
| 6. Host smoke | From the **laptop**: `LERUX_HW_SERIAL=/dev/ttyUSB0 BOARD=pc_z97_d3h just test-hw` |
| 7. Back to Ubuntu | Reboot and pick the default Ubuntu EFI entry. |

**Prerequisites**

- Gigabyte GA-Z97-D3H, i5-4690K (or another PC99 box with the same COM1 + CPU flags: `pat`, `xsave`, `fpu`, `sse`, `pdpe1gb`, `fsgsbase`)
- PS/2 keyboard in the rear combo port, or a USB HID boot keyboard on a root xHCI port
- Monitor cable on the motherboard HDMI port. Set Peripherals → Initial Display Output to `IGFX` before a boot that should appear there. See [VGA text on the RX 560](#vga-text-on-the-rx-560).
- USB stick (or a unused FAT partition). Not the Ubuntu root disk.
- Optional: second computer as serial console, to see `1 browser` on COM1

**Pass criteria**

| Check | Pass |
|-------|------|
| Screen | Blue background, `hello lerux` on the first line, then `1 browser` and `2 maths` |
| Keyboard | `2` then Enter, then `6/2` then Enter, prints `maths 3` |
| Serial | Kernel + `serial driver: NS16550 COM1` + `1 browser` |
| Ubuntu | `sda` still boots after the experiment |

**Likely failure modes**

| Symptom | Likely cause |
|---------|----------------|
| Limine menu, then a flashing cursor and no blue screen | The console PD did not run. Kernel output is still COM1 (COMA header, 115200 8N1). |
| Prompt is visible, keys do nothing | Serial shows `xhci none` or `xhci no keyboard`, and the combo port has no PS/2 keyboard. A boot keyboard on a hub is not seen. If the log already says `usb keyboard`, the xHCI interrupt did not arrive. |
| Limine says it could not find a load address | An old ISO that still Multiboot 2-boots `sel4_32.elf` directly. Rebuild with `just iso` so the menu loads `mb2-stub.elf`. |
| HDMI picture never changes | The cable is not on the firmware primary VGA device, or this was the UEFI entry. Until Initial Display Output is `IGFX`, that device is still the RX 560. This image does not modeset. See [VGA text on the RX 560](#vga-text-on-the-rx-560). |
| Kernel on serial, no prompt | COM1 IRQ pin/polarity; try `trigger="edge"` on the serial irq in the SDF |
| Silent serial | Cable on COMA header (TX/RX swapped); 115200 8N1; seL4 debug UART vs PD |
| Ubuntu gone | You deployed onto `sda` — don't. Use USB. |

This board has no native disk or Ethernet driver. QEMU `workstation-x86` is virtio-pci only. The native plan is [Milestones 19–22](plan-lionsos.md#milestone-19--advanced-host-controller-interface-disk).

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
| `x86_64` | x86 hello/echo | `-machine q35` + `-kernel sel4_32.elf` + `-initrd loader.img` |
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

The postcard net, request, paint, browser, agent, and filesystem boards are gone. Milestone 17 deleted `net-server`, `request-server`, `tls-proxy`, `display-server`, and `lerux-interface-types`. The live browser image is `qemu_virt_aarch64_browser_sddf`.

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