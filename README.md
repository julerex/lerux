# lerux

Rust userspace on the [seL4](https://sel4.systems/) microkernel, using [seL4 Microkit](https://github.com/seL4/microkit) for static system layout and [rust-sel4](https://github.com/seL4/rust-sel4) for userspace APIs.

The seL4 kernel is **not vendored** — it is cloned into `deps/workspace/` and built from source via the Microkit SDK build. All lerux-owned code is Rust protection domains and build orchestration.

## Quick start

**Prerequisites:** Linux, `git`, `just`, `rustup`, `python3`, `cmake`, `ninja`, `qemu-system-aarch64`, `libclang-dev` (for `bindgen` when building PDs), and optionally the [ARM GNU bare-metal toolchain](https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads) (`aarch64-none-elf-gcc`, 12.2.Rel1) for `just build-sdk`.

```bash
just fetch          # clone seL4 15.0.0 + microkit 2.2.0
just build-sdk      # build Microkit SDK from source (needs aarch64-none-elf-gcc)
# or: just fetch-sdk   # download prebuilt SDK 2.2.0 if the toolchain is not installed
just run            # build hello PD, assemble loader.img, boot QEMU
```

Smoke test:

```bash
pip install pexpect   # if needed
just test
```

## Architecture

| Layer | Source |
|-------|--------|
| Kernel | [seL4/seL4](https://github.com/seL4/seL4) — built by `build_sdk.py` |
| System framework | [seL4/microkit](https://github.com/seL4/microkit) SDK |
| Userspace | Rust crates in `userspace/pds/` via `sel4-microkit` |

Version pins: [`deps/versions.toml`](deps/versions.toml).

## Boards

Default: `qemu_virt_aarch64` (QEMU ARM virt). Override with `BOARD=... just run`.

x86_64 PC99/QEMU support is planned — the build is parameterized by `BOARD`; see [`docs/plan.md`](docs/plan.md).

## Documentation

- [docs/README.md](docs/README.md) — index
- [docs/seL4-whitepaper.pdf](docs/seL4-whitepaper.pdf) — seL4 overview
- [docs.sel4.systems](https://docs.sel4.systems/) — official tutorials and manuals

## License

MIT for lerux-owned code. seL4 kernel is GPL-2.0-only; rust-sel4 crates are BSD-2-Clause.