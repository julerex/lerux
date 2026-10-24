------------------------------ MODULE SerialTx ------------------------------
(***************************************************************************
A client transmit ring, the transmit virtualiser, and the serial driver's
transmit drain. One step is one branch in the source.

The client loop is emit / say:

  userspace/pds/sddf-shell/src/main.rs          emit 534-544
  userspace/pds/sddf-grok/src/main.rs           say 146-156
  userspace/pds/sddf-program/src/main.rs        say 276-286
                                                Guest::log 89-91

The consumer is userspace/pds/sddf-serial-virt-tx/src/main.rs:

  move_client 93-114, wake_client 117-123, provide 125-130,
  resume_pending 136-157, notified 163-172

The driver step is drain_tx in
userspace/pds/sddf-serial-driver/src/main.rs 173-190.

Queue words are userspace/crates/lerux-sddf/src/serial_queue.rs:

  serial_enqueue 87-98, serial_dequeue 123-134,
  serial_request_consumer_signal 153-160,
  serial_cancel_consumer_signal 165-171,
  serial_require_consumer_signal 208-216

The wait is microkit_cothread_wait_on_channel in
userspace/crates/lerux-cothread/src/lib.rs 455-464. block_running 488-492
consumes a set semaphore and does not block. The image data region is
SERIAL_REGION_SIZE in userspace/crates/lerux-sddf/src/serial_image.rs:20.

Shared pages start at zero, so both producer_signalled words start at 0.
The image data region is SERIAL_REGION_SIZE (0x1000). Capacity here is a
model bound. While the driver ring is empty and the virtualiser has not
been notified, the only enabled client step is another enqueue, so the
first rejected enqueue is the same control state at every capacity.

What this leaves out is listed in
.cursor/skills/verify-serial-tx-backpressure/SKILL.md.
***************************************************************************)

EXTENDS Integers, Sequences

CONSTANT Capacity, Count

ASSUME Capacity \in Nat /\ Capacity >= 1
ASSUME Count \in Nat /\ Count >= 1

RECURSIVE Range(_)
Range(n) == IF n = 0 THEN <<>> ELSE Append(Range(n - 1), n)
Bytes == Range(Count)

VARIABLES
  queue,      \* client transmit bytes, producer is the client
  signalled,  \* client producer_signalled: 0 asks the virtualiser to notify
  driver,     \* driver transmit bytes
  drvSignal,  \* driver producer_signalled
  uart,       \* bytes drain_tx has written
  accepted,   \* how many Bytes serial_enqueue has accepted
  phase,      \* "enqueue" | "request" | "wait" | "idle"
  blocked,    \* worker is inside microkit_cothread_wait_on_channel
  sem,        \* TX channel semaphore sticky bit
  pending,    \* serial_virt_tx pending[client]
  virtRun,    \* "none" | "check" | "request" | "recheck" | "move" | "drain" | "wake" | "resume"
  burst,      \* length local in move_client
  note        \* driver notified the virtualiser while it was already running

vars == <<queue, signalled, driver, drvSignal, uart, accepted, phase, blocked, sem, pending, virtRun, burst, note>>

\* The shell, grok, and program protection domains are priority 1.
\* serial_virt_tx is 3. serial_driver is 4. A higher priority runs before
\* the client continues, so client steps stay disabled while virtRun # "none".
ClientTurn == virtRun = "none" /\ ~blocked

Init ==
  /\ queue = <<>>
  /\ signalled = 0
  /\ driver = <<>>
  /\ drvSignal = 0
  /\ uart = <<>>
  /\ accepted = 0
  /\ phase = "enqueue"
  /\ blocked = FALSE
  /\ sem = FALSE
  /\ pending = FALSE
  /\ virtRun = "none"
  /\ burst = 0
  /\ note = FALSE

