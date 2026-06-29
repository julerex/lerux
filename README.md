# lerux

Rust userspace on the [seL4](https://sel4.systems/) microkernel, using [seL4 Microkit](https://github.com/seL4/microkit) for static system layout and [rust-sel4](https://github.com/seL4/rust-sel4) for userspace APIs.

The seL4 kernel is **not vendored** — it is cloned into `deps/workspace/` and built from source via the Microkit SDK. All lerux-owned code is Rust protection domains and build orchestration.

## Quick start

**Prerequisites:** Linux, `git`, `just`, `rustup`, `cmake`, `ninja`, `qemu-system-aarch64`, `libclang-dev` (for `bindgen` when building PDs), and optionally the [ARM GNU bare-metal toolchain](https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads) (`aarch64-none-elf-gcc`, 12.2.Rel1) for `just build-sdk`. Python 3 is only required for `just build-sdk` (upstream Microkit `build_sdk.py`).

```bash
just fetch          # clone seL4 15.0.0 + microkit 2.2.0
just build-sdk      # build Microkit SDK from source (auto-downloads ARM toolchain if needed)
# or: just fetch-sdk   # download prebuilt SDK 2.2.0 (no compile step)
just run            # build hello PD, assemble loader.img, boot QEMU
```

Smoke test:

```bash
just test
```

Full local CI mirror (SDK must include aarch64, x86_64, and RISC-V boards):

```bash
MICROKIT_BOARDS=qemu_virt_aarch64,x86_64_generic,qemu_virt_riscv64 just build-sdk
just test-all
```

## CI

GitHub Actions runs on every push to `main`: **check** (`just check`), one **sdk** job (SDK + patched SP804 QEMU), then **15 smoke** matrix jobs. Local lint: `just check`. Details: [`docs/ci.md`](docs/ci.md).

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

| Goal | Board | Command |
|------|-------|---------|
| Serial hello | `qemu_virt_aarch64` | `just test` |
| Echo IPC | `qemu_virt_aarch64_echo` | `just test-echo` |
| Virtio blk/net | `qemu_virt_aarch64_virtio` | `just disk-img && just test-virtio` |
| RTC + timer | `qemu_virt_aarch64_init` | `just test-init` |
| Init + virtio | `qemu_virt_aarch64_composed` | `just disk-img && just test-composed` |
| HTTP over virtio-net | `qemu_virt_aarch64_http` | `just test-http` |
| x86 HTTP over virtio-net | `x86_64_generic_http` | `just test-x86-http` |
| Init + HTTP | `qemu_virt_aarch64_http_composed` | `just test-http-composed` |
| x86 serial / echo / virtio | `x86_64_generic` variants | `BOARD=x86_64_generic just test` / `just test-x86-echo` / `just disk-img && just test-x86-virtio` |
| RISC-V serial / echo / virtio / HTTP | `qemu_virt_riscv64` variants | `just test-riscv` / `just test-riscv-echo` / `just test-riscv-virtio` / `just test-riscv-http` |

Full board reference: [`docs/boards.md`](docs/boards.md).

**Init and composed** need patched QEMU for SP804 at `0x90d0000` — run `cargo run -p lerux-cli -- install sp804-qemu` (Docker image includes build deps). Init is **aarch64 virt only**; cross-arch parity is in [`docs/plan.md`](docs/plan.md).

## Documentation

| Doc | Purpose |
|-----|---------|
| [AGENTS.md](AGENTS.md) | LLM agent instructions for idiomatic Rust |
| [docs/README.md](docs/README.md) | Documentation index |
| [docs/context.md](docs/context.md) | Domain language and decisions |
| [docs/plan.md](docs/plan.md) | Roadmap and smoke parity table |
| [docs/boards.md](docs/boards.md) | Board and QEMU profile reference |
| [docs/ci.md](docs/ci.md) | CI pipeline, caches, troubleshooting |
| [docs/seL4-whitepaper.pdf](docs/seL4-whitepaper.pdf) | seL4 overview (reference) |
| [docs.sel4.systems](https://docs.sel4.systems/) | Official tutorials and manuals |

## License

MIT for lerux-owned code. seL4 kernel is GPL-2.0-only; rust-sel4 crates are BSD-2-Clause.