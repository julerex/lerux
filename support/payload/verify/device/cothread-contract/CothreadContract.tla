----------------------------- MODULE CothreadContract -----------------------------
(***************************************************************************
The cothread contract for one kernel thread. One step is one branch in
userspace/crates/lerux-cothread/src/lib.rs. The module does not execute
Rust. A contract break is a call that panics, so it has no next state.
Those actions are in Next only so a runtime that returns from them enables
the step. The step's state is forbidden.

  microkit_cothread_init            272-294
    second init panics              280
  idx, a bad handle panics          243-248
  microkit_cothread_spawn           332-352
    empty free list is NULL_HANDLE  334-336
  microkit_cothread_yield           385-394
  microkit_cothread_destroy         404-411
    release of a blocked handle     413-421
  microkit_cothread_semaphore_wait  455-464
  microkit_cothread_semaphore_signal 467-475
  microkit_cothread_wait_on_channel 483-492
  microkit_cothread_recv_ntfn       504-520
    caller is the root              507-510
    co_switch before return         516-518
  block_running                     522-546
    the root cannot block           528-531
  signal_one                        548-570

signal_one pushes the caller and makes the waiter running. semaphore_signal
and recv_ntfn then switch to that waiter before they return. The host
process lock is not a runtime lock. Acquire and Release are the replay's
lock. A switch is enabled only while the lock is held by nobody.

What this leaves out is listed in
.cursor/skills/verify-cothread-contract/SKILL.md.
***************************************************************************)

EXTENDS Integers, Sequences

\* A config file cannot write a negative literal. The probed value is still -1.
MinusOne == -1

CONSTANTS
  Mode, Pool, NullHandle, RootHandle,
  RecvSwitched, SignalSwitched, FullReturnsNull,
  SecondInitPanics, BadHandlePanics, RootWaitPanics,
  WorkerRecvPanics, DestroyBlockedPanics,
  LockAcrossSwitch, WorkerPanicked

ASSUME Mode \in {"wake", "full"}
ASSUME Pool \in Nat /\ Pool >= 1
ASSUME NullHandle = MinusOne
ASSUME RootHandle = 0
ASSUME RecvSwitched \in BOOLEAN
ASSUME SignalSwitched \in BOOLEAN
ASSUME FullReturnsNull \in BOOLEAN
ASSUME SecondInitPanics \in BOOLEAN
ASSUME BadHandlePanics \in BOOLEAN
ASSUME RootWaitPanics \in BOOLEAN
ASSUME WorkerRecvPanics \in BOOLEAN
ASSUME DestroyBlockedPanics \in BOOLEAN
ASSUME LockAcrossSwitch \in BOOLEAN
ASSUME WorkerPanicked \in BOOLEAN

VARIABLES
  phase, initialised, free, nextHandle,
  ready, chQueue, semQueue,
  running, lockHeld,
  recvResumed, recvReturned,
  signalResumed, signalReturned,
  nullSpawn, breach, lockFault

vars == <<
  phase, initialised, free, nextHandle,
  ready, chQueue, semQueue,
  running, lockHeld,
  recvResumed, recvReturned,
  signalResumed, signalReturned,
  nullSpawn, breach, lockFault
>>

Init ==
  /\ phase = "acquireInit"
  /\ initialised = FALSE
  /\ free = Pool
  /\ nextHandle = 1
  /\ ready = <<>>
  /\ chQueue = <<>>
  /\ semQueue = <<>>
  /\ running = RootHandle
  /\ lockHeld = FALSE
  /\ recvResumed = FALSE
  /\ recvReturned = FALSE
  /\ signalResumed = FALSE
  /\ signalReturned = FALSE
  /\ nullSpawn = 0
  /\ breach = "none"
  /\ lockFault = FALSE

\* The replay takes the process lock before init. The runtime does not lock.
AcquireInit ==
  /\ phase = "acquireInit"
  /\ ~lockHeld
  /\ phase' = "init"
  /\ lockHeld' = TRUE
  /\ UNCHANGED <<
       initialised, free, nextHandle, ready, chQueue, semQueue,
       running, recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

