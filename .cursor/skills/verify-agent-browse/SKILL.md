---
name: verify-agent-browse
description: >
  Use when changing the agent browse handshake: sddf-grok answer_browse, issue,
  or notified; sddf-agent browse, worker, or notified; agent_mark_browse,
  agent_answer_browse, agent_browse_accepted, or agent_store_outcome; or the
  cothread wait and recv those channels use. Re-check that a browse finishes
  with the name or with err, and that neither worker stays blocked with no wake.
---

# Verify the agent browse handshake

## Property

A browse whose name fits in the tool page finishes in state ok with that name as the result. A name that does not fit finishes in state err with a zero result length. Neither the shell worker nor the agent worker is left blocked with no notification still pending.

## Harm

The shell worker stays inside `microkit_cothread_wait_on_channel` on the tool channel, so the grok command loop never prints a result. `just test-agent-sddf` sends `grok write` and a short read. It never sends `browse`.

## Result on this tree

The check exits 0. A name of length 4 finishes in state ok. A name of length 65 finishes in state err. Neither schedule sticks.

## Counterexample the check catches

Delete `Channel::new(BROWSE_CH).notify()` in `sddf-grok` `answer_browse` and leave the `agent_answer_browse` call. The check exits 1. The Rust schedule prints `verdict=stuck` and TLC reports `Invariant NotStuck is violated`, on both the fitting name and the long name. The page is already browse-done. The steps are:

1. `issue` notifies the agent tool channel. The agent is priority 2, so its handler runs before the shell worker reaches its wait.
2. The agent `recv_ntfn` on the tool channel wakes its worker.
3. `agent_mark_browse` sets state browse.
4. The agent notifies the shell browse channel. The shell is priority 1, so that bit waits.
5. The agent waits on the browse channel. The semaphore is clear, so the worker blocks.
6. The agent handler returns. The shell worker then blocks on the tool channel. The shell root receives the browse bit.
7. `agent_answer_browse` writes browse-done. The shell does not notify the agent.
8. The shell handler returns to recv. No shell bit is pending. The agent is still blocked on the browse channel, and the shell worker is still blocked on the tool channel.

A fitting name already has result length 4 and the bytes `host`. The agent never stores ok. A name of length 65 has result length 0. The stuck schedule is the same. That trace also fails `EventuallyDone`. `Result` stays true on it, because the shell never reaches the read. Do not treat a passing `Result` as evidence that the browse finished.

## Run

From the repository root:

```bash
support/verify/agent-browse/check.sh
```

