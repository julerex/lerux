# lerux Documentation

This directory is the canonical home for all written (markdown) documentation for the lerux project.

See the [root README](../README.md) for a high-level project overview.

## Organization

- [plan.md](plan.md) — Living roadmap, phases, Only Rust policy, vendoring strategy, and the rustc-hosting milestone.
- [context.md](context.md) — Domain language, core concepts, resolved decisions (AI role, runtime trade-offs, rustc goal definition, etc.).
- [notes.md](notes.md) — Verified bring-up facts, GDB notes, serial output, and debugging history for direct-boot.
- [vendored.md](vendored.md) — Full inventory of vendored components, divergence from upstream Redox, sync procedure.
- [glossary.md](GLOSSARY.md) — Terminology (boot, validation, Redox vs. lerux names).
- `building/`
  - [standalone.md](building/standalone.md) — How to build and run with `just` + direct-boot (no full Redox system required).
- `development/`
  - [coverage.md](development/coverage.md) — 100% unit test coverage goal (excl. redoxfs), tooling (`just coverage`), and the small approved list of exceptions.
  - [qemu.md](development/qemu.md) — (Consolidated from the old qemu/ tree README) QEMU harness, loader details, boot handoff.
  - [trampolines.md](development/trampolines.md) — (Consolidated) SMP trampoline validation story, golden files, and the NASM removal.
  - [redoxfs-unsafe-audit.md](development/redoxfs-unsafe-audit.md) — Post-smoke AI-assisted unsafe review of the vendored filesystem (kept here for reference).
- `kernel/`
  - [kernel/architecture.md](kernel/architecture.md) — Beginner-friendly, end-to-end tour of the kernel (boot, memory, scheduling, syscalls, schemes, interrupts, SMP), linking each concept to the annotated source. Start here for the kernel internals.
  - [kernel/README.md](kernel/README.md) — Kernel directory overview, build, and debugging (GDB/LLDB) notes.
  - [kernel/rmm.md](kernel/rmm.md) — Primer on the physical memory manager (RMM) and how the memory layers fit together.
  - Other kernel-specific notes (ARM port outline, etc.).
- Other historical or component READMEs have been folded into the above or left as thin pointers in their original locations with "see docs/" notes.

## Building the docs

- Markdown docs: just read the files (or use any md viewer).
- Rust API docs: `just docs` (or `cargo doc --document-private-items ...` for the scoped crates). This benefits from the comprehensive docstrings added as part of the documentation cleanup.

## Contributing to docs

When you add a major new subsystem, prefer adding (or updating) a focused markdown file under the appropriate `development/` or `kernel/` subdirectory and link it from this index, `plan.md`, `context.md`, and `glossary.md`.

Cross-links should use relative paths from within `docs/`.

## See also (for developers)

- The `just` recipes `test`, `coverage`, `docs`, `check-only-rust`, `validate-trampolines`.
- CI jobs in `.github/workflows/rust.yml` (now includes a `unit-tests` job that exercises host coverage collection).

This structure was introduced as part of the 2026-06 documentation cleanup (every function docstring + 100% unit tests excl. redoxfs + centralize all .md under docs/).
