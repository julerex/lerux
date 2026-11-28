----------------------------- MODULE AgentBrowse -----------------------------
(***************************************************************************
One browse, from the shell's notify of the agent through the shell reading
the tool page. One step is one branch in the source.

The shell issues the tool and then waits:

  userspace/pds/sddf-grok/src/main.rs
    issue                         301-303
    answer_browse                 127-135
    notified                      91-105

The agent marks the page, waits for the reply, then stores the outcome:

  userspace/pds/sddf-agent/src/tools.rs   browse 223-228
  userspace/pds/sddf-agent/src/main.rs
    notified                      78-90
    worker                        93-101

The page updates are lerux-sddf:

  userspace/crates/lerux-sddf/src/agent.rs
    agent_mark_browse             94-96
    agent_answer_browse           104-118
    agent_browse_accepted         122-124
    agent_store_outcome           127-134

The wait is microkit_cothread_wait_on_channel and the wake is
microkit_cothread_recv_ntfn in userspace/crates/lerux-cothread/src/lib.rs
483-492 and 504-520. block_running 522-545 consumes a set semaphore.
signal_one 548-570 wakes one waiter, or sticks when nobody is waiting.
A second signal while the bit is set is dropped.

The agent is priority 2 and the shell is priority 1. Notifying the agent
runs its handler before the shell continues. Notifying the shell sets a
bit the shell receives only once it is in microkit recv. A bit that arrives
while a handler is running is not in that handler's channel set.

What this leaves out is listed in
.cursor/skills/verify-agent-browse/SKILL.md.
***************************************************************************)

EXTENDS Integers

CONSTANTS
  NameLen, ExpectOk, DoNotify, BrowseNotifies, WorkerNotifies,
  ReplyState, ReplyLen, FinalState, FinalLen

ASSUME NameLen \in Nat /\ NameLen >= 1
ASSUME ExpectOk \in BOOLEAN
ASSUME DoNotify \in BOOLEAN
ASSUME BrowseNotifies \in BOOLEAN
ASSUME WorkerNotifies \in BOOLEAN
ASSUME ReplyState \in Nat /\ ReplyLen \in Nat
ASSUME FinalState \in Nat /\ FinalLen \in Nat

VARIABLES
  state, resultLen,
  shellPhase, shellBlocked, shellToolSem,
  agentPhase, agentBlocked, agentToolSem, agentBrowseSem,
  ntfnAgentTool, ntfnAgentBrowse, ntfnShellBrowse, ntfnShellTool,
  seenAgentTool, seenAgentBrowse, seenShellBrowse, seenShellTool,
  resume, run

vars == <<
  state, resultLen,
  shellPhase, shellBlocked, shellToolSem,
  agentPhase, agentBlocked, agentToolSem, agentBrowseSem,
  ntfnAgentTool, ntfnAgentBrowse, ntfnShellBrowse, ntfnShellTool,
  seenAgentTool, seenAgentBrowse, seenShellBrowse, seenShellTool,
  resume, run
>>

Init ==
  /\ state = 0
  /\ resultLen = 0
  /\ shellPhase = "notify"
  /\ shellBlocked = FALSE
  /\ shellToolSem = FALSE
  /\ agentPhase = "tool"
  /\ agentBlocked = TRUE
  /\ agentToolSem = FALSE
  /\ agentBrowseSem = FALSE
  /\ ntfnAgentTool = FALSE
  /\ ntfnAgentBrowse = FALSE
  /\ ntfnShellBrowse = FALSE
  /\ ntfnShellTool = FALSE
  /\ seenAgentTool = FALSE
  /\ seenAgentBrowse = FALSE
  /\ seenShellBrowse = FALSE
  /\ seenShellTool = FALSE
  /\ resume = "none"
  /\ run = "shellW"

