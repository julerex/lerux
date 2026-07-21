# Boards

Board names are the `BOARD=` value for `just run`, `just test`, and `just build`. Metadata lives in [`support/boards.toml`](../support/boards.toml).

**System profiles** (Phase 35) live in `support/profiles/*.toml` (e.g. `workstation`, `minimal`). They declare the PD list, template, and channel manifest. Use `lerux profile build <name>` (it resolves to a board via `default_board` or `--board`). Board entries in `boards.toml` still define hardware details + qemu config; profile tooling selects compositions on top of boards.

## Reference

| Board | Arch | Smoke command | PDs (summary) |
|-------|------|---------------|---------------|
| `qemu_virt_aarch64` | aarch64 | `just test` | hello + serial |
| `qemu_virt_aarch64_debug` | aarch64 | `just test-debug` | debug-handler (parent) + crash-demo (child fault) |
| `qemu_virt_aarch64_isolation` | aarch64 | `just test-isolation` | Phase 60: crash-demo fault then fs-client vs live fs-server |
| `qemu_virt_aarch64_echo` | aarch64 | `just test-echo` | echo client/server + serial |
| `qemu_virt_aarch64_virtio` | aarch64 | `just test-virtio` | hello + serial + virtio blk/net |
| `qemu_virt_aarch64_blk` | aarch64 | `just test-blk` | blk client/server + serial + virtio-blk |
| `qemu_virt_aarch64_blk_composed` | aarch64 | `just test-blk-composed` | supervisor + init drivers + blk IPC + virtio-blk |
| `qemu_virt_aarch64_net` | aarch64 | `just test-net` | net client/server + serial + virtio-net |
| `qemu_virt_aarch64_fetch` | aarch64 | `just test-fetch` | fetch-client + net-server + serial + virtio-net |
| `qemu_virt_aarch64_fs` | aarch64 | `just test-fs` | fs-client + fs-server (LERUXFS2) + serial + virtio-blk |
| `qemu_virt_aarch64_fs_fat` | aarch64 | `just test-fs-fat` | same SDF; fs-server FAT16 backend |
| `qemu_virt_aarch64_net_composed` | aarch64 | `just test-net-composed` | supervisor + init drivers + net IPC + virtio-net |
| `qemu_virt_aarch64_ipc_composed` | aarch64 | `just test-ipc-composed` | supervisor + init drivers + blk/net IPC + virtio-blk/net |
| `qemu_virt_aarch64_init` | aarch64 | `just test-init` | supervisor + PL031 + SP804 + serial |
| `qemu_virt_aarch64_composed` | aarch64 | `just test-composed` | supervisor + hello virtio + all drivers |
| `qemu_virt_aarch64_http` | aarch64 | `just test-http` | serial + virtio-net + http-server |
| `qemu_virt_aarch64_http_composed` | aarch64 | `just test-http-composed` | supervisor + init drivers + virtio-net + http-server |
| `qemu_virt_aarch64_workstation` | aarch64 | `just test-workstation` | full workstation (profile `workstation`) |
| `qemu_virt_riscv64` | riscv64 | `just test-riscv` | hello + serial (MMIO UART) |
| `qemu_virt_riscv64_echo` | riscv64 | `just test-riscv-echo` | echo + serial |
| `qemu_virt_riscv64_virtio` | riscv64 | `just test-riscv-virtio` | hello + serial + virtio |
| `qemu_virt_riscv64_init` | riscv64 | `just test-init-riscv` | supervisor + Goldfish RTC + rdtime timer + serial |
| `qemu_virt_riscv64_workstation` | riscv64 | `just test-workstation-riscv` | full workstation (Phase 59) |
| `x86_64_generic_workstation` | x86_64 | `just test-workstation-x86` | full workstation PCI virtio (Phase 59) |
| `qemu_virt_aarch64_bench_echo` | aarch64 | `just bench-echo` | echo microbench (Phase 49) |
| `qemu_virt_aarch64_bench_blk` | aarch64 | `just bench-blk` | blk microbench |
| `qemu_virt_aarch64_bench_net` | aarch64 | `just bench-net` | net microbench |
| `qemu_virt_riscv64_blk` | riscv64 | `just test-riscv-blk` | blk client/server + serial + virtio-blk |
| `qemu_virt_riscv64_net` | riscv64 | `just test-riscv-net` | net client/server + serial + virtio-net |
| `qemu_virt_riscv64_http` | riscv64 | `just test-riscv-http` | serial + virtio-net + http-server |
| `x86_64_generic` | x86_64 | `BOARD=x86_64_generic just test` | hello + serial (COM1) |
| `x86_64_generic_echo` | x86_64 | `just test-x86-echo` | echo + serial |
| `x86_64_generic_init` | x86_64 | `just test-init-x86` | supervisor + CMOS RTC + TSC timer + serial |
| `x86_64_generic_virtio` | x86_64 | `just test-x86-virtio` | hello + serial + virtio-pci blk/net |
| `x86_64_generic_blk` | x86_64 | `just test-x86-blk` | blk client/server + serial + virtio-pci blk |
| `x86_64_generic_net` | x86_64 | `just test-x86-net` | net client/server + serial + virtio-pci net |
| `x86_64_generic_http` | x86_64 | `just test-x86-http` | serial + virtio-pci net + http-server |
| `rpi4b_4gb` | aarch64 | `BOARD=rpi4b_4gb just image` | hello + serial (PL011; hardware only) |
| `rpi4b_4gb_workstation` | aarch64 | `just test-rpi4-workstation` | workstation over native genet + emmc2 (hardware only) |
| `rpi4b_4gb_net` | aarch64 | `BOARD=rpi4b_4gb_net just image` | net slice on hardware |
| `rpi4b_4gb_blk` | aarch64 | `BOARD=rpi4b_4gb_blk just image` | blk slice on hardware |

