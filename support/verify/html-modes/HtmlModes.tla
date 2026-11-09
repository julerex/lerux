---- MODULE HtmlModes ----
(* In `finish`, the `Title` arm reads raw text and then pops the open
   element, so the next token is not still inside the title. *)

CONSTANTS TakesRaw

VARIABLES mode, text

vars == <<mode, text>>

Init ==
  /\ mode = "data"
  /\ text = ""

SeeTheTitle ==
  /\ mode = "data"
  /\ text' = "Hi"
  /\ IF TakesRaw
     THEN mode' = "data"
     ELSE mode' = "title"

Park ==
  /\ text = "Hi"
  /\ UNCHANGED vars

Next ==
  \/ SeeTheTitle
  \/ Park

Spec == Init /\ [][Next]_vars

TitleClosed == (text # "Hi") \/ (mode # "title")

====