\* issue notifies the agent tool channel. The agent runs before the next
\* statement, which is the wait.
ShellNotify ==
  /\ run = "shellW"
  /\ shellPhase = "notify"
  /\ ~shellBlocked
  /\ ntfnAgentTool' = TRUE
  /\ shellPhase' = "wait"
  /\ resume' = "shellW"
  /\ run' = "agentEnter"
  /\ UNCHANGED <<
       state, resultLen, shellBlocked, shellToolSem,
       agentPhase, agentBlocked, agentToolSem, agentBrowseSem,
       ntfnAgentBrowse, ntfnShellBrowse, ntfnShellTool,
       seenAgentTool, seenAgentBrowse, seenShellBrowse, seenShellTool
     >>

\* Agent notified snapshots the bits that are pending at recv.
AgentEnter ==
  /\ run = "agentEnter"
  /\ seenAgentTool' = ntfnAgentTool
  /\ seenAgentBrowse' = ntfnAgentBrowse
  /\ ntfnAgentTool' = FALSE
  /\ ntfnAgentBrowse' = FALSE
  /\ run' = "agentBody"
  /\ UNCHANGED <<
       state, resultLen, shellPhase, shellBlocked, shellToolSem,
       agentPhase, agentBlocked, agentToolSem, agentBrowseSem,
       ntfnShellBrowse, ntfnShellTool,
       seenShellBrowse, seenShellTool, resume
     >>

\* recv_ntfn on the agent tool channel. The tool bit is checked first.
AgentRecvTool ==
  /\ run = "agentBody"
  /\ seenAgentTool
  /\ seenAgentTool' = FALSE
  /\ IF agentToolSem
     THEN UNCHANGED <<agentBlocked, agentPhase, agentToolSem, run>>
     ELSE IF agentBlocked /\ agentPhase = "tool"
          THEN /\ agentBlocked' = FALSE
               /\ agentPhase' = "mark"
               /\ run' = "agentW"
               /\ UNCHANGED agentToolSem
          ELSE /\ agentToolSem' = TRUE
               /\ UNCHANGED <<agentBlocked, agentPhase, run>>
  /\ UNCHANGED <<
       state, resultLen, shellPhase, shellBlocked, shellToolSem,
       agentBrowseSem, ntfnAgentTool, ntfnAgentBrowse,
       ntfnShellBrowse, ntfnShellTool, seenAgentBrowse,
       seenShellBrowse, seenShellTool, resume
     >>

\* recv_ntfn on the agent browse channel, after the tool bit.
AgentRecvBrowse ==
  /\ run = "agentBody"
  /\ ~seenAgentTool
  /\ seenAgentBrowse
  /\ seenAgentBrowse' = FALSE
  /\ IF agentBrowseSem
     THEN UNCHANGED <<agentBlocked, agentPhase, agentBrowseSem, run>>
     ELSE IF agentBlocked /\ agentPhase = "bwait"
          THEN /\ agentBlocked' = FALSE
               /\ agentPhase' = "check"
               /\ run' = "agentW"
               /\ UNCHANGED agentBrowseSem
          ELSE /\ agentBrowseSem' = TRUE
               /\ UNCHANGED <<agentBlocked, agentPhase, run>>
  /\ UNCHANGED <<
       state, resultLen, shellPhase, shellBlocked, shellToolSem,
       agentToolSem, ntfnAgentTool, ntfnAgentBrowse,
       ntfnShellBrowse, ntfnShellTool, seenAgentTool,
       seenShellBrowse, seenShellTool, resume
     >>

\* The agent handler returns to microkit recv. A still-pending agent bit
\* is delivered before a lower-priority shell resumes.
AgentLeave ==
  /\ run = "agentBody"
  /\ ~seenAgentTool
  /\ ~seenAgentBrowse
  /\ IF ntfnAgentTool \/ ntfnAgentBrowse
     THEN /\ run' = "agentEnter"
          /\ UNCHANGED resume
     ELSE IF resume = "shellW"
          THEN /\ run' = "shellW"
               /\ resume' = "none"
          ELSE IF resume = "shellBody"
               THEN /\ run' = "shellBody"
                    /\ resume' = "none"
               ELSE /\ run' = "shellEnter"
                    /\ UNCHANGED resume
  /\ UNCHANGED <<
       state, resultLen, shellPhase, shellBlocked, shellToolSem,
       agentPhase, agentBlocked, agentToolSem, agentBrowseSem,
       ntfnAgentTool, ntfnAgentBrowse, ntfnShellBrowse, ntfnShellTool,
       seenAgentTool, seenAgentBrowse, seenShellBrowse, seenShellTool
     >>