\* emit's while condition. serial_enqueue returns -1 when the ring is full
\* (serial_queue.rs serial_enqueue).
ShellEnqueue ==
  /\ ClientTurn
  /\ phase = "enqueue"
  /\ accepted < Count
  /\ IF Len(queue) < Capacity
     THEN /\ queue' = Append(queue, Bytes[accepted + 1])
          /\ accepted' = accepted + 1
          /\ UNCHANGED phase
     ELSE /\ phase' = "request"
          /\ UNCHANGED <<queue, accepted>>
  /\ UNCHANGED <<signalled, driver, drvSignal, uart, blocked, sem, pending, virtRun, burst, note>>

\* serial_request_consumer_signal, then the next statement is the wait.
\* An interrupt can run the driver between these two statements.
ShellRequest ==
  /\ ClientTurn
  /\ phase = "request"
  /\ signalled' = 0
  /\ phase' = "wait"
  /\ UNCHANGED <<queue, driver, drvSignal, uart, accepted, blocked, sem, pending, virtRun, burst, note>>

\* microkit_cothread_wait_on_channel. block_running returns without blocking
\* when the semaphore is already set, and clears it.
ShellWait ==
  /\ ClientTurn
  /\ phase = "wait"
  /\ IF sem
     THEN /\ sem' = FALSE
          /\ phase' = "enqueue"
          /\ UNCHANGED blocked
     ELSE /\ blocked' = TRUE
          /\ UNCHANGED <<sem, phase>>
  /\ UNCHANGED <<queue, signalled, driver, drvSignal, uart, accepted, pending, virtRun, burst, note>>

\* Channel::notify after the for-loop. The virtualiser is higher priority,
\* so provide starts before this notify returns. The client does not run
\* again until virtRun is "none".
ShellNotify ==
  /\ ClientTurn
  /\ phase = "enqueue"
  /\ accepted = Count
  /\ phase' = "idle"
  /\ virtRun' = "check"
  /\ UNCHANGED <<queue, signalled, driver, drvSignal, uart, accepted, blocked, sem, pending, burst, note>>

Free == Capacity - Len(driver)

FinishVirt ==
  IF note
  THEN /\ virtRun' = "resume"
       /\ note' = FALSE
  ELSE /\ virtRun' = "none"
       /\ UNCHANGED note

\* move_client reads the consumer length once.
VirtCheck ==
  /\ virtRun = "check"
  /\ IF Len(queue) = 0
     THEN /\ burst' = 0
          /\ FinishVirt
     ELSE /\ burst' = Len(queue)
          /\ virtRun' = "request"
          /\ UNCHANGED note
  /\ UNCHANGED <<queue, signalled, driver, drvSignal, uart, accepted, phase, blocked, sem, pending>>

\* First free-space check. serial_request_consumer_signal on the driver ring.
VirtRequest ==
  /\ virtRun = "request"
  /\ IF burst > Free
     THEN drvSignal' = 0
     ELSE UNCHANGED drvSignal
  /\ virtRun' = "recheck"
  /\ UNCHANGED <<queue, signalled, driver, uart, accepted, phase, blocked, sem, pending, burst, note>>

\* Second free-space check. The driver may have drained between the two.
VirtRecheck ==
  /\ virtRun = "recheck"
  /\ IF burst > Free
     THEN /\ pending' = TRUE
          /\ FinishVirt
     ELSE /\ virtRun' = "move"
          /\ UNCHANGED <<pending, note>>
  /\ UNCHANGED <<queue, signalled, driver, drvSignal, uart, accepted, phase, blocked, sem, burst>>

\* The counted bytes move, or the earlier check would not have reached here.
VirtMove ==
  /\ virtRun = "move"
  /\ driver' = driver \o queue
  /\ queue' = <<>>
  /\ virtRun' = "drain"
  /\ UNCHANGED <<signalled, drvSignal, uart, accepted, phase, blocked, sem, pending, burst, note>>

