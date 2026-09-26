# lerux

## Quickstart

Boot this Gigabyte Z97-D3H from a USB flash drive:

```bash
just iso  # build build/pc_z97_d3h/lerux.iso
lsblk  # list disks; the USB stick is sdc on this machine (not sda or sdb)
udisksctl unmount -b /dev/sdc3  # unmount the ISO filesystem Ubuntu mounted from the stick
sudo dd if=build/pc_z97_d3h/lerux.iso of=/dev/sdc bs=4M status=progress conv=fsync  # write that ISO onto the whole stick; bs=4M is the chunk size, status=progress prints bytes written, conv=fsync waits until they are on the device
```

Check `lsblk` before the `dd`. Do not write `sda` (Ubuntu SSD) or `sdb` (data disk). Reboot and open the firmware boot menu (F12 on this board). Choose the USB entry that does not say UEFI. If the stick is missing, enable CSM in setup. The Limine menu counts down, then the screen is blue with `hello lerux` on the first line and `lerux>` under it. A PS/2 keyboard in the rear combo port types at that prompt (`echo`, `help`, `pwd`, `clear`; other commands print `unavailable` until a disk driver exists). COM1 at 115200 8N1 (the COMA header; there is no rear DB9) prints `lerux-shell: prompt`. The UEFI entry stops in Limine: this kernel is linked at 1MB. Reboot and choose Ubuntu to return.

Reboot once into the stick from Ubuntu, without the F12 menu. The stick has to be plugged in so the entry exists:

```bash
efibootmgr  # list EFI entries; Boot0018 is Ubuntu (SHIMX64.EFI, the default), Boot0022 is the legacy USB stick
sudo efibootmgr -n 0022  # set BootNext to that legacy entry for one boot; BootOrder is unchanged, so the boot after that is Ubuntu again
sudo systemctl reboot  # reboot now into BootNext
```

`Boot0022` is the `Generic Flash Disk` line with no `UEFI:` prefix. `Boot0021` (`UEFI: Generic Flash Disk`) stops in Limine. If the numbers move, match the label from `efibootmgr` before using `-n`.

**Try it in QEMU:** `just qemu` (ARM virt) or `just qemu-x86-64` (x86-64 q35) boots the workstation with a serial shell at `lerux>`. Quit with `Ctrl-A x`. `just qemu-interactive` opens the interactive workstation in a QEMU window (close the window to quit).