\* First init. The root is already the running cothread. A second init is
\* SecondInit, which is enabled only when that call returns.
DoInit ==
  /\ phase = "init"
  /\ lockHeld
  /\ ~initialised
  /\ initialised' = TRUE
  /\ phase' = IF Mode = "wake" THEN "spawnCh" ELSE "fill"
  /\ UNCHANGED <<
       free, nextHandle, ready, chQueue, semQueue, running, lockHeld,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

SpawnCh ==
  /\ Mode = "wake"
  /\ phase = "spawnCh"
  /\ lockHeld
  /\ free > 0
  /\ free' = free - 1
  /\ nextHandle' = nextHandle + 1
  /\ ready' = Append(ready, nextHandle)
  /\ phase' = "releaseYieldCh"
  /\ UNCHANGED <<
       initialised, chQueue, semQueue, running, lockHeld,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

\* Drop the lock before yield. Yield switches.
ReleaseYieldCh ==
  /\ Mode = "wake"
  /\ phase = "releaseYieldCh"
  /\ lockHeld
  /\ lockHeld' = FALSE
  /\ phase' = "yieldCh"
  /\ UNCHANGED <<
       initialised, free, nextHandle, ready, chQueue, semQueue, running,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

\* Push the root, then run the spawned worker.
YieldCh ==
  /\ Mode = "wake"
  /\ phase = "yieldCh"
  /\ ~lockHeld
  /\ ready # <<>>
  /\ LET queued == Append(ready, RootHandle)
     IN /\ running' = Head(queued)
        /\ ready' = Tail(queued)
  /\ phase' = "waitCh"
  /\ UNCHANGED <<
       initialised, free, nextHandle, chQueue, semQueue, lockHeld,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

\* The worker blocks. The root was queued by YieldCh and runs again.
WaitCh ==
  /\ Mode = "wake"
  /\ phase = "waitCh"
  /\ ~lockHeld
  /\ running # RootHandle
  /\ ready # <<>>
  /\ chQueue' = Append(chQueue, running)
  /\ running' = Head(ready)
  /\ ready' = Tail(ready)
  /\ phase' = "acquireRecv"
  /\ UNCHANGED <<
       initialised, free, nextHandle, semQueue, lockHeld,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

AcquireRecv ==
  /\ Mode = "wake"
  /\ phase = "acquireRecv"
  /\ ~lockHeld
  /\ lockHeld' = TRUE
  /\ phase' = "releaseRecv"
  /\ UNCHANGED <<
       initialised, free, nextHandle, ready, chQueue, semQueue, running,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

ReleaseRecv ==
  /\ Mode = "wake"
  /\ phase = "releaseRecv"
  /\ lockHeld
  /\ lockHeld' = FALSE
  /\ phase' = "recvSwitch"
  /\ UNCHANGED <<
       initialised, free, nextHandle, ready, chQueue, semQueue, running,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

\* recv_ntfn from the root. The waiter runs before the call returns.
\* RecvSwitched is false when the runtime returns without that switch.
RecvSwitch ==
  /\ Mode = "wake"
  /\ phase = "recvSwitch"
  /\ ~lockHeld
  /\ running = RootHandle
  /\ chQueue # <<>>
  /\ IF RecvSwitched
     THEN /\ running' = Head(chQueue)
          /\ chQueue' = Tail(chQueue)
          /\ ready' = Append(ready, running)
          /\ recvResumed' = TRUE
          /\ phase' = "workerExitCh"
          /\ UNCHANGED recvReturned
     ELSE /\ recvReturned' = TRUE
          /\ recvResumed' = FALSE
          /\ phase' = "badRecv"
          /\ UNCHANGED <<running, chQueue, ready>>
  /\ UNCHANGED <<
       initialised, free, nextHandle, semQueue, lockHeld,
       signalResumed, signalReturned, nullSpawn, breach, lockFault
     >>

\* The waiter returns. destroy of the running cothread switches to the root.
WorkerExitCh ==
  /\ Mode = "wake"
  /\ phase = "workerExitCh"
  /\ ~lockHeld
  /\ recvResumed
  /\ running # RootHandle
  /\ ready # <<>>
  /\ free' = free + 1
  /\ running' = Head(ready)
  /\ ready' = Tail(ready)
  /\ phase' = "recvReturn"
  /\ UNCHANGED <<
       initialised, nextHandle, chQueue, semQueue, lockHeld,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

