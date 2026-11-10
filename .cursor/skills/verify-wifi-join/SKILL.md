---
name: verify-wifi-join
description: >
  Use when changing the Swift wifi menu, Mac::associate, wifi::associate,
  key_ok, or the MT7921 ready flag. The check keeps a failed stage on that
  stage, refuses a short passphrase, and waits for firmware before the scan.
---

# Verify the wifi join

## Property

The join is online only after the handshake finishes, a passphrase `key_ok` rejects never starts the radio, and `ready` stays false until the firmware stage is `Wait`.

## Harm

The panel says the join finished when the scan or the handshake failed, or a short passphrase is written to the radio. The smoke types one good passphrase. The address and the ping belong to the shell.

## Result on this tree

The check passes. TLC finds 16 distinct states. Each failed stage stays on that stage, a short passphrase returns to the menu, and firmware finishes before the menu offers wifi.

## Counterexample the check catches

In `Mac::associate`, change both `return Stage::Scan` lines to `return Stage::Wait`. The check exits 1.

1. The firmware reaches `Wait`.
2. You choose wifi, name a network, and type a passphrase.
3. The scan fails, and the model reports the radio online.
4. `OnlineIsFinished` fails.

`wifi::associate` without `nic.rx() != Some(&[1])` reports the handshake as online. `dispatch_psk` calling `post_wifi` before `key_ok` starts the radio on a short passphrase (`ShortKeyStays`). Removing `mac.stage == Stage::Wait` reports online before the firmware is ready. `key_ok(b"net", b"short")` returning true also exits 1, from the host call, even when the call site is unchanged.

## Run

```bash
support/verify/wifi-join/check.sh
```

The script uses TLC 1.7.4. It downloads `tla2tools.jar` into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc/` when that file is missing. The sha256 is `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`. Java 11 or newer must be on `PATH`. The script changes to the repo root before Cargo so rustup selects this repo's toolchain.

Exit 0: the source matches the model and every invariant holds. Exit 1: the source broke the property. Exit 2: the probe could not read the functions, the committed config drifted, or TLC failed while the source still matched.

Read every TLC result with `/statechart`. It shows each broken rule as its shortest path in plain words. When the check passes, it draws how the join moves.

## Coverage

| Step | Source |
| --- | --- |
| Menu, network name, passphrase, quit | `dispatch_ssid`, `dispatch_psk`, `post_wifi` in `userspace/pds/console-shell/src/main.rs` |
| Passphrase length | `key_ok` in `userspace/crates/lerux-swift/src/wpa.rs` |
| Scan, auth, assoc, handshake | `Mac::associate` in `userspace/pds/swift-mt7921/src/join.rs` |
| Handshake reply must be `[1]` | `associate` in `userspace/crates/lerux-swift/src/wifi.rs` |
| Firmware gate | `mac.stage == Stage::Wait` in `userspace/pds/swift-mt7921/src/device.rs` |
| Absent radio, missing firmware, empty passphrase, silent handshake | `bring_up` in `userspace/crates/lerux-swift/src/wifi.rs` |

The four-way handshake inside `four_way` is one stage here (`Eapol`), not the four messages. The firmware words `own` through `mac` are the `Wait` gate, not a stage of their own.

The labels a reader sees are in `support/verify/wifi-join/view.mjs`. The chart hides the step that stays on a stopped stage or on "Handshake finished". If you change a step the reader can see, update that file so the labels stay in the reader's words.

## Sync commit

`f4eef172662ea7db373f261516e5755fc28409ab`

Move this hash forward only after you re-read the functions above, update the model so each step is still one branch, and re-run the check at the new commit.

## When the covered code changes

Re-run the check. Exit 1 is a real break. Exit 2 means the probe lost the function or TLC disagrees with source that still has the branches. Fix the model or the probe. Do not weaken `OnlineIsFinished` or `ShortKeyStays`.

## Bounds

One network and one passphrase. The model does not cover the WPA message bytes or a second scan. The address and the ping are not steps of `Mac::associate`. `bring_up` is called with a radio that never replies, so the success path of that host function is the crate's own test.

## Tools

TLC, because the question is whether a failed stage can be reported as a finished join. `key_ok` and `bring_up` run on the host. `Mac::associate` does not, so the probe reads each `return Stage::...` and the model checks the order.

Bring this check back in line with later edits through `/maintain-formal-verification-skill`.
