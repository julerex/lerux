---
name: verify-shell-session
description: >
  Use when changing sddf-shell app_menu, maths_repl, browse, or cmd_source.
  The check keeps quit out of maths, returns the browser to the menu, and
  rejects a nested source.
---

# Verify the shell session

## Property

`quit` leaves maths, the browser returns to the menu, and a `source` that starts while one is already running does not open a second one.

## Harm

A stuck maths prompt, a browser that never returns, or a nested `source` that runs the inner file. The menu smoke types one path and does not try the bad order.

## Result on this tree

The check passes. TLC finds 10 distinct states. Quit returns to the menu, the paint returns to the menu, and a nested source is rejected.

## Counterexample the check catches

Remove the `return` after `text == b"quit"` in `maths_repl`. The check exits 1.

1. The menu chooses maths.
2. The line is `quit`, and `maths_repl` does not return.
3. `QuitLeftTheMaths` fails: the last line is quit and the phase is still maths.

`app_menu` leaving `browse` with `break` or `return` fails `BrowserReturned`. `cmd_source` falling through into `source_file` while `SOURCING` is already set fails `NoNestedSource`.

## Run

```bash
support/verify/shell-session/check.sh
```

The script uses TLC 1.7.4. It downloads `tla2tools.jar` into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc/` when that file is missing. The sha256 is `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`. Java 11 or newer must be on `PATH`. The script changes to the repo root before Cargo so rustup selects this repo's toolchain.

Exit 0: the source matches the model and every invariant holds. Exit 1: the source broke the property. Exit 2: the probe could not read the functions, the committed config drifted, or TLC failed while the source still matched.

Read every TLC result with `/statechart`. It shows each broken rule as its shortest path in plain words. When the check passes, it draws how the session moves.

## Coverage

| Step | Source |
| --- | --- |
| Choose maths, type an expression, quit | `maths_repl` in `userspace/pds/sddf-shell/src/main.rs` |
| Choose browser, the paint returns | `app_menu` and `browse` in the same file |
| Source a file, source again | `cmd_source` in the same file |

The menu and `source` are different images. A maths step runs only while sourcing is idle, and a source step runs only while the phase is the menu.

The labels a reader sees are in `support/verify/shell-session/view.mjs`. An unknown menu line stays on the menu, so the chart hides it. If you change a step the reader can see, update that file so the labels stay in the reader's words.

## Sync commit

`SYNC_COMMIT`

Move this hash forward only after you re-read the functions above, update the model so each step is still one branch, and re-run the check at the new commit.

## When the covered code changes

Re-run the check. Exit 1 is a real break: the counterexample above is the shape. Exit 2 means the probe lost the function or TLC disagrees with source that still has the branches. Fix the model or the probe. Do not weaken `QuitLeftTheMaths`, `BrowserReturned`, or `NoNestedSource` to make a failure pass.

## Bounds

One menu session. The model does not cover the serial transmit queue, the calculator grammar, or the bytes of the painted page. Those have their own checks or tests.

## Tools

TLC, because the question is whether quit, the browser, and `source` can happen in a bad order. The protection domain does not run on the host, so the probe reads the three branches and the model checks every order of those branches.

Bring this check back in line with later edits through `/maintain-formal-verification-skill`.