RecvReturn ==
  /\ Mode = "wake"
  /\ phase = "recvReturn"
  /\ running = RootHandle
  /\ recvReturned' = TRUE
  /\ phase' = "acquireSpawnSem"
  /\ UNCHANGED <<
       initialised, free, nextHandle, ready, chQueue, semQueue,
       running, lockHeld, recvResumed, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

\* The channel round dropped the lock before recv. Take it again before spawn.
AcquireSpawnSem ==
  /\ Mode = "wake"
  /\ phase = "acquireSpawnSem"
  /\ ~lockHeld
  /\ lockHeld' = TRUE
  /\ phase' = "spawnSem"
  /\ UNCHANGED <<
       initialised, free, nextHandle, ready, chQueue, semQueue, running,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

SpawnSem ==
  /\ Mode = "wake"
  /\ phase = "spawnSem"
  /\ lockHeld
  /\ free > 0
  /\ free' = free - 1
  /\ nextHandle' = nextHandle + 1
  /\ ready' = Append(ready, nextHandle)
  /\ phase' = "releaseYieldSem"
  /\ UNCHANGED <<
       initialised, chQueue, semQueue, running, lockHeld,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

ReleaseYieldSem ==
  /\ Mode = "wake"
  /\ phase = "releaseYieldSem"
  /\ lockHeld
  /\ lockHeld' = FALSE
  /\ phase' = "yieldSem"
  /\ UNCHANGED <<
       initialised, free, nextHandle, ready, chQueue, semQueue, running,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

YieldSem ==
  /\ Mode = "wake"
  /\ phase = "yieldSem"
  /\ ~lockHeld
  /\ ready # <<>>
  /\ LET queued == Append(ready, RootHandle)
     IN /\ running' = Head(queued)
        /\ ready' = Tail(queued)
  /\ phase' = "waitSem"
  /\ UNCHANGED <<
       initialised, free, nextHandle, chQueue, semQueue, lockHeld,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

WaitSem ==
  /\ Mode = "wake"
  /\ phase = "waitSem"
  /\ ~lockHeld
  /\ running # RootHandle
  /\ ready # <<>>
  /\ semQueue' = Append(semQueue, running)
  /\ running' = Head(ready)
  /\ ready' = Tail(ready)
  /\ phase' = "acquireSignal"
  /\ UNCHANGED <<
       initialised, free, nextHandle, chQueue, lockHeld,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

AcquireSignal ==
  /\ Mode = "wake"
  /\ phase = "acquireSignal"
  /\ ~lockHeld
  /\ lockHeld' = TRUE
  /\ phase' = "releaseSignal"
  /\ UNCHANGED <<
       initialised, free, nextHandle, ready, chQueue, semQueue, running,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

ReleaseSignal ==
  /\ Mode = "wake"
  /\ phase = "releaseSignal"
  /\ lockHeld
  /\ lockHeld' = FALSE
  /\ phase' = "signalSwitch"
  /\ UNCHANGED <<
       initialised, free, nextHandle, ready, chQueue, semQueue, running,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

\* semaphore_signal. The waiter runs before the call returns.
SignalSwitch ==
  /\ Mode = "wake"
  /\ phase = "signalSwitch"
  /\ ~lockHeld
  /\ running = RootHandle
  /\ semQueue # <<>>
  /\ IF SignalSwitched
     THEN /\ running' = Head(semQueue)
          /\ semQueue' = Tail(semQueue)
          /\ ready' = Append(ready, running)
          /\ signalResumed' = TRUE
          /\ phase' = "workerExitSem"
          /\ UNCHANGED signalReturned
     ELSE /\ signalReturned' = TRUE
          /\ signalResumed' = FALSE
          /\ phase' = "badSignal"
          /\ UNCHANGED <<running, semQueue, ready>>
  /\ UNCHANGED <<
       initialised, free, nextHandle, chQueue, lockHeld,
       recvResumed, recvReturned, nullSpawn, breach, lockFault
     >>

WorkerExitSem ==
  /\ Mode = "wake"
  /\ phase = "workerExitSem"
  /\ ~lockHeld
  /\ signalResumed
  /\ running # RootHandle
  /\ ready # <<>>
  /\ free' = free + 1
  /\ running' = Head(ready)
  /\ ready' = Tail(ready)
  /\ phase' = "signalReturn"
  /\ UNCHANGED <<
       initialised, nextHandle, chQueue, semQueue, lockHeld,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

