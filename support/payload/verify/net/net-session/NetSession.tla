---- MODULE NetSession ----
(* `take_dhcp` treats `Deconfigured` and no event as "no address".
   `Stack::connect` sets `connecting` and clears `listening`.
   `Stack::listen` sets `listening` and clears `connecting`.
   `HttpNet::poll` serves one page and then returns.
   `poll_udp_tx` returns once `tx_done` is set.
   `Sockets::Phase` is a separate explicit machine. This model is the
   flag layer those two methods write, including listen followed by connect. *)

CONSTANTS
  IgnoresDeconfigured, ConnectClearsListen, ListenClearsConnect,
  HttpStops, OnceStops

VARIABLES place, connecting, listening, sent, last

vars == <<place, connecting, listening, sent, last>>

Init ==
  /\ place = "bare"
  /\ connecting = FALSE
  /\ listening = FALSE
  /\ sent = "no"
  /\ last = "none"

TheOfferArrives ==
  /\ place = "bare"
  /\ place' = "address"
  /\ last' = "offer"
  /\ UNCHANGED <<connecting, listening, sent>>

TheLinkDrops ==
  /\ place = "bare"
  /\ last' = "deconfigured"
  /\ UNCHANGED <<connecting, listening, sent>>
  /\ IF IgnoresDeconfigured
     THEN UNCHANGED place
     ELSE place' = "address"

YouConnect ==
  /\ place \in {"address", "listening"}
  /\ connecting' = TRUE
  /\ place' = "connecting"
  /\ UNCHANGED <<sent, last>>
  /\ IF ConnectClearsListen
     THEN listening' = FALSE
     ELSE UNCHANGED listening

ThePeerAccepts ==
  /\ place = "connecting"
  /\ place' = "open"
  /\ UNCHANGED <<connecting, listening, sent, last>>

TheBodyArrives ==
  /\ place = "open"
  /\ place' = "fetched"
  /\ UNCHANGED <<connecting, listening, sent, last>>

YouListen ==
  /\ place \in {"address", "connecting"}
  /\ listening' = TRUE
  /\ place' = "listening"
  /\ UNCHANGED <<sent, last>>
  /\ IF ListenClearsConnect
     THEN connecting' = FALSE
     ELSE UNCHANGED connecting

ThePageIsServed ==
  /\ place = "listening"
  /\ place' = "served"
  /\ UNCHANGED <<connecting, listening, sent, last>>

AnotherRequest ==
  /\ place = "served"
  /\ UNCHANGED <<connecting, listening, sent, last>>
  /\ IF HttpStops
     THEN place' = "held"
     ELSE place' = "again"

TheDatagramGoes ==
  /\ sent = "no"
  /\ sent' = "yes"
  /\ UNCHANGED <<place, connecting, listening, last>>

TheDatagramRepeats ==
  /\ sent = "yes"
  /\ UNCHANGED <<place, connecting, listening, last>>
  /\ IF OnceStops
     THEN UNCHANGED sent
     ELSE sent' = "again"

YouFinish ==
  /\ place \in {"connecting", "open", "listening", "served", "held", "again"}
  /\ place' = "address"
  /\ connecting' = FALSE
  /\ listening' = FALSE
  /\ UNCHANGED <<sent, last>>

Park ==
  /\ place = "fetched" \/ sent = "again"
  /\ UNCHANGED vars

Next ==
  \/ TheOfferArrives
  \/ TheLinkDrops
  \/ YouConnect
  \/ ThePeerAccepts
  \/ TheBodyArrives
  \/ YouListen
  \/ ThePageIsServed
  \/ AnotherRequest
  \/ TheDatagramGoes
  \/ TheDatagramRepeats
  \/ YouFinish
  \/ Park

Spec == Init /\ [][Next]_vars

DeconfiguredStaysBare == (last # "deconfigured") \/ (place = "bare")
FlagsExclusive == ~(connecting /\ listening)
OnePage == place # "again"
HelloOnce == sent # "again"

====
