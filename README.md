# lerux

**Only Rust Redox**

A pure-Rust take on the Redox operating system, starting with a vendored and adapted version of the Redox microkernel.

## Relationship to upstream Redox

The tree under `kernel/` is a **vendored copy** of [redox-os/kernel](https://gitlab.redox-os.org/redox-os/kernel) (~2026-05). Most kernel logic, syscalls, and schemes are unchanged Redox code. lerux-specific work lives at the **repo root** (build, QEMU harness, direct-boot) and in a **small set of kernel patches** (embedded trampolines, pure-Rust PVH stub, `direct-boot` boot path).

See **[vendored.md](vendored.md)** for the full divergence list: what changed, what stayed the same, and what is still planned.

## Current Status (Phase 1)

- Kernel source vendored under `kernel/` (copy of redox-os/kernel as of 2026-05).
- "Only Rust" milestones achieved: the external assembler/compiler build dependencies have been removed.
  - SMP AP trampolines are now plain `&[u8]` data (see `kernel/src/arch/x86_shared/trampoline.rs`) — no `nasm`.
  - The direct-boot PVH boot stub is now pure Rust via `core::arch::global_asm!` (see `kernel/src/arch/x86_shared/pvh_boot.rs`); the `cc`/`clang` build dependency it required has been dropped from `build.rs` and `Cargo.toml`.
- Direct-boot (`just qemu-direct`) boots through early bring-up to the idle loop with no C toolchain required (see [docs/building/standalone.md](docs/building/standalone.md)).
- The kernel remains a drop-in buildable Redox kernel with all its existing features and multi-architecture support.

## Goals

- Eliminate non-Rust code wherever practical (assembly, build-time codegen in other languages, etc.).
- Keep the excellent Redox kernel design while making the implementation "only Rust".
- Long-term: a complete, bootable, multi-user Redox-like system built from this foundation.

## Building

- **Standalone / direct-boot (lerux):** [docs/building/standalone.md](docs/building/standalone.md) — `just build-direct`, no Redox image or C toolchain.
- **Full Redox-style build:** [docs/kernel/README.md](docs/kernel/README.md) — still requires the Redox build system when not using `direct-boot`. The nasm requirement listed in upstream docs **does not apply** to lerux kernel builds.

## QEMU Bring-up

See [docs/development/qemu.md](docs/development/qemu.md) for how to boot the kernel under QEMU for development and smoke testing. A minimal loader + launch script is provided so you can iterate quickly without the full Redox build system.

## Documentation

All significant project documentation now lives under [docs/](docs/README.md).

| Doc | Purpose |
|-----|---------|
| [docs/README.md](docs/README.md) | Index + organization of the docs tree |
| [docs/glossary.md](docs/GLOSSARY.md) | Terms and concepts (boot, validation, Redox, lerux-specific names) |
| [docs/plan.md](docs/plan.md) | Roadmap, Only Rust policy, phases, and open questions |
| [docs/context.md](docs/context.md) | Domain language and resolved decisions (including the rustc-hosting goal) |
| [docs/vendored.md](docs/vendored.md) | Vendoring policy, upstream inventory, kernel divergence, and sync procedure |
| [docs/notes.md](docs/notes.md) | Verified direct-boot facts, serial logs, and GDB recipes |
| [docs/building/standalone.md](docs/building/standalone.md) | Direct-boot build and run instructions |
| [docs/development/coverage.md](docs/development/coverage.md) | 100% unit test coverage goal, tooling, and approved exceptions (excl. redoxfs) |

Upstream snapshots and sync policy: see [docs/vendored.md](docs/vendored.md). Lerux does not depend on live Redox GitLab repos at build time.

## License

MIT (same as upstream Redox components we incorporate). See [docs/vendored.md](docs/vendored.md) for per-component attribution.

## Contributing

This is early-stage personal research. Issues and PRs discussing "Only Rust" refactoring approaches are welcome.
