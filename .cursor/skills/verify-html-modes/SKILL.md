---
name: verify-html-modes
description: >
  Use when changing the Ladybird tree builder's title arm, take_raw_text, or
  Document::parse. The check leaves the title mode after the title text, and
  keeps raw text such as a<b>c inside the title.
---

# Verify HTML modes

## Property

After the title text is taken, the builder is not left in the title, and the title's raw text is kept as text.

## Harm

The next tag is swallowed as the title, or `<title>a<b>c</title>` becomes an element instead of the text `a<b>c`. The document tests cover one fixture, not every order of tokens.

## Result on this tree

The check passes. TLC finds 2 distinct states. The title arm takes raw text and pops the open element, so the mode returns to data. `Document::parse("<title>Hi</title>")` yields `Hi`, and `Document::parse("<title>a<b>c</title>")` yields `a<b>c`.

## Counterexample the check catches

Remove `self.open.pop()` from the `InsertionMode::Title` arm in `finish`. The check exits 1.

1. The builder sees the title.
2. The arm takes the text and does not pop the element.
3. `TitleClosed` fails: the text is `Hi` and the mode is still title.

Removing `take_raw_text` from that arm is the same exit, and the host parse of `a<b>c` fails when the inner tag is no longer raw text.

## Run

```bash
support/verify/html-modes/check.sh
```

The script uses TLC 1.7.4. It downloads `tla2tools.jar` into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc/` when that file is missing. The sha256 is `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`. Java 11 or newer must be on `PATH`. The script changes to the repo root before Cargo so rustup selects this repo's toolchain.

Exit 0: the source matches the model and the invariant holds. Exit 1: the source broke the property. Exit 2: the probe could not read the title arm, the committed config drifted, or TLC failed while the source still matched.

Read every TLC result with `/statechart`. It shows a broken rule as its shortest path in plain words. When the check passes, it draws how the mode moves.

## Coverage

| Step | Source |
| --- | --- |
| Title text, then leave the title | `TreeBuilder::finish` in `lerux-ladybird/crates/lerux-ladybird-html/src/document/parse.rs` |
| Title bytes | `Document::parse` and `title_text` in `lerux-ladybird/crates/lerux-ladybird-html/src/document.rs` |

The tokenizer has no state enum. The cursor and `take_raw_text` are the title's raw-text mode. Other insertion modes are outside this check.

## Sync commit

`e599121d159bd050d2e69547eb42bfcc7deb791b`

Move this hash forward only after you re-read the title arm, update the model so the step is still that arm, and re-run the check at the new commit.

## When the covered code changes

Re-run the check. Exit 1 is a real break. Exit 2 means the probe lost the arm or TLC disagrees with source that still pops it. Fix the model or the probe. Do not weaken `TitleClosed`.

## Bounds

One title in one document. The model does not cover the rest of the insertion modes, comments, or a second title.

## Tools

TLC, because the question is whether the builder can stay in the title after taking the text. `Document::parse` runs on the host for the title bytes. The mode itself is the open-element stack, so the probe reads the pop.

Bring this check back in line with later edits through `/maintain-formal-verification-skill`.