SignalReturn ==
  /\ Mode = "wake"
  /\ phase = "signalReturn"
  /\ running = RootHandle
  /\ signalReturned' = TRUE
  /\ phase' = "done"
  /\ UNCHANGED <<
       initialised, free, nextHandle, ready, chQueue, semQueue,
       running, lockHeld, recvResumed, recvReturned, signalResumed,
       nullSpawn, breach, lockFault
     >>

\* Spawn until the free list is empty. Spawn does not switch, so the lock
\* stays held. The workers stay ready and are reaped without running.
Fill ==
  /\ Mode = "full"
  /\ phase = "fill"
  /\ lockHeld
  /\ IF free > 0
     THEN /\ free' = free - 1
          /\ nextHandle' = nextHandle + 1
          /\ ready' = Append(ready, nextHandle)
          /\ UNCHANGED phase
     ELSE /\ phase' = "overflow"
          /\ UNCHANGED <<free, nextHandle, ready>>
  /\ UNCHANGED <<
       initialised, chQueue, semQueue, running, lockHeld,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

\* The extra spawn. NULL_HANDLE leaves the pool alone. Any other result,
\* including a panic, is the forbidden state fullPanic.
Overflow ==
  /\ Mode = "full"
  /\ phase = "overflow"
  /\ lockHeld
  /\ free = 0
  /\ IF FullReturnsNull
     THEN /\ nullSpawn' = NullHandle
          /\ phase' = "reap"
          /\ UNCHANGED breach
     ELSE /\ breach' = "fullPanic"
          /\ phase' = "reap"
          /\ UNCHANGED nullSpawn
  /\ UNCHANGED <<
       initialised, free, nextHandle, ready, chQueue, semQueue,
       running, lockHeld, recvResumed, recvReturned,
       signalResumed, signalReturned, lockFault
     >>

Reap ==
  /\ Mode = "full"
  /\ phase = "reap"
  /\ lockHeld
  /\ IF ready # <<>>
     THEN /\ ready' = Tail(ready)
          /\ free' = free + 1
          /\ UNCHANGED phase
     ELSE /\ phase' = "done"
          /\ UNCHANGED <<ready, free>>
  /\ UNCHANGED <<
       initialised, nextHandle, chQueue, semQueue, running, lockHeld,
       recvResumed, recvReturned, signalResumed, signalReturned,
       nullSpawn, breach, lockFault
     >>

\* Enabled only when the call returns. A panic has no successor, so the
\* action stays disabled and breach remains "none".
SecondInit ==
  /\ initialised
  /\ breach = "none"
  /\ ~SecondInitPanics
  /\ breach' = "secondInit"
  /\ UNCHANGED <<
       phase, initialised, free, nextHandle, ready, chQueue, semQueue,
       running, lockHeld, recvResumed, recvReturned,
       signalResumed, signalReturned, nullSpawn, lockFault
     >>

BadHandle ==
  /\ initialised
  /\ breach = "none"
  /\ ~BadHandlePanics
  /\ breach' = "badHandle"
  /\ UNCHANGED <<
       phase, initialised, free, nextHandle, ready, chQueue, semQueue,
       running, lockHeld, recvResumed, recvReturned,
       signalResumed, signalReturned, nullSpawn, lockFault
     >>

RootWait ==
  /\ initialised
  /\ breach = "none"
  /\ running = RootHandle
  /\ ~RootWaitPanics
  /\ breach' = "rootWait"
  /\ UNCHANGED <<
       phase, initialised, free, nextHandle, ready, chQueue, semQueue,
       running, lockHeld, recvResumed, recvReturned,
       signalResumed, signalReturned, nullSpawn, lockFault
     >>

WorkerRecv ==
  /\ initialised
  /\ breach = "none"
  /\ running # RootHandle
  /\ ~WorkerRecvPanics
  /\ breach' = "workerRecv"
  /\ UNCHANGED <<
       phase, initialised, free, nextHandle, ready, chQueue, semQueue,
       running, lockHeld, recvResumed, recvReturned,
       signalResumed, signalReturned, nullSpawn, lockFault
     >>

