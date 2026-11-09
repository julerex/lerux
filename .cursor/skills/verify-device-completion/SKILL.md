---
name: verify-device-completion
description: >
  Use when changing e1000e descriptor-done checks, GENET DMA_OWN, the NVMe
  completion phase, or AHCI poll_pxci. The check copies a buffer only after
  the device has finished with it.
---

# Verify device completion

## Property

Software copies a buffer only after the device has finished with it.

## Harm

The driver reads or writes a buffer the device still owns, so the frame or the disk block is torn. Each driver's smoke sees a device that completes.

## Result on this tree

The check passes. TLC finds 3 distinct states. The copy happens after the device finishes. The constant is true only when all four checks below are present.

## Counterexample the check catches

Remove the `RX_DD` test from `copy_rx`. The check exits 1.

1. The device has not finished.
2. Software copies the buffer.
3. `CopyAfterDone` fails.

The same exit happens if any one of these is missing: `TX_DD` in `reclaim_tx`, `DMA_OWN` in GENET `transmit` or `receive`, the NVMe phase compare `(dw3 >> 16) & 1 == phase`, or `if !poll_pxci` in AHCI `run_dma`. The failure names the missing site.

## Run

```bash
support/verify/device-completion/check.sh
```

The script uses TLC 1.7.4. It downloads `tla2tools.jar` into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc/` when that file is missing. The sha256 is `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`. Java 11 or newer must be on `PATH`. The script changes to the repo root before Cargo so rustup selects this repo's toolchain.

Exit 0: the source matches the model and the invariant holds. Exit 1: the source broke the property. Exit 2: the probe could not read the functions, the committed config drifted, or TLC failed while the source still matched.

Read every TLC result with `/statechart`. It shows a broken rule as its shortest path in plain words. When the check passes, it draws how completion moves.

## Coverage

| Step | Source |
| --- | --- |
| Receive descriptor done | `copy_rx` in `userspace/pds/sddf-e1000e-driver/src/engine.rs` |
| Transmit descriptor done | `reclaim_tx` in the same file |
| GENET ownership bit | `transmit` and `receive` in `userspace/pds/genet-driver/src/genet.rs` |
| NVMe phase bit | `poll` in `userspace/pds/swift-nvme/src/main.rs` |
| AHCI command slot | `run_dma` in `userspace/pds/sddf-ahci-driver/src/engine.rs` |

One buffer. The four devices share the step "the device finishes, then software copies". The model does not cover ring indexes or a second descriptor.

## Sync commit

`SYNC_COMMIT`

Move this hash forward only after you re-read the functions above, update the model so each step is still one branch, and re-run the check at the new commit.

## When the covered code changes

Re-run the check. Exit 1 is a real break. Exit 2 means the probe lost a function or TLC disagrees with source that still has the checks. Fix the model or the probe. Do not weaken `CopyAfterDone`.

## Bounds

One buffer and one completion. A missing check on any of the four devices fails the whole constant, so the counterexample is the shared step and the message names the device.

## Tools

TLC, because the question is whether the copy can happen while the device still owns the buffer. The drivers do not run on the host, so the probe reads each guard.

Bring this check back in line with later edits through `/maintain-formal-verification-skill`.
