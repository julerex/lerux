---
name: verify-net-session
description: >
  Use when changing take_dhcp, Stack::connect, Stack::listen, HttpNet::poll,
  or the hello datagram flags. The check ignores a deconfigured link, keeps
  connect and listen apart, and serves one page.
---

# Verify the network session

## Property

A deconfigured link does not install an address, connect and listen are never both set, the HTTP server serves one page, and the hello datagram is sent once.

## Harm

The guest uses an address from a link that just dropped, or a listen and a connect overlap on one socket, or the page is served again. The fetch smoke waits for one offer and one body.

## Result on this tree

The check passes. TLC finds 18 distinct states. A dropped link stays without an address, connect clears listen, and the second request stops.

## Counterexample the check catches

Remove `self.listening = false` from `Stack::connect` in `userspace/pds/sddf-shell/src/net.rs`. The check exits 1.

1. An offer installs the address.
2. `listen` sets `listening`.
3. `connect` sets `connecting` and leaves `listening` set.
4. `FlagsExclusive` fails.

The same pair is required in `userspace/pds/sddf-agent/src/net.rs`. Changing `Some(Event::Deconfigured) | None => None` in the shell, the agent, or `sddf-net-client` fails `DeconfiguredStaysBare`. Removing either `if self.served` gate in `HttpNet::poll` fails `OnePage`. Removing the `tx_done` return in hello's `poll_udp_tx` fails `HelloOnce`.

`Sockets::Phase` is a separate explicit machine. This check is the flag layer `connect` and `listen` write, including listen followed by connect.

## Run

```bash
support/verify/net-session/check.sh
```

The script uses TLC 1.7.4. It downloads `tla2tools.jar` into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc/` when that file is missing. The sha256 is `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`. Java 11 or newer must be on `PATH`. The script changes to the repo root before Cargo so rustup selects this repo's toolchain.

Exit 0: the source matches the model and every invariant holds. Exit 1: the source broke the property. Exit 2: the probe could not read the functions, the committed config drifted, or TLC failed while the source still matched.

Read every TLC result with `/statechart`. It shows each broken rule as its shortest path in plain words. When the check passes, it draws how the session moves.

## Coverage

| Step | Source |
| --- | --- |
| Offer and deconfigured | `take_dhcp` in `sddf-shell`, `sddf-agent`, and `sddf-net-client` |
| Connect and listen | `Stack::connect` and `Stack::listen` in the shell and the agent |
| One page | `HttpNet::poll` in `userspace/pds/http-server/src/net.rs` |
| One datagram | `poll_udp_tx` and `is_done` in `userspace/pds/hello/src/net.rs` |

The labels a reader sees are in `support/verify/net-session/view.mjs`. The chart hides the hello datagram and a second drop of a link that is already down. If you change a step the reader can see, update that file so the labels stay in the reader's words.

## Sync commit

`SYNC_COMMIT`

Move this hash forward only after you re-read the functions above, update the model so each step is still one branch, and re-run the check at the new commit.

## When the covered code changes

Re-run the check. Exit 1 is a real break. Exit 2 means the probe lost the function or TLC disagrees with source that still has the branches. Fix the model or the probe. Do not weaken `DeconfiguredStaysBare`, `FlagsExclusive`, `OnePage`, or `HelloOnce`.

## Bounds

One offer, one connect, and one listen. The model does not cover the TCP state machine inside smoltcp, the TLS record loop, or a second socket. The connected flag on the TLS session is `verify-tls-drive`.

## Tools

TLC, because the question is whether connect, listen, and a dropped link can overlap. Those protection domains do not run on the host, so the probe reads the branches and the model checks their order.

Bring this check back in line with later edits through `/maintain-formal-verification-skill`.
