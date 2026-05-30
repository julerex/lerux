# lerux

**Only Rust Redox**

A pure-Rust take on the Redox operating system, starting with a vendored and adapted version of the Redox microkernel.

## Current Status (Phase 1)

- Kernel source vendored under `kernel/` (copy of redox-os/kernel as of 2026-05).
- "Only Rust" milestones achieved: the external assembler/compiler build dependencies have been removed.
  - SMP AP trampolines are now plain `&[u8]` data (see `kernel/src/arch/x86_shared/trampoline.rs`) — no `nasm`.
  - The direct-boot PVH boot stub is now pure Rust via `core::arch::global_asm!` (see `kernel/src/arch/x86_shared/pvh_boot.rs`); the `cc`/`clang` build dependency it required has been dropped from `build.rs` and `Cargo.toml`.
- Direct-boot (`just qemu-direct`) boots through early bring-up to the idle loop with no C toolchain required (see `BUILDING-standalone.md`).
- The kernel remains a drop-in buildable Redox kernel with all its existing features and multi-architecture support.

## Goals

- Eliminate non-Rust code wherever practical (assembly, build-time codegen in other languages, etc.).
- Keep the excellent Redox kernel design while making the implementation "only Rust".
- Long-term: a complete, bootable, multi-user Redox-like system built from this foundation.

## Building

See [kernel/README.md](kernel/README.md) for kernel build instructions. The nasm requirement listed in the upstream docs no longer applies to lerux.

## QEMU Bring-up

See [qemu/README.md](qemu/README.md) for how to boot the kernel under QEMU for development and smoke testing. A minimal loader + launch script is provided so you can iterate quickly without the full Redox build system.

## Vendored code

Upstream snapshots and sync policy are documented in [VENDORED.md](VENDORED.md). Lerux does not depend on live Redox GitLab repos at build time.

## License

MIT (same as upstream Redox components we incorporate). See [VENDORED.md](VENDORED.md) for per-component attribution.

## Contributing

This is early-stage personal research. Issues and PRs discussing "Only Rust" refactoring approaches are welcome.
