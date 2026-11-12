---
name: verify-cothread-contract
description: >
  Use when changing lerux-cothread init, spawn, yield, destroy, semaphore
  wait or signal, wait_on_channel, recv_ntfn, or the host lock around that
  runtime. Re-check that only the root receives, a worker blocks in wait,
  signal and recv switch to the waiter before they return, a contract break
  has no next state, and a full pool returns NULL_HANDLE.
---

# Verify the cothread contract

## Property

The root cothread is the kernel thread. Only the root calls `microkit_cothread_recv_ntfn`. A worker blocks in `microkit_cothread_wait_on_channel` or `microkit_cothread_semaphore_wait`. `microkit_cothread_semaphore_signal` and `microkit_cothread_recv_ntfn` switch to the waiter before they return.

A second init, a bad handle, a wait on the root, `recv_ntfn` from a worker, and destroying a blocked cothread panic. Those calls have no next state. Spawn of a full pool returns `NULL_HANDLE` and does not panic. Assertions that the replay itself checks stay on the root. The replay takes a process lock before it uses the runtime and drops that lock before every context switch.

## Harm

A `recv_ntfn` that returns without switching leaves the worker inside `microkit_cothread_wait_on_channel` while the root continues. A contract break that returns instead of panicking keeps a handle the pool no longer owns, or lets the root block on its own kernel thread. A process lock held across `co_switch` deadlocks the host exercise: the mutex is not reentrant, and the waiter is the same operating-system thread.

## Result on this tree

The check exits 0. The runtime was not changed. Wake is 22 states: the channel waiter runs before `recv_ntfn` returns, then the semaphore waiter runs before `semaphore_signal` returns. Full is 20 states: seven spawns fill the pool, the next spawn is `NULL_HANDLE`, and those ready handles are destroyed from the root. With the panic constants true, each forbidden action is disabled. The state graph's outdegree is 1.

## Counterexample the check catches

Delete the `switch::co_switch` call in `microkit_cothread_recv_ntfn` and leave `signal_one`. The check exits 1. The Rust schedule prints `verdict=contract` with `recv_switched=false`, and TLC reports `Invariant SwitchBeforeReturn is violated`. `signal_one` has already marked the waiter running and the root ready. The steps are:

1. `DoInit` leaves the root running and seven worker slots free.
2. `SpawnCh` queues the channel waiter.
3. `YieldCh` runs that waiter. `WaitCh` blocks it and returns to the root.
4. `RecvSwitch` sees `RecvSwitched = FALSE`, so it records the return without setting `recvResumed`, and the phase becomes `badRecv`.

The semaphore half of the schedule is not reached. A passing wake trace is not evidence that a contract break still panics. Those are separate actions. `SecondInit`, `BadHandle`, `RootWait`, `WorkerRecv`, and `DestroyBlocked` are enabled only when the matching call returns. Each one stores its name in `breach`. `NoForbidden` is `breach = "none"`. Deleting the root assert in `recv_ntfn` sets `WorkerRecvPanics = FALSE`. TLC then takes `WorkerRecv` while the waiter is running and reports `Invariant NoForbidden is violated`.

## Run

From the repository root:

```bash
support/verify/cothread-contract/check.sh
```