\* agent_mark_browse.
AgentMark ==
  /\ run = "agentW"
  /\ agentPhase = "mark"
  /\ ~agentBlocked
  /\ state' = 1
  /\ agentPhase' = "signal"
  /\ UNCHANGED <<
       resultLen, shellPhase, shellBlocked, shellToolSem,
       agentBlocked, agentToolSem, agentBrowseSem,
       ntfnAgentTool, ntfnAgentBrowse, ntfnShellBrowse, ntfnShellTool,
       seenAgentTool, seenAgentBrowse, seenShellBrowse, seenShellTool,
       resume, run
     >>

\* browse notifies the shell. The shell is lower priority, so the bit waits.
AgentSignal ==
  /\ run = "agentW"
  /\ agentPhase = "signal"
  /\ ~agentBlocked
  /\ IF BrowseNotifies THEN ntfnShellBrowse' = TRUE ELSE UNCHANGED ntfnShellBrowse
  /\ agentPhase' = "bwait"
  /\ UNCHANGED <<
       state, resultLen, shellPhase, shellBlocked, shellToolSem,
       agentBlocked, agentToolSem, agentBrowseSem,
       ntfnAgentTool, ntfnAgentBrowse, ntfnShellTool,
       seenAgentTool, seenAgentBrowse, seenShellBrowse, seenShellTool,
       resume, run
     >>

\* wait on the browse channel. A set bit returns. Otherwise the worker
\* switches back to the agent root, still inside notified.
AgentWaitBrowse ==
  /\ run = "agentW"
  /\ agentPhase = "bwait"
  /\ ~agentBlocked
  /\ IF agentBrowseSem
     THEN /\ agentBrowseSem' = FALSE
          /\ agentPhase' = "check"
          /\ UNCHANGED <<agentBlocked, run>>
     ELSE /\ agentBlocked' = TRUE
          /\ run' = "agentBody"
          /\ UNCHANGED <<agentBrowseSem, agentPhase>>
  /\ UNCHANGED <<
       state, resultLen, shellPhase, shellBlocked, shellToolSem,
       agentToolSem, ntfnAgentTool, ntfnAgentBrowse,
       ntfnShellBrowse, ntfnShellTool, seenAgentTool, seenAgentBrowse,
       seenShellBrowse, seenShellTool, resume
     >>

\* agent_browse_accepted. The outcome itself is applied by AgentStore.
AgentCheck ==
  /\ run = "agentW"
  /\ agentPhase = "check"
  /\ ~agentBlocked
  /\ agentPhase' = "store"
  /\ UNCHANGED <<
       state, resultLen, shellPhase, shellBlocked, shellToolSem,
       agentBlocked, agentToolSem, agentBrowseSem,
       ntfnAgentTool, ntfnAgentBrowse, ntfnShellBrowse, ntfnShellTool,
       seenAgentTool, seenAgentBrowse, seenShellBrowse, seenShellTool,
       resume, run
     >>

\* agent_store_outcome, probed from the real function.
AgentStore ==
  /\ run = "agentW"
  /\ agentPhase = "store"
  /\ ~agentBlocked
  /\ state' = FinalState
  /\ resultLen' = FinalLen
  /\ agentPhase' = "tsignal"
  /\ UNCHANGED <<
       shellPhase, shellBlocked, shellToolSem,
       agentBlocked, agentToolSem, agentBrowseSem,
       ntfnAgentTool, ntfnAgentBrowse, ntfnShellBrowse, ntfnShellTool,
       seenAgentTool, seenAgentBrowse, seenShellBrowse, seenShellTool,
       resume, run
     >>

