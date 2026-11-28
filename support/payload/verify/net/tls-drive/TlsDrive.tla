---- MODULE TlsDrive ----
(* `ClientSession::drive` sets `connected` only in the `WriteTraffic` and
   `ReadTraffic` arms. The socket is open before those arms run. *)

CONSTANTS EarlyConnected

VARIABLES tcp, place

vars == <<tcp, place>>

Init ==
  /\ tcp = FALSE
  /\ place = "new"

Open ==
  /\ tcp = FALSE
  /\ place = "new"
  /\ tcp' = TRUE
  /\ place' = "open"

Drive ==
  /\ place \in {"new", "open"}
  /\ UNCHANGED tcp
  /\ IF place = "open" \/ EarlyConnected
     THEN place' = "connected"
     ELSE UNCHANGED place

Park ==
  /\ place = "connected"
  /\ UNCHANGED vars

Next ==
  \/ Open
  \/ Drive
  \/ Park

Spec == Init /\ [][Next]_vars

ConnectedAfterTraffic == (place # "connected") \/ tcp

====