Java 11 or newer is required. The first run downloads TLC 1.7.4 (`tla2tools.jar`, sha256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`) into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc`. Later runs reuse that jar. Do not commit the jar.

Exit status:

- **0.** Both checkers accept wake and full. The waiter ran before signal and recv returned, the extra spawn is `NULL_HANDLE`, every contract break panicked, and the process lock was not held across a switch.
- **1.** The Rust schedule prints `verdict=contract` or its process exits 1. The runtime disagreed with the contract. This is the code, including when TLC fails as well.
- **2.** The replay tests fail, the probed constants differ from `wake.cfg` or `full.cfg`, the probed `null_handle` is not `-1`, the search cannot be read, or the Rust schedule accepts a run that TLC rejects. Treat that as a modeling error until the same calls on the runtime show the disagreement. A config file cannot contain a negative literal, so `-1` is the module definition `MinusOne`.

Read every TLC result with `/statechart`. It shows each invariant violation as its shortest path in plain words. When the check passes, `/statechart` draws the wake schedule and the full-pool schedule. Its commands and paths change between releases, so they are not copied here.

## Coverage

| Step | Source |
| --- | --- |
| init, second init panics | `microkit_cothread_init` |
| bad handle panics | `idx` |
| spawn, or `NULL_HANDLE` when the free list is empty | `microkit_cothread_spawn` |
| yield pushes the caller and runs the next ready cothread | `microkit_cothread_yield` |
| destroy, including the blocked panic | `microkit_cothread_destroy`, `release` |
| semaphore wait and signal | `microkit_cothread_semaphore_wait`, `microkit_cothread_semaphore_signal` |
| channel wait | `microkit_cothread_wait_on_channel` |
| recv from the root, then switch | `microkit_cothread_recv_ntfn` |
| the root cannot block | `block_running` |
| signal one waiter and leave it running | `signal_one` |
| process lock around the runtime, dropped before a switch | host tests in `lerux-cothread`, and `support/verify/cothread-contract/replay` |

Line numbers are the comment at the top of `support/verify/cothread-contract/CothreadContract.tla`. One TLA action is one branch in that source. The Rust twin is `support/verify/cothread-contract/replay`. It calls the real runtime. The TLA module does not execute Rust. The two have to change together when a branch changes. `wake.cfg` and `full.cfg` are the probed constants at the sync commit. The check regenerates them from the runtime and exits 2 when the committed files have fallen behind.

Sync commit: `c7ba3dc5404dc80491ca978e358279b1892dd0d3`

Move that hash forward only after you have re-read the covered functions at the new commit, updated both models so each step is still one branch, and re-run this check.

## When the covered code changes

1. Read the diff of `userspace/crates/lerux-cothread/src/lib.rs`. Leave the assembly syntax in `switch.rs` alone.
2. If a branch was added, removed, or reordered, change that action in `CothreadContract.tla` and the same step in `replay/src/main.rs`. Keep `Release` immediately before a step that changes `running`.
3. If only a panic condition changed, the Rust checker already calls it. Change the TLA action when the call returns instead of panicking, or when a full pool returns something other than `NULL_HANDLE`. Update `wake.cfg` and `full.cfg` from the new probe only after that effect is what you intend.
4. Re-run the check. On exit 2, replay the trace as calls on the real runtime. A step you cannot replay is a modeling error. Fix the model. Leave `NoForbidden` and `SwitchBeforeReturn` as they are.
5. Update the sync commit only after that run.

## Bounds and what is left out

- One kernel thread. The root handle is 0. The pool is `STACK_SLOTS` (7). `NULL_HANDLE` is -1.
- Wake spawns one channel waiter, yields, receives, then spawns one semaphore waiter and signals. Full spawns seven ready workers, records the next spawn, and destroys those ready handles. Spawn does not enter the worker. The parked entry sets `worker_panicked` if it runs.
- The root-wait probe uses a clear semaphore and a clear channel. A sticky signal consumed by the root is not that probe. `block_running` panics only when the root would block.
- The replay's stacks are `MIN_STACK_SIZE` (`0x1000`) and 16-byte aligned. There is no guard page. The context switch is the crate's existing assembly. This model does not describe registers.
- The host tests in `lerux-cothread` take a private process lock. This replay takes its own `LOCK` the same way, because that test lock is not public and the runtime does not lock. The worker `try_lock` must succeed on entry and again when signal or recv resumes it. A failure sets `LockAcrossSwitch`, which enables `LockSwitch` and sets `lockFault`.
- Both configs leave the forbidden actions disabled. The explorer stops at the one legal schedule. A second enabled action is a contract break that returned.
- Left out: a second kernel thread, a sticky signal with a waiter already queued, destroy of the root, `my_arg`, yield when the caller is the only ready cothread, and the Microkit notification that the cothread smoke delivers.

## Tools

TLC 1.7.4 checks that the legal schedule never enters a forbidden state and that signal and recv return only after the waiter has run. The Rust program runs that schedule on the real runtime, so a deleted switch or a missing panic fails the check before anyone edits the TLA actions.

After a change that did not re-run this check, use `/maintain-formal-verification-skill`.
