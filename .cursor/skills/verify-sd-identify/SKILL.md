---
name: verify-sd-identify
description: >
  Use when changing emmc2 identify_card, read_sector, or write_sector. The
  check selects the card before the block length and refuses a transfer
  before the card is identified.
---

# Verify SD identify

## Property

The card is selected before it is ready, and a sector transfer runs only after that.

## Harm

A read or write goes to a card that is still idle, or the block length is set before the card is selected. The board smoke uses a card that is already identified.

## Result on this tree

The check passes. TLC finds 4 distinct states. Identify sends CMD0, CMD8, CMD2, CMD3, CMD7, then CMD16, and the transfer waits for `initialized`.

## Counterexample the check catches

Remove the `if !self.initialized` guard from `read_sector`. The check exits 1.

1. The card is still idle.
2. The sector read runs.
3. `ReadAfterSelect` fails.

The same guard is required in `write_sector`. Reordering CMD7 after CMD16 in `identify_card` fails `OrderHolds`: the card is marked ready without having been selected.

## Run

```bash
support/verify/sd-identify/check.sh
```

The script uses TLC 1.7.4. It downloads `tla2tools.jar` into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc/` when that file is missing. The sha256 is `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`. Java 11 or newer must be on `PATH`. The script changes to the repo root before Cargo so rustup selects this repo's toolchain.

Exit 0: the source matches the model and every invariant holds. Exit 1: the source broke the property. Exit 2: the probe could not read the functions, the committed config drifted, or TLC failed while the source still matched.

Read every TLC result with `/statechart`. It shows each broken rule as its shortest path in plain words. When the check passes, it draws how identify moves.

## Coverage

| Step | Source |
| --- | --- |
| CMD0, CMD8, CMD2, CMD3, CMD7, CMD16 | `identify_card` in `userspace/pds/emmc2-driver/src/emmc2.rs` |
| Transfer after `initialized` | `read_sector` and `write_sector` in the same file |

`init_card` sets `initialized` after `identify_card` and `read_capacity`. ACMD41 inside `wait_sdhc_ready` sits between CMD8 and CMD2 and is not a separate chart state.

## Sync commit

`e599121d159bd050d2e69547eb42bfcc7deb791b`

Move this hash forward only after you re-read the functions above, update the model so each step is still one command, and re-run the check at the new commit.

## When the covered code changes

Re-run the check. Exit 1 is a real break. Exit 2 means the probe lost the function or TLC disagrees with source that still has the sequence. Fix the model or the probe. Do not weaken `OrderHolds` or `ReadAfterSelect`.

## Bounds

One card and one transfer. The model checks that CMD7 appears before CMD16 in the identify sequence. It does not cover clock setup, the 4-bit switch, or the bytes of a sector.

## Tools

TLC, because the question is whether a transfer can run before the card is selected. The driver does not run on the host, so the probe reads the command order and the `initialized` guard.

Bring this check back in line with later edits through `/maintain-formal-verification-skill`.
