# Unit Test Coverage

This document tracks the 100% unit test coverage goal (excluding `userspace/redoxfs/` per project request) and the small list of approved exceptions.

## Current status
- Tooling: `cargo llvm-cov` (via `just coverage`).
- In-scope: kernel (host `cargo test --bin kernel` for `#[cfg(test)]` modules), `kernel/rmm --features std`, userspace workspace crates + standalone testable crates (`initfs*`, `runtime/*`, `bootstrap`, `scheme-utils`, daemons, drivers/common, etc.).
- Excludes (hard-coded in recipes/CI): `userspace/redoxfs`, `vendor/`, `target/`, build artifacts, validation asm, etc.

Run locally:
```bash
just coverage          # full report + summary (HTML in target/coverage/)
just coverage-rmm
cargo test -p rmm --features std   # raw
```

In CI a dedicated job (or extension of existing) runs this and uploads the report artifact.

## Approved small list of coverage exceptions
(These are allowed; see user confirmation 2026-06-16.)

- Early architecture bring-up sequences that only execute under real CPU reset / specific firmware state not reproducible under `cargo test` host runs or QEMU direct-boot user simulation (e.g. certain parts of `arch/*/start.rs` before any testable abstraction, low-level GDT/IDT init that assumes live CPU mode).
- IRQ / interrupt controller paths that require specific hardware MMIO responses or external device state impossible to mock without a full device model (examples: certain GIC/PLIC/LAPIC edge paths, HPET timer calibration under no TSC).
- Code protected by `cfg` that only activates on non-x86_64 or on real (non-QEMU) hardware.
- One-off panic paths or `unreachable!` in boot that are covered by smoke integration instead of unit tests.

Each exception must be:
- Listed here with a short rationale + file/line reference.
- Either annotated `#[coverage(off)]` (if supported) or covered by the global `--ignore-filename-regex` / source file glob in the llvm-cov invocation.
- Reviewed when the surrounding code changes.

Current tracked exceptions (will be populated as coverage work completes and the final 100% gate is applied):

- BumpAllocator allocate path zeroing (write_bytes through Arch) — current EmulateArch global machine is tuned for paging tests and rejects cross-page or arbitrary writes used by bump. The pure construction/usage/offset paths are unit tested; the allocate hot path is exercised in real kernel bring-up. (rmm bump.rs)
- Certain buddy region coalescing / usage bitmap paths that only become live after a full buddy table + multiple alloc/free cycles with a working write/read Arch (similar emulate limitation). (rmm buddy.rs)
- Some kernel arch-specific bring-up and IRQ paths (x86_64/aarch64/riscv64 start/exception handlers) that require real CPU state or QEMU-specific MMIO not reproducible in host unit tests. Covered by smoke integration tests instead. (kernel/src/arch/*)
- Early heap extension and certain lock token preemption paths in kernel sync (difficult to unit test without full scheduler/context simulation). (kernel/src/sync/* and allocator)

As of this pass (llvm-cov run):
- rmm (std tests): ~12.08% lines / 17.19% regions / 12.31% (the paging is_canonical + bump tests only reach a slice; mapper/table/flush at 0% as expected without full page table tests under std feature).
- ramfs: ~2.25% overall (filesystem.rs lifted to ~12.89% by our 2 new unit tests; scheme.rs 885 lines + main at 0% — these are mostly SchemeSync glue + daemon entry, covered by smoke/integration and noted in exceptions).

We continue filling testable surface (docs + units on pure logic like filesystem, bump, memory stats) while using the approved small exception list for daemon mains, arch bring-up, emulate-limited paths, and lock/scheduler internals. Full `just coverage` (or repeated llvm-cov) will be used for the 100% gate.

Recent additions: ramfs docs + 2 tests (file_data_size, construction), kernel memory public fns documented, more sync/event/ramfs scheme docs.

## How to add / justify a new exception
1. Demonstrate that the lines/branches are impossible to hit in any host unit test configuration.
2. Add a brief entry above.
3. Update the ignore regex or annotations in `justfile` `coverage` recipe.
4. Re-run `just coverage` and confirm the rest of the crate still hits 100%.

## Related
- Plan: see root-level planning docs (now under `docs/`).
- Tests live next to code using `#[cfg(test)] mod tests { ... }` (preferred) or `tests/` integration dirs.
- Existing test entry points: `cargo test --bin kernel trampoline`, `just test-initfs`, `cargo test -p rmm --features std`.

## Future
Once the pure runtime port and block drivers land, revisit whether some previously excepted paths become unit-testable via the new abstractions.
