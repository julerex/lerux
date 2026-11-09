---- MODULE ShellSession ----
(* The browser menu and the fetch `source` command.
   `app_menu` calls `browse` or `maths_repl` and then prints the menu again.
   `maths_repl` returns when the line is `quit`.
   `cmd_source` returns before `source_file` when `SOURCING` is already set.
   Those three functions are different images. A maths step runs only while
   sourcing is idle, and a source step runs only while the phase is the menu. *)

CONSTANTS QuitReturns, BrowserReturns, NestedRejected

VARIABLES phase, sourcing, last

vars == <<phase, sourcing, last>>

Init ==
  /\ phase = "menu"
  /\ sourcing = "idle"
  /\ last = "none"

YouChooseMaths ==
  /\ phase = "menu"
  /\ sourcing = "idle"
  /\ phase' = "maths"
  /\ last' = "prompt"
  /\ UNCHANGED sourcing

YouQuitMaths ==
  /\ phase = "maths"
  /\ last' = "quit"
  /\ phase' = IF QuitReturns THEN "menu" ELSE "maths"
  /\ UNCHANGED sourcing

YouTypeExpression ==
  /\ phase = "maths"
  /\ last # "quit"
  /\ phase' = "answered"
  /\ last' = "value"
  /\ UNCHANGED sourcing

YouSeeThePrompt ==
  /\ phase = "answered"
  /\ phase' = "maths"
  /\ last' = "prompt"
  /\ UNCHANGED sourcing

YouChooseBrowser ==
  /\ phase = "menu"
  /\ sourcing = "idle"
  /\ phase' = "painting"
  /\ last' = "browser"
  /\ UNCHANGED sourcing

ThePaintReturns ==
  /\ phase = "painting"
  /\ phase' = IF BrowserReturns THEN "menu" ELSE "browser"
  /\ UNCHANGED <<sourcing, last>>

YouTypeAnUnknownLine ==
  /\ phase = "menu"
  /\ sourcing = "idle"
  /\ last' = "err"
  /\ UNCHANGED <<phase, sourcing>>

YouSourceAFile ==
  /\ phase = "menu"
  /\ sourcing = "idle"
  /\ sourcing' = "open"
  /\ last' = "source"
  /\ UNCHANGED phase

YouSourceAgain ==
  /\ sourcing = "open"
  /\ sourcing' = IF NestedRejected THEN "rejected" ELSE "nested"
  /\ UNCHANGED <<phase, last>>

TheNestedLineIsRejected ==
  /\ sourcing = "rejected"
  /\ sourcing' = "open"
  /\ UNCHANGED <<phase, last>>

TheSourceFinishes ==
  /\ sourcing = "open"
  /\ last = "source"
  /\ sourcing' = "idle"
  /\ UNCHANGED <<phase, last>>

Stuck ==
  /\ phase = "browser" \/ sourcing = "nested"
  /\ UNCHANGED vars

Next ==
  \/ YouChooseMaths
  \/ YouQuitMaths
  \/ YouTypeExpression
  \/ YouSeeThePrompt
  \/ YouChooseBrowser
  \/ ThePaintReturns
  \/ YouTypeAnUnknownLine
  \/ YouSourceAFile
  \/ YouSourceAgain
  \/ TheNestedLineIsRejected
  \/ TheSourceFinishes
  \/ Stuck

Spec == Init /\ [][Next]_vars

QuitLeftTheMaths == last # "quit" \/ phase = "menu"
BrowserReturned == phase # "browser"
NoNestedSource == sourcing # "nested"

====
