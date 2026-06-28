# lerux

Rust userspace on the [seL4](https://sel4.systems/) microkernel, using [seL4 Microkit](https://github.com/seL4/microkit) for static system layout and [rust-sel4](https://github.com/seL4/rust-sel4) for userspace APIs.

The seL4 kernel is **not vendored** — it is cloned into `deps/workspace/` and built from source via the Microkit SDK build. All lerux-owned code is Rust protection domains and build orchestration.

## Quick start

**Prerequisites:** Linux, `git`, `just`, `rustup`, `python3`, `cmake`, `ninja`, `qemu-system-aarch64`, `libclang-dev` (for `bindgen` when building PDs), and optionally the [ARM GNU bare-metal toolchain](https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads) (`aarch64-none-elf-gcc`, 12.2.Rel1) for `just build-sdk`.

```bash
just fetch          # clone seL4 15.0.0 + microkit 2.2.0
just build-sdk      # build Microkit SDK from source (auto-downloads ARM toolchain if needed)
# or: just fetch-sdk   # download prebuilt SDK 2.2.0 (no compile step)
# MICROKIT_BOARDS=qemu_virt_aarch64,qemu_x86_64 just build-sdk  # add boards
just run            # build hello PD, assemble loader.img, boot QEMU
```

Smoke test:

```bash
pip install pexpect   # if needed
just test
```

All CI smoke tests locally (SDK must include `qemu_virt_aarch64`, `x86_64_generic`, and `qemu_virt_riscv64`):

```bash
MICROKIT_BOARDS=qemu_virt_aarch64,x86_64_generic,qemu_virt_riscv64 just build-sdk
just test-all
```

## CI

GitHub Actions (`.github/workflows/rust.yml`) on every push to `main`:

1. **sdk** — build Docker image, fetch sources, build Microkit SDK once for both boards (cached)
2. **smoke** (matrix) — `just test`, `BOARD=x86_64_generic just test`, `just test-riscv`, `just test-virtio`

## Architecture

| Layer | Source |
|-------|--------|
| Kernel | [seL4/seL4](https://github.com/seL4/seL4) — built by `build_sdk.py` |
| System framework | [seL4/microkit](https://github.com/seL4/microkit) SDK |
| Userspace | Rust protection domains in `userspace/pds/` via `sel4-microkit` |
| Utilities | Shared crates in `userspace/crates/` (`lerux-logging`, `lerux-ipc`, `lerux-sync`) |
| Serial console | Driver PD + IPC client PDs — PL011 (aarch64), NS16550 MMIO (riscv64), or NS16550/COM1 (x86) |

Version pins: [`deps/versions.toml`](deps/versions.toml).

## Boards

Default: `qemu_virt_aarch64` (QEMU ARM virt). Override with `BOARD=... just run`.

x86_64 QEMU PC (`x86_64_generic`):

```bash
MICROKIT_BOARDS=qemu_virt_aarch64,x86_64_generic just build-sdk
BOARD=x86_64_generic just run
```

RISC-V QEMU virt (`qemu_virt_riscv64`):

```bash
MICROKIT_BOARDS=qemu_virt_aarch64,x86_64_generic,qemu_virt_riscv64 just build-sdk
BOARD=qemu_virt_riscv64 just run
# or: just test-riscv
```

Virtio block + net drivers on aarch64 virt (`qemu_virt_aarch64_virtio`):

```bash
just disk-img          # 4 MiB empty disk for virtio-blk
just test-virtio       # serial + virtio-blk read + virtio-net TX smoke test
```

## Documentation

- [docs/README.md](docs/README.md) — index
- [docs/seL4-whitepaper.pdf](docs/seL4-whitepaper.pdf) — seL4 overview
- [docs.sel4.systems](https://docs.sel4.systems/) — official tutorials and manuals

## License

MIT for lerux-owned code. seL4 kernel is GPL-2.0-only; rust-sel4 crates are BSD-2-Clause.