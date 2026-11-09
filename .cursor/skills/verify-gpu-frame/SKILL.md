---
name: verify-gpu-frame
description: >
  Use when changing sddf-gpu-driver flush, resource transfer, or the gpu_virt
  note. The check flushes only a staged transfer and clears the note after
  the response.
---

# Verify the GPU frame

## Property

A flush succeeds only after a transfer is staged, and a display response clears the note it used.

## Harm

The screen shows a frame that was never transferred, or the next request reuses a note that still says the previous response is in flight. The browser smoke paints one fixture.

## Result on this tree

The check passes. TLC finds 18 distinct states. Flush from an attached resource with no transfer is refused, and the response sets the note free.

## Counterexample the check catches

In `flush`, return `GPU_RESP_OK` from the `resource.transfer` else branch instead of `GPU_RESP_ERR_INVALID_PARAMETER`. The check exits 1.

1. Create the resource and attach the backing.
2. Flush runs with no transfer and returns success.
3. `FlushNeedsStage` fails.

Removing `note.live = false` from `forward_responses` fails `NoteCleared`: the response was handled and the note is still live.

## Run

```bash
support/verify/gpu-frame/check.sh
```

The script uses TLC 1.7.4. It downloads `tla2tools.jar` into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc/` when that file is missing. The sha256 is `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`. Java 11 or newer must be on `PATH`. The script changes to the repo root before Cargo so rustup selects this repo's toolchain.

Exit 0: the source matches the model and every invariant holds. Exit 1: the source broke the property. Exit 2: the probe could not read the functions, the committed config drifted, or TLC failed while the source still matched.

Read every TLC result with `/statechart`. It shows each broken rule as its shortest path in plain words. When the check passes, it draws how the frame moves.

## Coverage

| Step | Source |
| --- | --- |
| Create, attach, transfer, flush, detach, unref | `flush` and the resource methods in `userspace/pds/sddf-gpu-driver/src/main.rs` |
| Note goes live, then free | `forward_requests` and `forward_responses` in `userspace/pds/sddf-gpu-virt/src/main.rs` |

One resource and one note. Blob rejection and the scanout size are outside this check.

## Sync commit

`e599121d159bd050d2e69547eb42bfcc7deb791b`

Move this hash forward only after you re-read the functions above, update the model so each step is still one branch, and re-run the check at the new commit.

## When the covered code changes

Re-run the check. Exit 1 is a real break. Exit 2 means the probe lost the function or TLC disagrees with source that still has the branches. Fix the model or the probe. Do not weaken `FlushNeedsStage` or `NoteCleared`.

## Bounds

One resource record and one note slot. The model does not cover pixel bytes, a second resource id, or the virtio device status register.

## Tools

TLC, because the question is whether flush can succeed before the transfer is staged. The driver does not run on the host, so the probe reads the branch and the model checks the order.

Bring this check back in line with later edits through `/maintain-formal-verification-skill`.
