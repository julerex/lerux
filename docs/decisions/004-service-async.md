# ADR-004: Stackless cooperative async inside service PDs

## Status

Accepted (Phase 45)

## Date

2026-07-12

## Context

Service PDs such as `fs-server` and `net-server` sit on Microkit’s event-handler model (`Handler::protected` / `Handler::notified`). Multi-step block and network I/O is today encoded as **explicit state machines** (`FsJob` step counters, poll-until-ready sector helpers). That works, but long sequential operations (e.g. FAT format writing tens of sectors) are hard to read and easy to get wrong.

Upstream inspiration:

| Source | Model |
|--------|--------|
| [libmicrokitco](https://github.com/au-ts/libmicrokitco) | Stackful cothreads + semaphores; sync-looking code over async notifies |
| rust-sel4 `sel4-async-*` | Full `LocalPool` + futures (http-server example) |

Constraints from lerux:

- Userspace stays **Rust-only** (`#![no_std]` + optional `alloc` in service PDs).
- Do **not** port C `libmicrokitco` or add stack MRs for cothreads without a strong need.
- **Clients** keep postcard `Poll` RPC (`FsRequest::Poll`, `NetRequest::Poll`); Phase 45 is **server-internal**.
- Prefer small, host-testable crates over pulling experimental rust-sel4 async stacks as workspace deps.

## Decision

1. **Adopt stackless cooperative async** for server-internal I/O sequencing:
   - Express sequential device ops as `core::future::Future` (async/await or manual `poll`).
   - Drive a **single outstanding task** from `Handler::protected` / `Handler::notified` via `run_until_stalled`.
   - Wake the task when the block (or net) driver notifies.

2. **Ship `lerux-service-async`**: minimal `no_std` helpers (`poll_fn`, single-task runner, channel wake flag). No full multi-task executor, no preemption, no cothread stacks.

3. **Adopt first in `fs-server`**: sector I/O gains a waker-friendly API; the LERUXFS2 **format** path runs as an async task. Remaining FS jobs may stay as step machines until migrated. Net-server migration is a follow-up.

4. **Do not** change app-facing `FsRequest` / `NetRequest` contracts in this phase.

## Alternatives considered

### Port libmicrokitco (C cothreads)

Rejected: C userspace dependency; separate co-stacks and controller MRs; PPC interacts poorly with multi-cothread PD (documented footguns). Stackful sync is attractive for pure compute clients, not required for reactive service PDs that already live in `Handler`.

### Keep only explicit step machines forever

Rejected as sole long-term model: FAT format and multi-cluster I/O already show the cost. Step machines remain valid for simple jobs and can coexist with async tasks.

### Depend on experimental `sel4-async-single-threaded-executor` workspace-wide

Deferred: powerful, but pulls `futures`, thread-locals, and a larger dependency surface than Phase 45 needs. May revisit for net/DHCP-style concurrency; v1 stays in-tree and tiny.

## Consequences

- Service authors can write sequential `read_sector(…).await` / `write_sector(…).await` for long I/O chains.
- Handler remains the root of the PD; the Microkit event loop is never blocked spinning on rings.
- Clients still spin on `Poll` until the server finishes a job (same external latency model).
- Residual: migrate more `FsJob` / net ops; optional multi-task spawn later.

## References

- [libmicrokitco](https://github.com/au-ts/libmicrokitco)
- rust-sel4 microkit http-server (async Handler pattern)
- Phase 45 in [plan-au-ts.md](../plan-au-ts.md)
- [`lerux-service-async`](../../userspace/crates/lerux-service-async/)
