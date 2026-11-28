---- MODULE SdIdentify ----
(* `identify_card` sends CMD0, CMD8, CMD2, CMD3, CMD7 (select), then CMD16.
   `read_sector` and `write_sector` return before any transfer when
   `initialized` is false. `init_card` sets that flag after identify. *)

CONSTANTS OrderOk, ReadChecksInit

VARIABLES phase, selected, saw

vars == <<phase, selected, saw>>

Init ==
  /\ phase = "idle"
  /\ selected = FALSE
  /\ saw = FALSE

Identify ==
  /\ phase = "idle"
  /\ UNCHANGED saw
  /\ IF OrderOk
     THEN phase' = "ident" /\ UNCHANGED selected
     ELSE phase' = "ready" /\ selected' = FALSE

Select ==
  /\ phase = "ident"
  /\ phase' = "ready"
  /\ selected' = TRUE
  /\ UNCHANGED saw

Read ==
  /\ ~saw
  /\ IF (phase = "ready" /\ selected) \/ ~ReadChecksInit
     THEN saw' = TRUE /\ UNCHANGED <<phase, selected>>
     ELSE UNCHANGED vars

Park ==
  /\ saw
  /\ UNCHANGED vars

Next ==
  \/ Identify
  \/ Select
  \/ Read
  \/ Park

Spec == Init /\ [][Next]_vars

OrderHolds == (phase # "ready") \/ selected
ReadAfterSelect == (~saw) \/ (phase = "ready" /\ selected)

====