## SDK boards

`just build-sdk` compiles kernel + loader for Microkit board names (not always identical to lerux `BOARD`):

```bash
MICROKIT_BOARDS=qemu_virt_aarch64,x86_64_generic,qemu_virt_riscv64,rpi4b_4gb just build-sdk
```

RPi4 workstation images require `rpi4b_4gb` in `MICROKIT_BOARDS` (e.g. `MICROKIT_BOARDS=qemu_virt_aarch64,rpi4b_4gb just build-sdk`).

CI sets this via `MICROKIT_BOARDS` in the workflow env.

## Hardware boards (Phase 37+)

Real (non-QEMU) boards have no `qemu` field and produce `loader.img` only.
Use `lerux image --board <name>` (or `BOARD=<name> just image`).

- `rpi4b_4gb`: Raspberry Pi 4 Model B (4 GB). Requires U-Boot on SD card. See seL4 docs for initial bring-up and `fatload` / `go`.
- Serial: PL011 at 0xfe201000 (GPIO 14/15). Update IRQ in `boards.toml` if the platform IRQ mapping differs.
- Full workstation (FS + net) on hardware requires native (non-virtio) block and network drivers; see `support/profiles/hardware-rpi4.toml`.

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

**Manual REPL checklist** (record pass/fail for the Phase 39/52 gate)

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

## Composed board

`qemu_virt_aarch64_composed` runs two app PDs in one system:

- **supervisor** — RTC + SP804 via serial IPC (crate `supervisor`; was `boot-init` before Phase 33).
- **hello** — virtio blk/net via serial IPC; waits for `supervisor` notify before probing virtio.

See [plan.md](plan.md) Phases 15 and 24.

## HTTP boards

`qemu_virt_aarch64_http` serves `GET /` on guest port **8080** (`10.0.2.15`). QEMU user netdev forwards host `127.0.0.1:18080` → guest `:8080`; smoke uses `curl` after serial shows `lerux-http: listening`.

`qemu_virt_aarch64_http_composed` runs supervisor (RTC + SP804) then http-server over virtio-net — same notify gate as composed hello. See [plan.md](plan.md) Phase 17.

`x86_64_generic_http` uses the same HTTP PD and hostfwd layout on QEMU **q35** with PCI virtio-net via `virtio-pci-driver` (net-only). See [plan.md](plan.md) Phase 19.

`qemu_virt_riscv64_http` serves HTTP over MMIO virtio-net on `virtio-mmio-bus.1` (same layout as riscv virtio hello). See [plan.md](plan.md) Phase 22.

## Net IPC board

`qemu_virt_aarch64_net`, `qemu_virt_riscv64_net`, and `x86_64_generic_net` run `net-server` (virtio-net driver client) and `net-client` (UDP TX over IPC). Smoke expects `lerux-net: IPC ok` after `virtio-net: TX ok`. See [plan.md](plan.md) Phases 27–28.

`qemu_virt_aarch64_fetch` runs `fetch-client` over extended net IPC (DNS resolve, TCP connect/send/recv) to perform `GET /` against a host HTTP server at `10.0.2.2:8081`. Smoke expects `lerux-fetch: 200`. See [plan.md](plan.md) Phase 31.

`qemu_virt_aarch64_fs` runs `fs-client` over filesystem IPC (`Create`/`Write`/`Read`/`Stat`/`ListDir`/`Mkdir`/`Unlink`/`Rename`) backed by `fs-server` on virtio-blk with **LERUXFS2** (hierarchical dirs, multi-sector files). Smoke expects `lerux-fs: ready (LERUXFS2)` and `lerux-fs: round-trip ok` (includes nested path + multi-sector). See [plan.md](plan.md) Phase 50.

`qemu_virt_aarch64_isolation` (Phase 60) nests `crash-demo` under `debug-handler`, then notifies `fs-client` only after the untrusted PD is suspended. Smoke expects fault strings plus `lerux-isolation: fs-server survived untrusted PD crash`. See [`security.md`](security.md).

`qemu_virt_aarch64_fs_fat` uses the same SDF and IPC; `fs-server` is built with `backend-fat` (Phase 44 minimal FAT16: root-only, 8.3 names, single-cluster files; hierarchy ops return Error). Smoke expects `lerux-fs: ready (FAT16)` and `lerux-fs: round-trip ok` (basic create/read only).

`qemu_virt_aarch64_net_composed` gates net probe on supervisor notify (same composed-sync pattern as blk-composed). See [plan.md](plan.md) Phase 29.

`qemu_virt_aarch64_ipc_composed` runs supervisor plus both block and net IPC services in one system. Probes run sequentially via a notify chain: supervisor → blk-client → net-client. See [plan.md](plan.md) Phase 30.

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