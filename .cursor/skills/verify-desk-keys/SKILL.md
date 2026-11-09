---
name: verify-desk-keys
description: >
  Use when changing from_set1, HID decode, UsbKbd::poll, or PortStop::later.
  The check clears the extended prefix, suppresses a held usage, waits until
  the controller is ready, and never moves a port stage backward.
---

# Verify desk keys

## Property

After an extended prefix, the following letter is delivered once the prefix is consumed. A usage that is still held is not emitted again. A poll emits a key only after the controller is ready. A port stage never moves backward.

## Harm

The extended prefix swallows the next letter, a held key repeats on every report, a poll emits a key before the controller is up, or a later Bluetooth failure erases a stage the port already reached. The console smoke sends one maths line.

## Result on this tree

The check passes. TLC finds 240 distinct states because the four walks run side by side. The letter after the prefix is delivered, the second report is silent, the early poll emits nothing, and the port rank only increases.

## Counterexample the check catches

Remove `state.extended = false` that follows `let extended = state.extended` in `from_set1`. The check exits 1.

1. You press the extended prefix (`0xE0`).
2. You press the next byte. The prefix stays set.
3. You press A (`0x1E`). `from_set1` returns `None`.
4. `LetterAfterPrefix` fails.

The host call is the same sequence: the third byte must be `Key::Char(b'a')`. Removing `state.down.contains(&usage)` in `decode` fails `SecondReportSilent`. Removing `if !self.ready` in `UsbKbd::poll` fails `PollWaitsReady`. Changing `PortStop::later` so a lower rank replaces a higher one fails `RankOnlyForward`, and the host call `Desc.later(Reset)` must stay `Desc`.

## Run

```bash
support/verify/desk-keys/check.sh
```

The script uses TLC 1.7.4. It downloads `tla2tools.jar` into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc/` when that file is missing. The sha256 is `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`. Java 11 or newer must be on `PATH`. The script changes to the repo root before Cargo so rustup selects this repo's toolchain.

Exit 0: the source matches the model and every invariant holds. Exit 1: the source broke the property. Exit 2: the probe could not read the functions, the committed config drifted, or TLC failed while the source still matched.

Read every TLC result with `/statechart`. It shows each broken rule as its shortest path in plain words. When the check passes, it draws how the keys move.

## Coverage

| Step | Source |
| --- | --- |
| Prefix, next byte, letter | `from_set1` in `userspace/crates/lerux-input/src/lib.rs` |
| Held usage | `decode` in `userspace/pds/console-driver/src/hid.rs` |
| Poll before ready | `UsbKbd::poll` in `userspace/pds/console-driver/src/xhci.rs` |
| Port stage | `PortStop::later` in `userspace/crates/lerux-swift/src/bt.rs` |

`console-driver` `scancode.rs` has a second set-1 decoder. This check's host proof is `from_set1`. The chart labels only that letter. It hides the held-key report, the controller poll, and the Bluetooth port, which run beside the key and do not change the letter.

The labels are in `support/verify/desk-keys/view.mjs`. If you change a step the reader can see, update that file so the labels stay in the reader's words. Do not give a set-1 failure and a HID failure the same label.

## Sync commit

`e599121d159bd050d2e69547eb42bfcc7deb791b`

Move this hash forward only after you re-read the functions above, update the model so each step is still one branch, and re-run the check at the new commit.

## When the covered code changes

Re-run the check. Exit 1 is a real break. Exit 2 means the probe lost the function or TLC disagrees with source that still has the branches. Fix the model or the probe. Do not weaken `LetterAfterPrefix`, `SecondReportSilent`, `PollWaitsReady`, or `RankOnlyForward`.

## Bounds

One prefix and one letter, two HID reports, one poll, and port ranks 0 through 3. The model does not cover the key ring, rollover, or the xHCI event ring.

## Tools

TLC, because the question is whether the prefix, the poll, and the port stage can move in a bad order. `from_set1` and `PortStop::later` run on the host. `decode` and `UsbKbd::poll` do not, so the probe reads those guards.

Bring this check back in line with later edits through `/maintain-formal-verification-skill`.
