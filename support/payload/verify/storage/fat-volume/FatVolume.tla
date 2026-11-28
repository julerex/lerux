---- MODULE FatVolume ----
(* `file_busy` refuses a second file open. `handle_dir_open` refuses a
   second directory the same way. `load_block` calls `flush_block` before
   it reads a different transfer block. *)

EXTENDS Integers

CONSTANTS SecondRefused, FlushesFirst

VARIABLES files, block, dirty, lost

vars == <<files, block, dirty, lost>>

Init ==
  /\ files = 0
  /\ block = "none"
  /\ dirty = FALSE
  /\ lost = FALSE

Open ==
  /\ files = 0
  /\ files' = 1
  /\ UNCHANGED <<block, dirty, lost>>

OpenAgain ==
  /\ files = 1
  /\ IF SecondRefused
     THEN UNCHANGED vars
     ELSE files' = 2 /\ UNCHANGED <<block, dirty, lost>>

Close ==
  /\ files \in {1, 2}
  /\ files' = files - 1
  /\ UNCHANGED <<block, dirty, lost>>

Load(target) ==
  /\ block # target
  /\ UNCHANGED files
  /\ IF dirty /\ ~FlushesFirst
     THEN (lost' = TRUE /\ dirty' = FALSE /\ block' = target)
     ELSE (dirty' = FALSE /\ block' = target /\ UNCHANGED lost)

LoadA == Load("a")
LoadB == Load("b")

Edit ==
  /\ block # "none"
  /\ dirty' = TRUE
  /\ UNCHANGED <<files, block, lost>>

Park ==
  /\ (files = 2 \/ lost)
  /\ UNCHANGED vars

Next ==
  \/ Open
  \/ OpenAgain
  \/ Close
  \/ LoadA
  \/ LoadB
  \/ Edit
  \/ Park

Spec == Init /\ [][Next]_vars

OneFile == files < 2
NoLostBlock == ~lost

====