\* The worker notifies the shell tool channel, then loops back to the wait.
AgentSignalTool ==
  /\ run = "agentW"
  /\ agentPhase = "tsignal"
  /\ ~agentBlocked
  /\ IF WorkerNotifies THEN ntfnShellTool' = TRUE ELSE UNCHANGED ntfnShellTool
  /\ agentPhase' = "tool"
  /\ UNCHANGED <<
       state, resultLen, shellPhase, shellBlocked, shellToolSem,
       agentBlocked, agentToolSem, agentBrowseSem,
       ntfnAgentTool, ntfnAgentBrowse, ntfnShellBrowse,
       seenAgentTool, seenAgentBrowse, seenShellBrowse, seenShellTool,
       resume, run
     >>

AgentWaitTool ==
  /\ run = "agentW"
  /\ agentPhase = "tool"
  /\ ~agentBlocked
  /\ IF agentToolSem
     THEN /\ agentToolSem' = FALSE
          /\ agentPhase' = "mark"
          /\ UNCHANGED <<agentBlocked, run>>
     ELSE /\ agentBlocked' = TRUE
          /\ run' = "agentBody"
          /\ UNCHANGED <<agentToolSem, agentPhase>>
  /\ UNCHANGED <<
       state, resultLen, shellPhase, shellBlocked, shellToolSem,
       agentBrowseSem, ntfnAgentTool, ntfnAgentBrowse,
       ntfnShellBrowse, ntfnShellTool, seenAgentTool, seenAgentBrowse,
       seenShellBrowse, seenShellTool, resume
     >>

\* The shell worker reaches its wait only after the agent handler has returned.
\* Both branches prime shellPhase: the set-bit path moves to "read", and the
\* blocking path keeps "wait".
ShellWaitOk ==
  /\ run = "shellW"
  /\ shellPhase = "wait"
  /\ ~shellBlocked
  /\ IF shellToolSem
     THEN /\ shellToolSem' = FALSE
          /\ shellPhase' = "read"
          /\ UNCHANGED <<shellBlocked, run>>
     ELSE /\ shellBlocked' = TRUE
          /\ run' = "shellEnter"
          /\ shellPhase' = shellPhase
          /\ UNCHANGED shellToolSem
  /\ UNCHANGED <<
       state, resultLen, agentPhase, agentBlocked,
       agentToolSem, agentBrowseSem,
       ntfnAgentTool, ntfnAgentBrowse, ntfnShellBrowse, ntfnShellTool,
       seenAgentTool, seenAgentBrowse, seenShellBrowse, seenShellTool,
       resume
     >>

ShellEnter ==
  /\ run = "shellEnter"
  /\ ntfnShellBrowse \/ ntfnShellTool
  /\ seenShellBrowse' = ntfnShellBrowse
  /\ seenShellTool' = ntfnShellTool
  /\ ntfnShellBrowse' = FALSE
  /\ ntfnShellTool' = FALSE
  /\ run' = "shellBody"
  /\ UNCHANGED <<
       state, resultLen, shellPhase, shellBlocked, shellToolSem,
       agentPhase, agentBlocked, agentToolSem, agentBrowseSem,
       ntfnAgentTool, ntfnAgentBrowse,
       seenAgentTool, seenAgentBrowse, resume
     >>

\* answer_browse. The probed reply is the page after agent_answer_browse.
\* DoNotify is that call returning true and the shell still notifying.
ShellAnswer ==
  /\ run = "shellBody"
  /\ seenShellBrowse
  /\ seenShellBrowse' = FALSE
  /\ state' = ReplyState
  /\ resultLen' = ReplyLen
  /\ IF DoNotify
     THEN /\ ntfnAgentBrowse' = TRUE
          /\ resume' = "shellBody"
          /\ run' = "agentEnter"
     ELSE UNCHANGED <<ntfnAgentBrowse, resume, run>>
  /\ UNCHANGED <<
       shellPhase, shellBlocked, shellToolSem,
       agentPhase, agentBlocked, agentToolSem, agentBrowseSem,
       ntfnAgentTool, ntfnShellBrowse, ntfnShellTool,
       seenAgentTool, seenAgentBrowse, seenShellTool
     >>

