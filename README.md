# lerux

## Overview

Rust userspace on the [seL4](https://sel4.systems/) microkernel, using [seL4 Microkit](https://github.com/seL4/microkit) for static system layout and [rust-sel4](https://github.com/seL4/rust-sel4) for userspace APIs.

The seL4 kernel is **not vendored** — it is cloned into `deps/workspace/` and built from source via the Microkit SDK. All lerux-owned code is Rust protection domains and build orchestration.

## Quickstart

Build the desk image as a UEFI USB ISO, write it to a flash drive, and boot that drive with **UEFI**. The documented metal board is `pc_z97_d3h` (Gigabyte GA-Z97-D3H). Full procedure: [`docs/boards.md`](docs/boards.md#gigabyte-z97-d3h-install-path).

**Prerequisites:** Linux, `git`, `just`, `rustup`, `cmake`, `ninja`, `libclang-dev`, `clang` (for the Multiboot 2 trampoline), and a USB flash drive. Do not overwrite the host OS disk.

```bash
just fetch          # clone seL4 + microkit into deps/workspace/
just fetch-sdk      # download prebuilt Microkit SDK 2.3.0
# or: just build-sdk   # build the SDK from source instead
just iso pc_z97_d3h  # pack build/pc_z97_d3h/lerux_pc_z97_d3h.iso (`just iso` packs build/swift_sfx14_41g/lerux_swift_sfx14_41g.iso)
lsblk               # find the flash drive (e.g. /dev/sdX) — not the OS disk
sudo dd if=build/pc_z97_d3h/lerux_pc_z97_d3h.iso of=/dev/sdX bs=4M status=progress conv=fsync
```

Reboot and open the firmware boot menu (often F12). Choose the USB entry that says **UEFI** (GRUB `EFI/BOOT/BOOTX64.EFI`). GRUB reads `/boot/grub/grub.cfg` and boots immediately, then the screen is blue with `hello lerux` and the menu `1 browser` / `2 maths`. Type with a PS/2 keyboard in the rear combo port, or a USB HID boot keyboard on a root xHCI port.

The metal kernel is 64-bit `sel4.elf`, still linked at 1 MiB. GRUB Multiboot 2-loads `/boot/mb2-stub.elf` with modules `/boot/sel4.elf` and `/boot/loader.img`. The stub places the kernel at 1 MiB after `ExitBootServices`.

## Quick start (QEMU)

**Prerequisites:** the same host tools as above, plus `qemu-system-aarch64`. x86 QEMU smokes also need `ovmf`, `xorriso`, and `grub-efi-amd64-bin` (Arch: `grub`). Optionally the [ARM GNU bare-metal toolchain](https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads) (`aarch64-none-elf-gcc`, 12.2.Rel1) for `just build-sdk`. Python 3 is only required for `just build-sdk` (upstream Microkit `build_sdk.py`).

```bash
just fetch          # clone seL4 16.0.0 + microkit 2.3.0
just build-sdk      # build Microkit SDK from source (auto-downloads ARM toolchain if needed)
# or: just fetch-sdk   # download prebuilt SDK 2.3.0 (no compile step)
just qemu           # browser and maths menu in QEMU (Ctrl-A x to quit)
just qemu-x86-64    # same shell on the x86-64 filesystem image
just run            # hello PD only (override with BOARD=… just run)
```

Smoke test:

```bash
just test
```

`just test` boots the browser image on QEMU ARM virt and waits at a menu, `1 browser` or `2 maths`. `just qemu` leaves that menu running. Quit with `Ctrl-A x`. The desk image `pc_z97_d3h` shows the same menu on the VGA text page.

Full local CI mirror (SDK must include aarch64, x86_64, and RISC-V boards):

```bash
MICROKIT_BOARDS=qemu_virt_aarch64,x86_64_generic,qemu_virt_riscv64,rpi4b_4gb just build-sdk  # include hardware boards as needed
just test-all
```

## CI

GitHub Actions runs on every push to `main`: **check** (`just check`), **verify-ring** (`just verify-ring`, in parallel with check), one **sdk** job (SDK + patched SP804 QEMU), **check-pd** (cross-target clippy on userspace crates), then **20 smoke** matrix jobs (the workflow `include` list). `just test-all` runs every `ci = true` board, a wider set than that list. Local lint: `just check` (host crates) or `just check-all` (host + PD, needs SDK). Details: [`docs/ci.md`](docs/ci.md).

## Architecture

| Layer | Source |
|-------|--------|
| Kernel | [seL4/seL4](https://github.com/seL4/seL4) — built by `build_sdk.py` |
| System framework | [seL4/microkit](https://github.com/seL4/microkit) SDK |
| Userspace | Rust protection domains in `userspace/pds/` via `sel4-microkit` |
| Utilities | Shared crates in `userspace/crates/` (`lerux-logging`, `lerux-sddf`, `lerux-driver-protocols`) |
| Serial console | Driver PD + IPC client PDs — PL011 (aarch64), NS16550 MMIO (riscv64), or NS16550/COM1 (x86) |

Version pins: [`deps/versions.toml`](deps/versions.toml).

## Boards

Default board for `just build` and `just run` is `qemu_virt_aarch64` (QEMU ARM virt). Override with `BOARD=...`. Bare `just test` boots `qemu_virt_aarch64_browser_sddf`.

| Goal | Board | Command |
|------|-------|---------|
| Serial and filesystem shell | `qemu_virt_aarch64_fs_sddf` | `just test-fs-sddf` |
| Serial hello | `qemu_virt_aarch64` | `BOARD=qemu_virt_aarch64 just test` |
| HTML subset parse | `qemu_virt_aarch64_html` | `just test-html` |
| Browser and maths menu | `qemu_virt_aarch64_browser_sddf` | `just test` |
| Agent tools | `qemu_virt_aarch64_agent_sddf` | `just test-agent-sddf` |
| Virtio blk/net | `qemu_virt_aarch64_virtio` | `just disk-img && just test-virtio` |
| HTTP over virtio-net | `qemu_virt_aarch64_http` | `just test-http` |
| x86 HTTP over virtio-net | `x86_64_generic_http` | `just test-x86-http` |
| x86 serial / virtio | `x86_64_generic` variants | `BOARD=x86_64_generic just test` / `just disk-img && just test-x86-virtio` |
| LionsOS WebAssembly runtime | `qemu_virt_aarch64_program_sddf` | `just test-program-sddf` |
| RISC-V serial / virtio / HTTP | `qemu_virt_riscv64` variants | `just test-riscv` / `just test-riscv-virtio` / `just test-riscv-http` |
| System profiles | `lerux profile` | `cargo run -p lerux-cli -- profile list` |
| Real hardware (RPi4 hello) | `rpi4b_4gb` | `BOARD=rpi4b_4gb just image` |

Full board reference: [`docs/boards.md`](docs/boards.md).

**aarch64 init and composed** need patched QEMU for SP804 at `0x90d0000` — run `cargo run -p lerux-cli -- install sp804-qemu` (Docker image includes build deps). RISC-V/x86 init use stock QEMU (Goldfish RTC + `rdtime`; CMOS RTC + TSC). See Phase 12 in [`docs/plan.md`](docs/plan.md).

## Documentation

| Doc | Purpose |
|-----|---------|
| [AGENTS.md](AGENTS.md) | LLM agent instructions for idiomatic Rust |
| [docs/README.md](docs/README.md) | Documentation index |
| [docs/context.md](docs/context.md) | Domain language and decisions |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Dependency hierarchy and call graph across lerux, rust-sel4, Microkit, and seL4 |
| [docs/plan.md](docs/plan.md) | Roadmap and smoke parity table (phases 1–82, done) |
| [docs/plan-arch.md](docs/plan-arch.md) | Arch-level functionality gap plan (phases 50–60, done; physical lab still open) |
| [docs/plan-qemu.md](docs/plan-qemu.md) | QEMU-only workstation deepening (phases 61–70, done) |
| [docs/plan-interactive.md](docs/plan-interactive.md) | Interactive surface (phases 71–80 record; Milestone 17 deleted those protection domains) |
| [docs/boards.md](docs/boards.md) | Board and QEMU profile reference |
| [docs/ci.md](docs/ci.md) | CI pipeline, caches, troubleshooting |
| [docs/seL4-whitepaper.pdf](docs/seL4-whitepaper.pdf) | seL4 overview (reference) |
| [docs.sel4.systems](https://docs.sel4.systems/) | Official tutorials and manuals |

## License

MIT for lerux-owned code. seL4 kernel is GPL-2.0-only; rust-sel4 crates are BSD-2-Clause.
