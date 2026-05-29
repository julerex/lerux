# lerux

**Only Rust Redox**

A pure-Rust take on the Redox operating system, starting with a vendored and adapted version of the Redox microkernel.

## Current Status (Phase 1)

- Kernel source vendored under `kernel/` (copy of redox-os/kernel as of 2026-05).
- First "Only Rust" milestone achieved: the last external assembler (nasm) dependency for SMP AP trampolines has been removed.
  - Trampolines are now plain `&[u8]` data (see `kernel/src/arch/x86_shared/trampoline.rs`).
  - `build.rs` no longer invokes nasm; the unused `cc` build-dep was also dropped.
- The kernel remains a drop-in buildable Redox kernel with all its existing features and multi-architecture support.

## Goals

- Eliminate non-Rust code wherever practical (assembly, build-time codegen in other languages, etc.).
- Keep the excellent Redox kernel design while making the implementation "only Rust".
- Long-term: a complete, bootable, multi-user Redox-like system built from this foundation.

## Building

See [kernel/README.md](kernel/README.md) for kernel build instructions. The nasm requirement listed in the upstream docs no longer applies to lerux.

## QEMU Bring-up

See [qemu/README.md](qemu/README.md) for how to boot the kernel under QEMU for development and smoke testing. A minimal loader + launch script is provided so you can iterate quickly without the full Redox build system.

## License

MIT (same as upstream Redox components we incorporate).

## Contributing

This is early-stage personal research. Issues and PRs discussing "Only Rust" refactoring approaches are welcome.
