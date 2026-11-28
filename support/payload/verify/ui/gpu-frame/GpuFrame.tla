---- MODULE GpuFrame ----
(* `flush` copies pixels only when `resource.transfer` is `Some`.
   `forward_requests` stores `live: true`. `forward_responses` stores
   `note.live = false` after it has used the note. *)

CONSTANTS FlushNeedsTransfer, ClearsNote

VARIABLES resource, flush, note, answered

vars == <<resource, flush, note, answered>>

Init ==
  /\ resource = "none"
  /\ flush = "none"
  /\ note = "free"
  /\ answered = FALSE

Create ==
  /\ resource = "none"
  /\ resource' = "created"
  /\ flush' = "none"
  /\ UNCHANGED <<note, answered>>

Attach ==
  /\ resource = "created"
  /\ resource' = "attached"
  /\ flush' = "none"
  /\ UNCHANGED <<note, answered>>

Transfer ==
  /\ resource = "attached"
  /\ resource' = "staged"
  /\ flush' = "none"
  /\ UNCHANGED <<note, answered>>

Detach ==
  /\ resource \in {"attached", "staged"}
  /\ resource' = "created"
  /\ flush' = "none"
  /\ UNCHANGED <<note, answered>>

Unref ==
  /\ resource # "none"
  /\ resource' = "none"
  /\ flush' = "none"
  /\ UNCHANGED <<note, answered>>

Flush ==
  /\ resource \in {"attached", "staged"}
  /\ UNCHANGED <<resource, note, answered>>
  /\ IF resource = "staged"
     THEN flush' = "ok"
     ELSE IF FlushNeedsTransfer
          THEN flush' = "refused"
          ELSE flush' = "ok"

Forward ==
  /\ note = "free"
  /\ note' = "live"
  /\ answered' = FALSE
  /\ UNCHANGED <<resource, flush>>

Reply ==
  /\ note = "live"
  /\ answered' = TRUE
  /\ UNCHANGED <<resource, flush>>
  /\ IF ClearsNote
     THEN note' = "free"
     ELSE UNCHANGED note

Park ==
  /\ (flush = "ok" \/ answered)
  /\ UNCHANGED vars

Next ==
  \/ Create
  \/ Attach
  \/ Transfer
  \/ Detach
  \/ Unref
  \/ Flush
  \/ Forward
  \/ Reply
  \/ Park

Spec == Init /\ [][Next]_vars

FlushNeedsStage == (flush # "ok") \/ (resource = "staged")
NoteCleared == (~answered) \/ (note = "free")

====