ShellRecvTool ==
  /\ run = "shellBody"
  /\ ~seenShellBrowse
  /\ seenShellTool
  /\ seenShellTool' = FALSE
  /\ IF shellToolSem
     THEN UNCHANGED <<shellBlocked, shellPhase, shellToolSem, run>>
     ELSE IF shellBlocked /\ shellPhase = "wait"
          THEN /\ shellBlocked' = FALSE
               /\ shellPhase' = "read"
               /\ run' = "shellW"
               /\ UNCHANGED shellToolSem
          ELSE /\ shellToolSem' = TRUE
               /\ UNCHANGED <<shellBlocked, shellPhase, run>>
  /\ UNCHANGED <<
       state, resultLen, agentPhase, agentBlocked,
       agentToolSem, agentBrowseSem,
       ntfnAgentTool, ntfnAgentBrowse, ntfnShellBrowse, ntfnShellTool,
       seenAgentTool, seenAgentBrowse, seenShellBrowse, resume
     >>

ShellLeave ==
  /\ run = "shellBody"
  /\ ~seenShellBrowse
  /\ ~seenShellTool
  /\ run' = "shellEnter"
  /\ UNCHANGED <<
       state, resultLen, shellPhase, shellBlocked, shellToolSem,
       agentPhase, agentBlocked, agentToolSem, agentBrowseSem,
       ntfnAgentTool, ntfnAgentBrowse, ntfnShellBrowse, ntfnShellTool,
       seenAgentTool, seenAgentBrowse, seenShellBrowse, seenShellTool,
       resume
     >>

ShellRead ==
  /\ run = "shellW"
  /\ shellPhase = "read"
  /\ ~shellBlocked
  /\ shellPhase' = "done"
  /\ UNCHANGED <<
       state, resultLen, shellBlocked, shellToolSem,
       agentPhase, agentBlocked, agentToolSem, agentBrowseSem,
       ntfnAgentTool, ntfnAgentBrowse, ntfnShellBrowse, ntfnShellTool,
       seenAgentTool, seenAgentBrowse, seenShellBrowse, seenShellTool,
       resume, run
     >>

Terminated == shellPhase = "done"

Park == Terminated /\ UNCHANGED vars

Next ==
  \/ Park
  \/ ShellNotify
  \/ AgentEnter
  \/ AgentRecvTool
  \/ AgentRecvBrowse
  \/ AgentLeave
  \/ AgentMark
  \/ AgentSignal
  \/ AgentWaitBrowse
  \/ AgentCheck
  \/ AgentStore
  \/ AgentSignalTool
  \/ AgentWaitTool
  \/ ShellWaitOk
  \/ ShellEnter
  \/ ShellAnswer
  \/ ShellRecvTool
  \/ ShellLeave
  \/ ShellRead

Spec == Init /\ [][Next]_vars /\ WF_vars(Next)

\* A blocked shell whose agent is also blocked, with both protection domains
\* in recv and no bit pending, never wakes.
NotStuck ==
  \/ shellPhase = "done"
  \/ run = "shellW" /\ ~shellBlocked
  \/ run = "agentW" /\ ~agentBlocked
  \/ run = "agentEnter"
  \/ run = "agentBody"
  \/ run = "shellBody"
  \/ run = "shellEnter" /\ (ntfnShellBrowse \/ ntfnShellTool)

\* A finished browse of a name that fits is state ok and that length.
\* A name that does not fit is state err and a zero result length.
Result ==
  \/ shellPhase # "done"
  \/ IF ExpectOk
     THEN state = 3 /\ resultLen = NameLen
     ELSE state = 4 /\ resultLen = 0

EventuallyDone == <>Terminated

=============================================================================