DestroyBlocked ==
  /\ initialised
  /\ breach = "none"
  /\ ~DestroyBlockedPanics
  /\ chQueue # <<>> \/ semQueue # <<>>
  /\ breach' = "destroyBlocked"
  /\ UNCHANGED <<
       phase, initialised, free, nextHandle, ready, chQueue, semQueue,
       running, lockHeld, recvResumed, recvReturned,
       signalResumed, signalReturned, nullSpawn, lockFault
     >>

\* The replay observed the process lock held across a switch.
LockSwitch ==
  /\ LockAcrossSwitch
  /\ ~lockFault
  /\ lockFault' = TRUE
  /\ UNCHANGED <<
       phase, initialised, free, nextHandle, ready, chQueue, semQueue,
       running, lockHeld, recvResumed, recvReturned,
       signalResumed, signalReturned, nullSpawn, breach
     >>

WorkerPanic ==
  /\ WorkerPanicked
  /\ breach = "none"
  /\ breach' = "workerPanic"
  /\ UNCHANGED <<
       phase, initialised, free, nextHandle, ready, chQueue, semQueue,
       running, lockHeld, recvResumed, recvReturned,
       signalResumed, signalReturned, nullSpawn, lockFault
     >>

Terminated == phase = "done"

Park == Terminated /\ UNCHANGED vars

Next ==
  \/ Park
  \/ AcquireInit
  \/ DoInit
  \/ SpawnCh
  \/ ReleaseYieldCh
  \/ YieldCh
  \/ WaitCh
  \/ AcquireRecv
  \/ ReleaseRecv
  \/ RecvSwitch
  \/ WorkerExitCh
  \/ RecvReturn
  \/ AcquireSpawnSem
  \/ SpawnSem
  \/ ReleaseYieldSem
  \/ YieldSem
  \/ WaitSem
  \/ AcquireSignal
  \/ ReleaseSignal
  \/ SignalSwitch
  \/ WorkerExitSem
  \/ SignalReturn
  \/ Fill
  \/ Overflow
  \/ Reap
  \/ SecondInit
  \/ BadHandle
  \/ RootWait
  \/ WorkerRecv
  \/ DestroyBlocked
  \/ LockSwitch
  \/ WorkerPanic

Spec == Init /\ [][Next]_vars /\ WF_vars(Next)

NoRootQueued(q) == \A i \in 1..Len(q) : q[i] # RootHandle

\* breach names the contract break that returned. lockFault is a switch
\* under the process lock. The root is never on a wait queue.
NoForbidden ==
  /\ breach = "none"
  /\ ~lockFault
  /\ NoRootQueued(chQueue)
  /\ NoRootQueued(semQueue)

\* A returned recv or signal has already run the waiter.
SwitchBeforeReturn ==
  /\ recvReturned => recvResumed
  /\ signalReturned => signalResumed
  /\ phase \notin {"badRecv", "badSignal"}

IsQueue(q) ==
  /\ Len(q) <= Pool + 1
  /\ \A i \in 1..Len(q) : q[i] \in 0..Pool

TypeOK ==
  /\ phase \in {
       "acquireInit", "init",
       "spawnCh", "releaseYieldCh", "yieldCh", "waitCh",
       "acquireRecv", "releaseRecv", "recvSwitch", "workerExitCh", "recvReturn",
       "acquireSpawnSem",
       "spawnSem", "releaseYieldSem", "yieldSem", "waitSem",
       "acquireSignal", "releaseSignal", "signalSwitch", "workerExitSem",
       "signalReturn",
       "fill", "overflow", "reap",
       "badRecv", "badSignal", "done"
     }
  /\ initialised \in BOOLEAN
  /\ free \in 0..Pool
  /\ nextHandle \in 1..(Pool + 1)
  /\ IsQueue(ready)
  /\ IsQueue(chQueue)
  /\ IsQueue(semQueue)
  /\ running \in 0..Pool
  /\ lockHeld \in BOOLEAN
  /\ recvResumed \in BOOLEAN
  /\ recvReturned \in BOOLEAN
  /\ signalResumed \in BOOLEAN
  /\ signalReturned \in BOOLEAN
  /\ nullSpawn \in {NullHandle, 0}
  /\ breach \in {
       "none", "secondInit", "badHandle", "rootWait", "workerRecv",
       "destroyBlocked", "fullPanic", "workerPanic"
     }
  /\ lockFault \in BOOLEAN

EventuallyDone == <>Terminated

=============================================================================
