---- MODULE DeskKeys ----
(* Four desk input walks that share no bytes and so run side by side.
   `from_set1` clears `extended` after the prefix byte.
   `decode` skips a usage already in `state.down`.
   `UsbKbd::poll` returns before a key when `ready` is false.
   `PortStop::later` keeps the higher rank. *)

EXTENDS Integers

CONSTANTS ClearsExtended, SuppressHeld, PollChecksReady, OnlyForward

VARIABLES stroke, extended, letter, hid, hidEmit, ready, polled, emitted, rank, peak

vars == <<stroke, extended, letter, hid, hidEmit, ready, polled, emitted, rank, peak>>

restSet1 == <<hid, hidEmit, ready, polled, emitted, rank, peak>>
restHid == <<stroke, extended, letter, ready, polled, emitted, rank, peak>>
restHost == <<stroke, extended, letter, hid, hidEmit, ready, rank, peak>>
restPort == <<stroke, extended, letter, hid, hidEmit, ready, polled, emitted>>

Init ==
  /\ stroke = 0
  /\ extended = FALSE
  /\ letter = FALSE
  /\ hid = 0
  /\ hidEmit = FALSE
  /\ ready = FALSE
  /\ polled = FALSE
  /\ emitted = FALSE
  /\ rank = 0
  /\ peak = 0

YouPressThePrefix ==
  /\ stroke = 0
  /\ stroke' = 1
  /\ extended' = TRUE
  /\ letter' = FALSE
  /\ UNCHANGED restSet1

YouPressTheNextByte ==
  /\ stroke = 1
  /\ stroke' = 2
  /\ letter' = FALSE
  /\ UNCHANGED restSet1
  /\ IF ClearsExtended
     THEN extended' = FALSE
     ELSE extended' = TRUE

YouPressA ==
  /\ stroke = 2
  /\ stroke' = 3
  /\ UNCHANGED restSet1
  /\ IF extended
     THEN letter' = FALSE /\ UNCHANGED extended
     ELSE letter' = TRUE /\ UNCHANGED extended

TheKeyIsHeld ==
  /\ hid = 0
  /\ hid' = 1
  /\ hidEmit' = TRUE
  /\ UNCHANGED restHid

TheKeyRepeats ==
  /\ hid = 1
  /\ hid' = 2
  /\ UNCHANGED restHid
  /\ IF SuppressHeld
     THEN hidEmit' = FALSE
     ELSE hidEmit' = TRUE

TheControllerStarts ==
  /\ ~ready
  /\ ready' = TRUE
  /\ UNCHANGED <<stroke, extended, letter, hid, hidEmit, polled, emitted, rank, peak>>

TheHostPolls ==
  /\ ~polled
  /\ polled' = TRUE
  /\ UNCHANGED restHost
  /\ IF ready \/ ~PollChecksReady
     THEN emitted' = TRUE
     ELSE emitted' = FALSE

ThePortAdvances ==
  /\ rank < 3
  /\ rank' = rank + 1
  /\ peak' = rank + 1
  /\ UNCHANGED restPort

ThePortGoesBack ==
  /\ rank > 0
  /\ IF OnlyForward
     THEN UNCHANGED vars
     ELSE rank' = rank - 1 /\ UNCHANGED <<peak, stroke, extended, letter, hid, hidEmit, ready, polled, emitted>>

Park ==
  /\ stroke = 3
  /\ hid = 2
  /\ polled
  /\ rank = 3
  /\ UNCHANGED vars

Next ==
  \/ YouPressThePrefix
  \/ YouPressTheNextByte
  \/ YouPressA
  \/ TheKeyIsHeld
  \/ TheKeyRepeats
  \/ TheControllerStarts
  \/ TheHostPolls
  \/ ThePortAdvances
  \/ ThePortGoesBack
  \/ Park

Spec == Init /\ [][Next]_vars

LetterAfterPrefix == (stroke < 3) \/ letter
SecondReportSilent == (hid < 2) \/ ~hidEmit
PollWaitsReady == (~emitted) \/ ready
RankOnlyForward == rank = peak

====
