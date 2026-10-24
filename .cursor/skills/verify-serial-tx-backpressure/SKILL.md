---
name: verify-serial-tx-backpressure
description: >
  Use when changing sDDF serial transmit backpressure: the client emit or say
  loop in sddf-shell, sddf-grok, or sddf-program; serial_virt_tx move_client,
  provide, wake_client, or resume_pending; serial_driver drain_tx; or
  serial_enqueue, serial_dequeue, and the producer_signalled helpers in
  lerux-sddf. Re-check that a full client transmit ring cannot leave the
  client blocked with no wake scheduled, and that accepted bytes stay in order.
---

# Verify serial transmit backpressure

## Property

Every byte a client passes to `emit` or `say` is written by `drain_tx` in that order, and the call returns.

## Harm

The client worker stays inside `microkit_cothread_wait_on_channel`. On `sddf-program`, `Guest::log` passes the guest buffer to that loop with no length cap, so a `lerux.log` longer than the transmit data region hangs the protection domain. `sddf-shell` `emit` and `sddf-grok` `say` are the same loop.

## Known finding

The check exits 1 on the current source. Keep that failure. It is the code, and the trace below is the one TLC prints for `NotStuck`.

At model capacity 2 (the image region is 4096 bytes; the first rejected enqueue is the same control state at every capacity):

1. `serial_enqueue` accepts byte 1.
2. `serial_enqueue` accepts byte 2. The client ring is full.
3. `serial_enqueue` returns -1 for byte 3. That byte is not stored.
4. `serial_request_consumer_signal` stores 0 in `producer_signalled`.
5. `microkit_cothread_wait_on_channel` blocks. The TX semaphore is clear, so `block_running` does not return early.
6. `serial_virt_tx` has no enabled step. `provide` runs from the client `notify`, and that `notify` is the statement after the loop. The driver ring is empty, so `drain_tx` does not notify the virtualiser. `pending` stays false.

The shell's callers pass at most `LOG_CAP` (2048) bytes, which is under `SERIAL_REGION_SIZE` (4096), so the shell commands in this tree do not take this path. The program log import does.

A product change that makes the client return is what turns exit 1 into exit 0. Update both models to the new branches first. Do not edit `NotStuck` so the run goes green.

Any other `NotStuck` trace is a new bug, or a model that has drifted from the source.

## Run

From the repository root:

```bash
support/verify/serial-tx-backpressure/check.sh
```

Java 11 or newer is required. The first run downloads TLC 1.7.4 (`tla2tools.jar`, sha256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`) into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc`. Later runs reuse that jar.

Exit status:

- **0.** Both models accept every byte and the client returns. The finding is gone.
- **1.** Byte order holds, and a reachable state has the client blocked, `virtRun = "none"`, `pending = FALSE`, and the driver ring empty. That is the known finding when the trace is the six steps above.
- **2.** The short emit failed, a byte was lost (`Order` / Rust `Order`), or the Rust schedule and the TLA model disagree. A lost byte is a bug in the queue functions, or a model that no longer matches them. A short-emit failure means the model no longer delivers two bytes into an empty ring of capacity 2. Treat that as a modeling error until `serial_enqueue` of those two bytes also fails on the real function.

The short emit is `fits.cfg` (`Count = 2`) plus `replay` `fits`. It must reach the UART and satisfy `EventuallyDone`. The hanging emit is `overflow.cfg` (`Count = 3`) plus `replay` `overflow`.

Read every TLC result with `/statechart`. It shows each invariant violation as its shortest path in plain words, and which of those violations a change fixes. When the check passes, `/statechart` draws how the client, the virtualiser, and the driver move. Its commands and paths change between releases, so they are not copied here.

## Coverage

| Step | Source |
| --- | --- |
| enqueue, request, wait, notify | `sddf-shell` `emit`, `sddf-grok` `say`, `sddf-program` `say` |
| `move_client`, `provide`, `wake_client`, `resume_pending` | `sddf-serial-virt-tx` |
| `drain_tx` | `sddf-serial-driver` |
| `serial_enqueue`, `serial_dequeue`, request, cancel, require | `lerux-sddf` `serial_queue.rs` |
| sticky wait | `lerux-cothread` `microkit_cothread_wait_on_channel`, `block_running` |

Line numbers are the comment at the top of `support/verify/serial-tx-backpressure/SerialTx.tla`. One TLA action is one branch in that source. The Rust twin is `support/verify/serial-tx-backpressure/replay`. It calls the real queue functions. The TLA module does not execute Rust. The two have to change together when a branch changes.

Sync commit: `41dd1272fc0a4e968964e8824c7261af4e5baaf9`

Move that hash forward only after you have re-read the covered functions at the new commit, updated both models so each step is still one branch, and re-run this check.

## When the covered code changes

1. Read the diff of the files in the coverage table.
2. If a branch was added, removed, or reordered, change that action in `SerialTx.tla` and the function of the same name in `replay/src/main.rs`. Leave `ShellRequest` and `ShellWait` as two steps. An interrupt can run `drain_tx` between them.
3. If only the body of `serial_enqueue`, `serial_dequeue`, or a signal function changed, the Rust checker already calls it. Change the TLA action when the visible effect changed: a byte stored or not, a signal word stored, or a return value that selects the other branch.
4. Re-run the check. On exit 2, replay the trace as calls on the real queue functions. A step you cannot replay is a modeling error. Fix the model. Leave `NotStuck` as it is.
5. Update the sync commit only after that run.

## Bounds and what is left out

- One client. Colour transfer is off. One core.
- Priorities are `serial_driver` 4, `serial_virt_tx` 3, and the client 1. Notifying a higher-priority domain runs it before the caller continues, so client steps stay disabled while `virtRun` is not `"none"`.
- Model capacity is 2. `SERIAL_REGION_SIZE` is `0x1000`. The hang is the first `serial_enqueue` that returns -1 while the driver ring is still empty. That control state is the same at every capacity when the emit is one byte longer than the ring.
- `drain_tx` writes every dequeued byte before it returns, because the UART write spins. `DriverIrq` is enabled when the driver ring holds a byte outside that synchronous drain. These two configs never reach that state: the ring is filled only inside `VirtDrain`, and the drain empties it before the client runs again.
- Both `producer_signalled` words start at 0. Microkit zeroes shared pages. The host test in `userspace/crates/lerux-sddf/tests/serial_queue.rs` initialises that word to 1. This check follows the image.
- Left out: the receive path, a second client, `serial_enqueue_local`, the begin string in `serial_virt_tx` init, timer preemption, and a second `emit` after the first returns.

## Tools

TLC 1.7.4 checks the order of the protection-domain steps. The question is whether the client can wait with no wake scheduled. The Rust program runs those steps on the real queue functions, so a change inside the queue functions fails the check before anyone edits the TLA actions.

After a change that did not re-run this check, use `/maintain-formal-verification-skill`.