Rust userspace on the [seL4](https://sel4.systems/) microkernel, using [seL4 Microkit](https://github.com/seL4/microkit) for static system layout and [rust-sel4](https://github.com/seL4/rust-sel4) for userspace APIs.

The seL4 kernel is **not vendored** — it is cloned into `deps/workspace/` and built from source via the Microkit SDK. All lerux-owned code is Rust protection domains and build orchestration.

## Quick start (QEMU)

**Prerequisites:** Linux, `git`, `just`, `rustup`, `cmake`, `ninja`, `qemu-system-aarch64`, `libclang-dev` (for `bindgen` when building PDs), and optionally the [ARM GNU bare-metal toolchain](https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads) (`aarch64-none-elf-gcc`, 12.2.Rel1) for `just build-sdk`. Python 3 is only required for `just build-sdk` (upstream Microkit `build_sdk.py`).

```bash
just fetch          # clone seL4 15.0.0 + microkit 2.2.0
just build-sdk      # build Microkit SDK from source (auto-downloads ARM toolchain if needed)
# or: just fetch-sdk   # download prebuilt SDK 2.2.0 (no compile step)
just qemu           # aarch64 workstation in QEMU (serial shell; Ctrl-A x to quit)
just qemu-x86-64    # x86-64 workstation in QEMU (same; alias: just qemu-x86)
just qemu-interactive  # interactive workstation in a QEMU window (close window to quit)
just run            # hello PD only (override with BOARD=… just run)
```

Smoke test:

```bash
just test
```

Full local CI mirror (SDK must include aarch64, x86_64, and RISC-V boards):

```bash
MICROKIT_BOARDS=qemu_virt_aarch64,x86_64_generic,qemu_virt_riscv64,rpi4b_4gb just build-sdk  # include hardware boards as needed
just test-all
```

## CI

GitHub Actions runs on every push to `main`: **check** (`just check`), one **sdk** job (SDK + patched SP804 QEMU), **check-pd** (cross-target clippy on userspace crates), then **43 smoke** matrix jobs (`just test-all` runs every `ci = true` board). Local lint: `just check` (host crates) or `just check-all` (host + PD, needs SDK). Details: [`docs/ci.md`](docs/ci.md).

## Architecture

| Layer | Source |
|-------|--------|
| Kernel | [seL4/seL4](https://github.com/seL4/seL4) — built by `build_sdk.py` |
| System framework | [seL4/microkit](https://github.com/seL4/microkit) SDK |
| Userspace | Rust protection domains in `userspace/pds/` via `sel4-microkit` |
| Utilities | Shared crates in `userspace/crates/` (`lerux-logging`, `lerux-ipc`, `lerux-driver-protocols`) |
| Serial console | Driver PD + IPC client PDs — PL011 (aarch64), NS16550 MMIO (riscv64), or NS16550/COM1 (x86) |

Version pins: [`deps/versions.toml`](deps/versions.toml).

## Boards

Default: `qemu_virt_aarch64` (QEMU ARM virt). Override with `BOARD=... just run`.

| Goal | Board | Command |
|------|-------|---------|
| Serial hello | `qemu_virt_aarch64` | `just test` |
| Echo IPC | `qemu_virt_aarch64_echo` | `just test-echo` |
| Display (ramfb) | `qemu_virt_aarch64_display` | `just test-display` |
| HTML subset parse | `qemu_virt_aarch64_html` | `just test-html` |
| HTML+CSS paint | `qemu_virt_aarch64_paint` | `just test-paint` |
| Browser (one tab) | `qemu_virt_aarch64_browser` | `just test-browser` |
| Agent runtime | `qemu_virt_aarch64_agent_runtime` | `just test-agent-runtime` |
| Agent tools | `qemu_virt_aarch64_agent` | `just test-agent` |
| Interactive (joint) | `qemu_virt_aarch64_interactive` | `just test-interactive` |
| Virtio blk/net | `qemu_virt_aarch64_virtio` | `just disk-img && just test-virtio` |
| RTC + timer (all arches) | `*_init` | `just test-init` / `test-init-riscv` / `test-init-x86` |
| Init + virtio | `qemu_virt_aarch64_composed` | `just disk-img && just test-composed` |
| HTTP over virtio-net | `qemu_virt_aarch64_http` | `just test-http` |
| x86 HTTP over virtio-net | `x86_64_generic_http` | `just test-x86-http` |
| Init + HTTP | `qemu_virt_aarch64_http_composed` | `just test-http-composed` |
| x86 serial / echo / virtio | `x86_64_generic` variants | `BOARD=x86_64_generic just test` / `just test-x86-echo` / `just disk-img && just test-x86-virtio` |
| Block IPC over virtio-blk | `qemu_virt_aarch64_blk` variants | `just test-blk` / `just test-riscv-blk` / `just test-x86-blk` |
| Net IPC over virtio-net | `qemu_virt_aarch64_net` variants | `just test-net` / `just test-riscv-net` / `just test-x86-net` |
| HTTP fetch over net IPC | `qemu_virt_aarch64_fetch` | `just test-fetch` |
| HTTPS fetch over tls-proxy | `qemu_virt_aarch64_fetch_tls` | `just test-fetch-tls` |
| HTTPS via request-server | `qemu_virt_aarch64_request` | `just test-request` |
| Signed Wasm program | `qemu_virt_aarch64_program` | `just test-program` |
| Init + net IPC | `qemu_virt_aarch64_net_composed` | `just test-net-composed` |
| Init + blk/net IPC | `qemu_virt_aarch64_ipc_composed` | `just test-ipc-composed` |
| RISC-V serial / echo / virtio / HTTP | `qemu_virt_riscv64` variants | `just test-riscv` / `just test-riscv-echo` / `just test-riscv-virtio` / `just test-riscv-http` |
| System profiles (workstation etc) | `lerux profile` | `cargo run -p lerux-cli -- profile list` / `profile build workstation` |
| Real hardware (RPi4 serial slice) | `rpi4b_4gb` | `BOARD=rpi4b_4gb just image` (or `just test` for build verification; see docs for U-Boot deploy) |
| Real hardware (RPi4 workstation) | `rpi4b_4gb_workstation` | `just deploy-rpi4` / `just test-hw` — [install path](docs/boards.md#rpi4-workstation-install-path-phase-52) |
| USB ISO (Gigabyte Z97-D3H shell) | `pc_z97_d3h` | `just iso` — [install path](docs/boards.md#gigabyte-z97-d3h-install-path) |
| Real hardware (Gigabyte Z97-D3H shell) | `pc_z97_d3h` | `just deploy-pc` / `just test-hw` — [install path](docs/boards.md#gigabyte-z97-d3h-install-path) |
| x86 on-screen shell (QEMU) | `x86_64_generic_console` | `just test-x86-console` |

Full board reference: [`docs/boards.md`](docs/boards.md).

**aarch64 init and composed** need patched QEMU for SP804 at `0x90d0000` — run `cargo run -p lerux-cli -- install sp804-qemu` (Docker image includes build deps). RISC-V/x86 init use stock QEMU (Goldfish RTC + `rdtime`; CMOS RTC + TSC). See Phase 12 in [`docs/plan.md`](docs/plan.md).

## Documentation

| Doc | Purpose |
|-----|---------|
| [AGENTS.md](AGENTS.md) | LLM agent instructions for idiomatic Rust |
| [docs/README.md](docs/README.md) | Documentation index |
| [docs/context.md](docs/context.md) | Domain language and decisions |
| [docs/plan.md](docs/plan.md) | Roadmap and smoke parity table (phases 1–82, done) |
| [docs/plan-arch.md](docs/plan-arch.md) | Arch-level functionality gap plan (phases 50–60, done; physical lab still open) |
| [docs/plan-qemu.md](docs/plan-qemu.md) | QEMU-only workstation deepening (phases 61–70, done) |
| [docs/plan-interactive.md](docs/plan-interactive.md) | Interactive surface (phases 71–80, done: browser + agent) |
| [docs/boards.md](docs/boards.md) | Board and QEMU profile reference |
| [docs/ci.md](docs/ci.md) | CI pipeline, caches, troubleshooting |
| [docs/seL4-whitepaper.pdf](docs/seL4-whitepaper.pdf) | seL4 overview (reference) |
| [docs.sel4.systems](https://docs.sel4.systems/) | Official tutorials and manuals |

## License

MIT for lerux-owned code. seL4 kernel is GPL-2.0-only; rust-sel4 crates are BSD-2-Clause.