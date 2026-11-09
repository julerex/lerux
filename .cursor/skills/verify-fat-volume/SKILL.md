---
name: verify-fat-volume
description: >
  Use when changing sddf-fatfs file_busy, handle_dir_open, or load_block.
  The check refuses a second open and flushes a dirty block before reading
  another.
---

# Verify the FAT volume

## Property

A second file or directory open is refused, and a dirty transfer block is flushed before another block is read.

## Harm

Two writers share one open file, or a later read replaces a block that was edited and not written. The filesystem smoke runs one command at a time.

## Result on this tree

The check passes. TLC finds 10 distinct states. The second open is refused, and switching blocks clears the dirty bit only after a flush.

## Counterexample the check catches

In `load_block`, drop the `flush_block()` call so the read runs on its own. The check exits 1.

1. Load block A.
2. Edit it, so the block is dirty.
3. Load block B without flushing.
4. `NoLostBlock` fails.

Removing `file_busy()` from `create_file`, `open_read`, or `open_nested_read`, or removing the `dir.is_some()` guard in `handle_dir_open`, fails `OneFile`.

## Run

```bash
support/verify/fat-volume/check.sh
```

The script uses TLC 1.7.4. It downloads `tla2tools.jar` into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc/` when that file is missing. The sha256 is `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`. Java 11 or newer must be on `PATH`. The script changes to the repo root before Cargo so rustup selects this repo's toolchain.

Exit 0: the source matches the model and every invariant holds. Exit 1: the source broke the property. Exit 2: the probe could not read the functions, the committed config drifted, or TLC failed while the source still matched.

Read every TLC result with `/statechart`. It shows each broken rule as its shortest path in plain words. When the check passes, it draws how the volume moves.

## Coverage

| Step | Source |
| --- | --- |
| First open, second open, close | `create_file`, `open_read`, `open_nested_read`, `handle_dir_open` in `userspace/pds/sddf-fatfs/src/main.rs` |
| Load, edit, load the other block | `load_block` and `flush_block` in the same file |

Two transfer blocks stand in for the volume. The format walk in `stage_zero` is one pass and is not this machine.

## Sync commit

`e599121d159bd050d2e69547eb42bfcc7deb791b`

Move this hash forward only after you re-read the functions above, update the model so each step is still one branch, and re-run the check at the new commit.

## When the covered code changes

Re-run the check. Exit 1 is a real break. Exit 2 means the probe lost the function or TLC disagrees with source that still has the branches. Fix the model or the probe. Do not weaken `OneFile` or `NoLostBlock`.

## Bounds

One open file, one open directory, and two blocks. The model does not cover directory entries, cluster allocation, or a third block.

## Tools

TLC, because the question is whether a dirty block can be replaced before it is flushed. The server does not run on the host, so the probe reads the guards and the model checks the order.

Bring this check back in line with later edits through `/maintain-formal-verification-skill`.