Java 11 or newer is required. The first run downloads TLC 1.7.4 (`tla2tools.jar`, sha256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`) into `${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc`. Later runs reuse that jar.

Exit status:

- **0.** Both checkers accept the fitting name and the long name. The shell reads state ok or state err, and a wake is still scheduled whenever a worker is blocked.
- **1.** The Rust schedule prints `verdict=stuck` or `verdict=result`, or its process exits 1. The shell is blocked with no wake, or the finished page has the wrong state, length, or bytes. This is the code, including when TLC fails as well.
- **2.** The protection-domain source is not a shape this model can read, the search passes 10000 states, the replay tests fail, the probed constants differ from `fits.cfg` or `reject.cfg`, or the Rust schedule accepts a run that TLC rejects. Treat that as a modeling error until you can replay the trace on the real functions. A lost notify that the scanner still sees as present is a scanner bug, not a reason to weaken `NotStuck`.

`verdict=result` is the Rust checker only. TLC sees the state number and the length. A copy that keeps the length and changes the bytes is exit 1 even when TLC still passes.

Reordering `sddf-grok` `notified` so `microkit_cothread_recv_ntfn(TOOL_CH)` precedes `answer_browse()` is exit 2. The scanner refuses that order. One browse does not deliver the browse bit and the tool bit in the same shell handler, because the tool completion is sent from inside the agent's browse handler, which the shell notify runs before that handler looks at the tool channel. Update both models if that order is what you intend. Do not treat the exit 2 as `NotStuck`.

Read every TLC result with `/statechart`. It shows each invariant violation as its shortest path in plain words. When the check passes, `/statechart` draws how the shell and the agent move. Its commands and paths change between releases, so they are not copied here.

## Coverage

| Step | Source |
| --- | --- |
| `issue` notifies, waits, and reads state ok | `sddf-grok` `issue` |
| `answer_browse`, then the browse notify | `sddf-grok` `answer_browse` |
| browse answer before the tool `recv_ntfn` | `sddf-grok` `notified` |
| mark, notify, wait, then accepted | `sddf-agent` `browse` |
| tool `recv_ntfn`, then browse `recv_ntfn` | `sddf-agent` `notified` |
| wait, `agent_store_outcome`, tool notify | `sddf-agent` `worker` |
| `agent_mark_browse`, `agent_answer_browse`, `agent_browse_accepted`, `agent_store_outcome` | `lerux-sddf` `agent.rs` |
| sticky wait and the root's `recv_ntfn` | `lerux-cothread` `microkit_cothread_wait_on_channel`, `microkit_cothread_recv_ntfn`, `block_running`, `signal_one` |

Line numbers are the comment at the top of `support/verify/agent-browse/AgentBrowse.tla`. One TLA action is one branch in that source. The Rust twin is `support/verify/agent-browse/replay`. It calls the four page functions and reads the notify sites from the protection-domain source. The TLA module does not execute Rust. The two have to change together when a branch changes. `fits.cfg` and `reject.cfg` are the probed constants at the sync commit. The check regenerates them from the functions and the source, and exits 2 when the committed files have fallen behind.

Sync commit: `1a924e4f71bfe996e2a678af618b0ba12a095d19`

Move that hash forward only after you have re-read the covered functions at the new commit, updated both models so each step is still one branch, and re-run this check.

## When the covered code changes

1. Read the diff of the files in the coverage table.
2. If a branch was added, removed, or reordered, change that action in `AgentBrowse.tla` and the function of the same name in `replay/src/main.rs`. Keep `ShellAnswer` as the page update and the notify that follows it. The agent runs before the shell handler continues.
3. If only the body of `agent_answer_browse` or `agent_store_outcome` changed, the Rust checker already calls it. Change the TLA action when the visible effect changed: a state stored, a length stored, or a return value that selects the other branch. Update `fits.cfg` and `reject.cfg` from the new probe only after that effect is what you intend.
4. Re-run the check. On exit 2, replay the trace as calls on the real page functions. A step you cannot replay is a modeling error. Fix the model. Leave `NotStuck` and `Result` as they are.
5. Update the sync commit only after that run.

## Bounds and what is left out

- One browse. One shell and one agent. The agent priority is 2 and the shell priority is 1. Notifying the agent runs its handler before the caller continues. Notifying the shell sets a bit the shell receives only from microkit recv.
- The fitting name is `host` (length 4). The other name has `name_len` 65 and does not fit in the 64-byte name field. `issue` returns before the notify when the shell itself passes a longer name, so 65 is the branch inside `agent_answer_browse`.
- A handler's channel set is the bits pending when `notified` starts. A bit that arrives during the handler waits for the next recv.
- Both configs leave the tool page zeroed, which is what Microkit does to a shared page.
- The search is the reachable schedules of this one browse, 22 states when the notifies are present. The explorer stops at 10000 states and exits 2.
- Left out: serial transmit and receive, the filesystem, the network, `fetch`, the other tool kinds, a second browse in the same boot, and a name `issue` rejects before it notifies.

## Tools

TLC 1.7.4 checks the order of the protection-domain steps. The question is whether a browse can leave a worker blocked with no wake scheduled. The Rust program runs those steps on the real page functions and reads the notify sites from source, so a deleted notify fails the check before anyone edits the TLA actions.

After a change that did not re-run this check, use `/maintain-formal-verification-skill`.