\* provide notifies the driver only after a successful move. The driver is
\* higher priority and drain_tx empties the ring before wake_client. The
\* UART write spins, so one drain writes every queued byte.
VirtDrain ==
  /\ virtRun = "drain"
  /\ uart' = uart \o driver
  /\ driver' = <<>>
  /\ IF drvSignal = 0 /\ Len(driver) > 0
     THEN /\ drvSignal' = 1
          /\ note' = TRUE
     ELSE UNCHANGED <<drvSignal, note>>
  /\ virtRun' = "wake"
  /\ UNCHANGED <<queue, signalled, accepted, phase, blocked, sem, pending, burst>>

\* wake_client. A blocked worker is switched to and the wait returns.
\* Otherwise recv_ntfn leaves the sticky bit set.
VirtWake ==
  /\ virtRun = "wake"
  /\ IF signalled = 0
     THEN /\ signalled' = 1
          /\ IF blocked
             THEN /\ blocked' = FALSE
                  /\ phase' = "enqueue"
                  /\ UNCHANGED sem
             ELSE /\ sem' = TRUE
                  /\ UNCHANGED <<blocked, phase>>
     ELSE UNCHANGED <<signalled, blocked, phase, sem>>
  /\ FinishVirt
  /\ UNCHANGED <<queue, driver, drvSignal, uart, accepted, pending, burst>>

\* resume_pending. The driver channel woke the virtualiser. Clients that
\* are not pending are left alone, including a full ring the client has
\* not finished emit for.
VirtResume ==
  /\ virtRun = "resume"
  /\ IF pending
     THEN /\ pending' = FALSE
          /\ virtRun' = "check"
     ELSE /\ virtRun' = "none"
          /\ UNCHANGED pending
  /\ UNCHANGED <<queue, signalled, driver, drvSignal, uart, accepted, phase, blocked, sem, burst, note>>

\* serial_driver notified on its interrupt. Enabled only when the driver
\* ring holds a byte, and not during the synchronous drain above.
DriverIrq ==
  /\ virtRun \in {"none", "check", "request", "recheck", "move", "wake", "resume"}
  /\ Len(driver) > 0
  /\ uart' = uart \o driver
  /\ driver' = <<>>
  /\ IF drvSignal = 0
     THEN /\ drvSignal' = 1
          /\ IF virtRun = "none"
             THEN /\ virtRun' = "resume"
                  /\ UNCHANGED note
             ELSE /\ note' = TRUE
                  /\ UNCHANGED virtRun
     ELSE UNCHANGED <<drvSignal, virtRun, note>>
  /\ UNCHANGED <<queue, signalled, accepted, phase, blocked, sem, pending, burst>>

Terminated ==
  /\ phase = "idle"
  /\ virtRun = "none"
  /\ ~blocked
  /\ ~pending
  /\ queue = <<>>
  /\ driver = <<>>
  /\ accepted = Count
  /\ uart = Bytes

Park == Terminated /\ UNCHANGED vars

Next ==
  \/ Park
  \/ /\ ~Terminated
     /\ \/ ShellEnqueue
        \/ ShellRequest
        \/ ShellWait
        \/ ShellNotify
        \/ VirtCheck
        \/ VirtRequest
        \/ VirtRecheck
        \/ VirtMove
        \/ VirtDrain
        \/ VirtWake
        \/ VirtResume
        \/ DriverIrq

Spec == Init /\ [][Next]_vars /\ WF_vars(Next)

\* Bytes accepted by serial_enqueue are exactly the bytes still in the
\* client ring, the driver ring, or already written by drain_tx, in order.
Order == uart \o driver \o queue = SubSeq(Bytes, 1, accepted)

\* A blocked client with nothing scheduled to run, an empty driver ring,
\* and pending clear will not be notified. emit does not return.
NotStuck == blocked => virtRun # "none" \/ pending \/ Len(driver) > 0

EventuallyDone == <>Terminated

=============================================================================
