---
name: verify-bt-stage
description: >
  Use when changing the Swift Bluetooth stage line: fail_line, port_line,
  version_line, format_report, bring_up, further, or PortStop::later in
  lerux-swift, or the paint calls in swift-bt bring_up_text. Re-check that
  bt: port N STAGE keeps the furthest of reset, slot, addr, and desc, and
  that bt: reset ok is still only the command-complete line.
---

# Verify the Bluetooth stage line

## Property

`bt: port N STAGE` uses `STAGE` of `reset`, `slot`, `addr`, or `desc`. `N` is the number of ports that reported a connection, and the furthest stage is the one painted. The order is reset, then slot, then addr, then desc. A later failure does not paint an earlier stage as the furthest.

`bt: reset ok` is painted only from a command-complete event after the Bluetooth firmware is present. A port line is not that success. An empty firmware blob is `bt: firmware err`. A capability word outside `0x0100..=0x0120` is `bt: ver` plus that word. An addressed device that is not the HCI interface is `bt: id`.

The 8 November 2026 panel reached `bt: port 3 desc`. That word means the command ring ran and three ports were addressed. It is a finished line. `bt: reset ok` is still the milestone exit and is a later boot.

## Harm

A later port failure paints an earlier stage, so the panel hides how far the command ring got. Or a port line is treated as the reset acknowledgement, and the milestone looks done before the controller answers. The xHCI sequence, the command-ring address, the firmware load, and the reset path are not what this check edits.

## Result on this tree

The check exits 0. TLC finds 142 distinct states and the search depth is 7. One finished line is `OpenPorts`, `SeeReset`, `SeeReset`, `SeeDesc`, `PaintFail`, which paints `bt: port 3 desc`. The success line is a different boot: `OpenPorts`, `AckEvent`, `PaintOk`, which paints `bt: reset ok`. The port line does not take that step.

## Counterexample the check catches

Change `other.rank() > self.rank()` to `other.rank() < self.rank()` in `PortStop::later`. The check exits 1. The Rust schedule prints `verdict=line` and `a later failure replaced a further stage`. TLC reports `Invariant PortLine is violated`. With `OnlyForward` false the steps are:

1. `OpenPorts`. Firmware is present, and nothing is painted yet.
2. `SeeStage` records one connected port at rank 2. Peak is 2.
3. `SeeStage` records a second port at rank 1. Peak stays 2 and the painted rank drops to 1.

That is a later reset painted in place of the further slot. Put the higher rank back in `later`. Do not answer this failure by painting `bt: reset ok`, and do not move the command ring.

Painting `bt_line(&BtReport::ResetOk)` from the success arm, without `bt_bring_up` on the command-complete event, is `verdict=paint` and exit 1. Restore the `bt_bring_up` call. That is the only paint-call edit this check asks for.

## Run

From the repository root:

```bash
support/verify/bt-stage/check.sh
```

Java 11 or newer is required. The first run downloads TLC 1.7.4 (`tla2tools.jar`, sha256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`) into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc`. Later runs reuse that jar. Do not commit the jar.

Exit status:

- **0.** The formatters match the milestone lines, the protection domain calls those formatters, and TLC accepts the stage order.
- **1.** The Rust schedule prints `verdict=line` or `verdict=paint`, or its process exits 1. A formatter disagrees with the milestone, or the protection domain paints a different call. A formatter miss is fixed in `userspace/crates/lerux-swift/src/bt.rs`. A paint miss is fixed only at that call in `swift-bt`.
- **2.** The protection-domain source is not a shape this model can read, the search passes 10000 states, the replay tests fail, the probed constants differ from `boot.cfg`, or the Rust schedule accepts a run that TLC rejects. Treat that as a modeling error until you can replay the trace on the real formatters. A missing `further` in `reset_either`, or a `find_radio` that no longer names the stages in order, is exit 2. Do not invent a reset-ok path to satisfy it.

Read every TLC result with `/statechart`. It shows each invariant violation as its shortest path in plain words. When the check passes, `/statechart` draws how a boot moves from the open line to a port stage or to reset ok. Its commands and paths change between releases, so they are not copied here.

## Coverage

| Step | Source |
| --- | --- |
| out-of-range capability word, empty blob, then the transfer | `swift-bt` `bring_up_text` |
| the two controllers keep the further failure | `swift-bt` `reset_either` |
| capability range `0x0100..=0x0120` | `swift-bt` `controllers_live` |
| stage names in order reset, slot, addr, desc | `swift-bt` `find_radio` |
| `PortStop::later`, `fail_line`, `further`, `bring_up`, `format_report`, `version_line` | `lerux-swift` `bt.rs` |

Line numbers are the comment at the top of `support/verify/bt-stage/BtStage.tla`. `SeeStage(r)` is one TLA action. The Rust steps `SeeReset`, `SeeSlot`, `SeeAddr`, and `SeeDesc` are that action at ranks 1 through 4. The Rust twin is `support/verify/bt-stage/replay`. It calls the formatters and reads which call the protection domain makes. The TLA module does not execute Rust, and it does not execute xHCI. The two have to change together when a branch changes. `boot.cfg` is the probed constants at the sync commit. The check regenerates it from the functions and the source, and exits 2 when the committed file has fallen behind.

Sync commit: `c7ba3dc5404dc80491ca978e358279b1892dd0d3`

Move that hash forward only after you have re-read the covered functions at the new commit, updated both models so each step is still one branch, and re-run this check.

## When the covered code changes

1. Read the diff of the files in the coverage table.
2. If a branch was added, removed, or reordered, change that action in `BtStage.tla` and the function of the same name in `replay/src/main.rs`. Keep `PaintFail` as a finished port line. Desc is not a step that must continue into `PaintOk`.
3. If only the body of a formatter changed, the Rust checker already calls it. Change the TLA action when the visible effect changed: which stage is kept, or which line is selected. Update `boot.cfg` from the new probe only after that effect is what you intend.
4. Re-run the check. On exit 2, replay the trace as calls on the real formatters. A step you cannot replay is a modeling error. Fix the model. Leave `PortLine` and `ResetOkSeparate` as they are.
5. Update the sync commit only after that run.

## Bounds and what is left out

- One boot. At most three connected ports, which is the 8 November line. The Rust twin also asks the formatter for a count of 1 and a count of 12.
- Ranks are 1 reset, 2 slot, 3 addr, and 4 desc. Rank 0 is a count with no stage word (`bt: port 0`).
- `AckEvent` is the command-complete event. A stage word does not set it. `PaintOk` is enabled only after that event, and only when firmware is present.
- A second controller may still acknowledge after the first returned a port failure. The painted line is then `bt: reset ok`. The November boot is the one that stops at `PaintFail`.
- The search is the reachable paint choices, 142 states. The explorer stops at 10000 states and exits 2.
- Left out: xHCI register writes, the command ring at `0x1f20000`, firmware bytes, the Wi-Fi download, and the personal-area-network profile. This check is not part of `support/verify/implicit/`.

## Tools

TLC 1.7.4 checks that the furthest stage only moves forward and that reset ok stays on the command-complete path. The Rust program runs those steps on the real formatters and reads the paint sites from source, so a formatter that disagrees with the milestone fails the check before anyone edits the TLA actions.

After a change that did not re-run this check, use `/maintain-formal-verification-skill`.
