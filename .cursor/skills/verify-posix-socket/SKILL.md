---
name: verify-posix-socket
description: >
  Use when changing the one-socket table: Sockets::socket, connect, listen,
  accept, send, shutdown, or close; Tcp::listen; Files::open, idle, or
  exchange; or the descriptor numbers 0, 1, and 2. Re-check that the
  documented errnos are the only exits and that close leaves the table empty.
---

# Verify the one-socket table

## Property

Descriptors 0, 1, and 2 are not stored. The first `open` returns 3. The first `socket` also returns 3. A second `socket` while one is open returns errno 24. `listen` on an idle socket binds one port and keeps that same socket. `accept` waits until the peer is connected. A default `Tcp::listen` returns errno 95. A listen the stack rejects as already bound returns errno 98. `shutdown` finishes an open socket and leaves the table closed. `close` aborts and returns the table to empty. `send` before `connect` or `accept` returns errno 107. A second `connect` returns errno 106. A second call while a command is in flight returns errno 16.

## Harm

`fetch` and `chat` each connect once in a boot. If `close` leaves the entry closed, the next `socket` returns 24 and that second connection never starts. A `send` that succeeds before the peer is connected writes on a pipe that is not open. A second `connect` that succeeds shares one pipe. A `listen` that allocates another socket, or an `accept` that returns before the peer connects, breaks the one-entry table. A second filesystem call during a command mixes two completions on one ring.

## Result on this tree

The check exits 0. The library already matched these exits, so `userspace/crates/lerux-posix` was left unchanged. The Rust checker walks 6 socket phases. TLC finds 121 distinct states and a depth of 7. The extra states are the file slot, the in-flight flag, and the last operation paired with each phase.

## Counterexample the check catches

In `Sockets::close`, set `self.phase = Phase::Closed` instead of `Phase::Empty`. The check exits 1. The replay prints `verdict=table` and `still occupied errno 24`. The steps are:

1. `socket` returns descriptor 3. The phase is idle.
2. `close` returns success and leaves the phase closed.
3. The next `socket` returns errno 24. The table is not empty.

TLC still passes on that edit, because the module was not changed. The replay calls `close` and then `socket`.

Setting `phase' = "closed"` in the TLA `Close` action, and leaving `fd' = FirstFd`, makes TLC report `Invariant EmptyAfterClose is violated`. The replay still passes. That is a model break. Do not weaken `EmptyAfterClose` or `OnlyExits` to hide it.

Deleting both `Errno::EBUSY` returns, in `idle` and in `exchange`, makes the nested `open` return something other than errno 16. The replay prints `verdict=errno`.

A pipe whose `poll` never reports ready, if `accept` returns success anyway, prints `verdict=table` and `accept returned before the peer was connected`.

## Run

From the repository root:

```bash
support/verify/posix-socket/check.sh
```

Java 11 or newer is required. The first run downloads TLC 1.7.4 (`tla2tools.jar`, sha256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`) into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc`. Later runs reuse that jar.

Exit status:

- **0.** The replay and TLC both accept the table. The errnos above are the exits that occur, `shutdown` leaves the entry closed, and `close` leaves it empty.
- **1.** The replay prints `verdict=errno` or `verdict=table`, or its process exits 1. A call returned a different number, a descriptor below 3 appeared, `accept` succeeded before the peer was connected, or `close` left the entry occupied. TLC syntax errors, assumption failures, and invariant failures are also exit 1.
- **2.** The crate source is not a shape this model can read, the replay tests fail, the search passes 10000 states, or the probed constants differ from `socket.cfg`. Treat that as a modeling error until the same calls can be replayed on `Sockets` and `Files`.

`verdict=errno` is a wrong number. `verdict=table` is the entry: empty after `close`, still held after `shutdown`, the same socket after `listen`, or `accept` before the peer.

Read every TLC result with `/statechart`. It shows each invariant violation as its shortest path in plain words. When the check passes, `/statechart` draws how the table moves. Its commands and paths change between releases, so they are not copied here.

## Coverage

| Step | Source |
| --- | --- |
| First socket is 3, second socket is 24 | `Sockets::socket` |
| Second connect is 106 | `Sockets::connect` |
| Default listen is 95 | `Tcp::listen` |
| Bound listen is 98, successful listen keeps the socket | `Sockets::listen` |
| `accept` waits until the peer is connected | `Sockets::accept`, `wait_until_open` |
| `send` before connect or accept is 107 | `Sockets::send`, `open` |
| `shutdown` leaves the entry closed | `Sockets::shutdown` |
| `close` leaves the table empty | `Sockets::close` |
| First open is 3 | `Files::open`, `index` |
| A second call in flight is 16 | `Files::idle`, `Files::exchange` |

Line numbers are the comment at the top of `support/verify/posix-socket/PosixSocket.tla`. One TLA action is one branch in that source. `BusyCall` is the action and `Busy` is the errno constant; TLA cannot use one name for both. The Rust twin is `support/verify/posix-socket/replay`. It calls `Sockets` against a pipe and `Files` against a fake queue. The TLA module does not execute Rust. The two have to change together when a branch changes. `socket.cfg` is the probed constants at the sync commit. The check regenerates it from the functions and the source, and exits 2 when the committed file has fallen behind.

Sync commit: `c7ba3dc5404dc80491ca978e358279b1892dd0d3`

Move that hash forward only after you have re-read the covered functions at the new commit, updated both models so each step is still one branch, and re-run this check.

## When the covered code changes

1. Read the diff of the files in the coverage table.
2. If a branch was added, removed, or reordered, change that action in `PosixSocket.tla` and the function of the same name in `replay/src/main.rs`. Keep `Close` as the step that stores empty, and `Shutdown` as the step that stores closed.
3. If only the errno constant changed, the Rust checker already calls the function. Change the TLA action when the visible effect changed: the number returned, the phase stored, or which descriptor is allocated. Update `socket.cfg` from the new probe only after that effect is what you intend.
4. Re-run the check. On exit 2, replay the trace as calls on `Sockets` and `Files`. A step you cannot replay is a modeling error. Fix the model. Leave `OnlyExits` and `EmptyAfterClose` as they are.
5. Update the sync commit only after that run.

## Bounds and what is left out

- One socket and one file slot. The Rust script's second `open` returns 4 only to show the in-flight flag has cleared. That number is not a TLA state.
- `PeerArrives` is the pipe's `poll` reporting ready. It is not a `lerux-posix` call. `Accept` stays disabled until that flag is set. A blocked pipe must not make `accept` return success. Timeout, `ECONNREFUSED`, and `EINVAL` are outside this model.
- The nested `open` runs from the wake, which is the window where a command is in flight. It returns from `idle` before it writes a slot.
- Descriptors 0, 1, and 2 have no constructor in the source the probe reads. The table length is `MAX_FDS - 3`.
- The socket search is the six phases empty, idle, listening, listening with a peer, open, and closed. TLC also tracks the file slot and the last operation, 121 states. The explorer stops at 10000 states and exits 2.
- Left out: `EBADF`, `ETIMEDOUT`, `ECONNREFUSED`, `ENOENT`, directory reads, and a second socket after the table has grown past one entry.

## Tools

TLC 1.7.4 checks that those errnos are the only exits and that the table is empty after `close`. The Rust program runs the same steps on `Sockets` and `Files`, so a changed return fails the check before anyone edits the TLA actions.

After a change that did not re-run this check, use `/maintain-formal-verification-skill`.
