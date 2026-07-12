# ADR-005: Debug protection domain (stock Microkit faults + QEMU GDB)

## Status

Accepted (Phase 46)

## Date

2026-07-12

## Context

Phase 46 wants a workflow to inspect a crashing or hung protection domain under QEMU (then RPi4), inspired by [libgdb](https://github.com/au-ts/libgdb) (debugger PD as fault handler; serial or TCP GDB RSP).

libgdb’s README states it depends on **seL4 and Microkit patches that are not upstream**. Stock lerux uses Microkit **2.2.0** and rust-sel4 **v4.0.0** with no forked kernel.

Stock Microkit already supports:

- **Monitor fault dump** for top-level PDs (debug configuration).
- **Parent/child hierarchy**: a child PD’s faults are delivered to the parent’s `fault` entry point (`Handler::fault` in rust-sel4), with access to the child’s TCB cap index.

## Decision

1. **Do not** vendor or depend on libgdb, or on forked seL4/Microkit trees, for the default debug story.
2. **In-tree path (v1):** a small hierarchy demo on `qemu_virt_aarch64`:
   - `debug-handler` (parent) implements `Handler::fault`, logs fault kind / IP / address via kernel debug UART.
   - `crash-demo` (child) deliberately raises a VM fault after logging readiness.
3. **Host GDB path:** document QEMU’s built-in gdbstub (`-s` / `tcp::1234`) + `gdb-multiarch` with PD ELF symbols for interactive backtraces. This does not require kernel patches.
4. Stretch (later): TCP GDB RSP inside a PD only if/when upstream Microkit exposes the APIs libgdb needs, or we accept a documented fork.

## Alternatives considered

### Ship libgdb + forked Microkit/seL4

Rejected for default path: breaks AGENTS.md “do not modify deps/workspace upstream trees” for routine work; dual SDK maintenance; not CI-default.

### Only document QEMU gdbstub, no in-tree fault PD

Rejected as sole outcome: does not exercise Microkit hierarchy/`fault` and is harder to smoke-test in `lerux-cli test` expects.

### Full GDB RSP stub in Rust over serial

Deferred: large protocol surface; needs careful TCB/register mapping; hierarchy + QEMU gdbstub cover the Phase 46 exit criterion.

## Consequences

- `just test-debug` proves: child faults → parent logs structured fault info (smoke strings).
- Developers use [`docs/debug.md`](../debug.md) for QEMU + gdb-multiarch symbol backtraces.
- Full in-guest GDB server remains optional and gated on upstream or an explicit fork decision.

## References

- [libgdb](https://github.com/au-ts/libgdb)
- Microkit manual — Faults / hierarchy example
- rust-sel4 `Handler::fault` / `MessageInfo::fault` / `Child`
- Phase 46 in [plan-au-ts.md](../plan-au-ts.md)
