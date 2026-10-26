---
name: port-and-smoke
description: >
  Use when porting a protection domain to another architecture, changing a
  system description, combining virtio devices on one image, porting a context
  switch, or judging a QEMU smoke. When adding or changing an interrupt on a
  board smoke, state whether the guest proof exercised that interrupt or only
  polling and log lines. Read the serial log and finish both host checks before
  calling the work done.
---

# Port and smoke

Rules for a cross-architecture port and for a QEMU smoke in this repository. The same lessons are proposed upstream in the workflow skills. This file is the copy that applies here.

## Proof

A smoke is done when the serial log contains the expected lines. Read that file. A delegate summary, a completion notice, and a shell pipeline whose last command is `tail` or `echo` are not the result. The recipe status line is.

A firmware port is done when that smoke is green and `just check` and `just check-pd` have both exited zero on the same tree. Mark the plan, or call the branch finished, only after both.

A claim that a smoke passed, that a serial capture is intact, or that a reboot reloaded the guest is checked against the log file or the harness transcript.

After a change to how QEMU is assembled, run the command-line interface tests for that board before the guest smoke.

## Sequence

Before scheduling a role on a new architecture, find the compile-time gates and which protection domains link the dependency. Order the units behind that proof.

The first unit ends with host tests that compare the rendered system description, including the architecture that must stay unchanged. Those tests assert the program name, the regions, and the interrupt element. The next unit's guest smoke starts after those tests are green.

Land each green unit as its own commit or pull request. One branch at the end is the shape only when the operator has said to wait.

## Machine description

A queue or config struct with no address or transport field stays that way. The architecture lives in the machine description and the device backend.

Constants used only to build the system description stay behind the feature the protection-domain build does not enable. The default build of a shared crate is the one `just check-pd` compiles.

An image that combines devices on one bus has its own interrupt and scheduling tables. The standalone image of one of those devices is a different machine.

When the machine table cannot express a rule that depends on that combination, write the rule as an invariant of the image kind.

The shared grounding for a design names those feature splits and the invariants that are already true, including what the renderer emits and which board metadata is stale.

Registration of a backend is not proof the image uses it. Trace the function that renders the board until the program name, the regions, and the interrupt element match that backend.

An x86 message-signaled interrupt element carries the PCI address, a handle equal to the guest vector plus 0x30, and that guest vector. A QEMU tree listing does not supply an I/O APIC pin for the stock controller.

Shared queue memory with no physical address is not a device direct-memory-access target. Copy through a driver-owned window that has a fixed physical address.

When the protection domain uses compile-time addresses, map the regions directly. A setvar patch for a symbol the program does not define fails the image step.

## Context switch

A context-switch port matches the recorded upstream register layout. Stop when that layout cannot be found. The proof is the project's canonical log lines.

## Failed premises

When the investigation moves to a new premise, remove the edits that only served the discarded premise before the next smoke.

## Exploration

Reconcile an explorer claim about how an image is generated against the function that writes it. A template name in board metadata is a claim until that path is traced.

Harvest each explorer's final report into one scratch file and point the explainer at that file.

When the unknown is operational, probe it on the host before a large delegated smoke. Reset behavior, interrupt routing, and capture plumbing are that kind of unknown. A QEMU device probe is a bounded headless machine (`-machine q35 -nodefaults -S`), then monitor `info qtree` and `quit`. Bare `-device <name>,help` can boot a full virtual machine and hang.

Jobs that share one build-driver target directory run one at a time. Host-only tests use `just test-host <crate>` or `just check-host <crate>`. Both set `CARGO_TARGET_DIR` to `build/host` and do not start `just check-pd`.

Before a Task spawn, run `python3 support/check-spawn.py` with the role, the subagent type, and the prompt. A non-zero exit is a refusal. Do not spawn.

Before the numbered plan skill does any other work, run `python3 support/check-plan-owner.py --skills` with every attached skill name. If poteto-mode is one of them, the script exits non-zero and names the design doc.

A change that touches a shared filesystem config writer is not done until `python3 support/check-sibling-smoke.py` exits zero. That requires `ci = true` on the new board, or a sibling `just test-...` recipe named in the test plan.

A long run keeps its decision trail. Skipping the trail takes an explicit waiver from the operator.

When board metadata is stale relative to the generator, leave an issue, a plan note, or a decision record.
