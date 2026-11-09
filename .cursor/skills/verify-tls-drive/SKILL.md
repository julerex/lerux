---
name: verify-tls-drive
description: >
  Use when changing ClientSession::drive or the connected flag in lerux-tls.
  The check sets connected only from WriteTraffic and ReadTraffic, after the
  socket is open.
---

# Verify the TLS client

## Property

`connected` becomes true only from a traffic state, which the session reaches after the socket is open.

## Harm

`drive_timeout` reports the session connected while the handshake has not reached traffic and the socket is still closed. Callers then send application data into a handshake.

## Result on this tree

The check passes. TLC finds 3 distinct states. `drive` sets `connected` only in the `WriteTraffic` and `ReadTraffic` arms, and only after `open`.

## Counterexample the check catches

Add `self.connected = true` as the first line of `drive`. The check exits 1.

1. The socket is still closed.
2. `drive` sets `connected`.
3. `ConnectedAfterTraffic` fails.

An assignment of `self.connected = true` whose preceding source is neither `WriteTraffic` nor `ReadTraffic` is the same failure. A count other than the two traffic assignments exits 2: the probe no longer knows the shape, so update the model instead of treating it as this bug.

## Run

```bash
support/verify/tls-drive/check.sh
```

The script uses TLC 1.7.4. It downloads `tla2tools.jar` into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc/` when that file is missing. The sha256 is `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`. Java 11 or newer must be on `PATH`. The script changes to the repo root before Cargo so rustup selects this repo's toolchain.

Exit 0: the source matches the model and the invariant holds. Exit 1: the source broke the property. Exit 2: the probe could not read `drive`, the committed config drifted, or TLC failed while the source still matched.

Read every TLC result with `/statechart`. It shows a broken rule as its shortest path in plain words. When the check passes, it draws how the session moves.

## Coverage

| Step | Source |
| --- | --- |
| Open the socket, then drive | `ClientSession::drive` in `userspace/crates/lerux-tls/src/lib.rs` |

`drive_timeout` reads `connected` and does not set it. rustls `ConnectionState` is not re-checked beyond the two arms that set the flag.

## Sync commit

`SYNC_COMMIT`

Move this hash forward only after you re-read `drive`, update the model so each step is still one arm, and re-run the check at the new commit.

## When the covered code changes

Re-run the check. Exit 1 is a real break. Exit 2 means the two traffic assignments moved or TLC disagrees with source that still has them. Fix the model or the probe. Do not weaken `ConnectedAfterTraffic`.

## Bounds

One session. The model does not cover record bytes, certificates, or a second drive after `Closed`.

## Tools

TLC, because the question is whether `connected` can become true before the socket is open. The probe reads the assignments. The session is not driven on the host here, because that needs a peer.

Bring this check back in line with later edits through `/maintain-formal-verification-skill`.
