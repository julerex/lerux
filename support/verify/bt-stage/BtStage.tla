------------------------------ MODULE BtStage ------------------------------
(***************************************************************************
The Swift Bluetooth panel line. One step is one branch the protection
domain can take when it chooses which formatter to paint. This module does
not run the command ring, move its address, or load firmware.

The 8 November 2026 panel reached `bt: port 3 desc`. That is a finished
line: three ports reported a connection, and desc is the furthest stage.
`bt: reset ok` is still the milestone exit. It is a later boot, painted
only after a command-complete event when the Bluetooth firmware is present.
A port line is not that success.

The protection domain picks the formatter:

  userspace/pds/swift-bt/src/main.rs
    controllers_live              57-72
    bring_up_text                 74-98
    reset_either                  100-109

The bytes come from lerux-swift:

  userspace/crates/lerux-swift/src/bt.rs
    acknowledge_reset             21-28
    PortStop::later               83-89
    fail_line                     114-125
    port_line                     127-154
    id_line                       156-172
    further                       198-218
    bring_up                      299-310
    format_report                 313-319
    version_line                  323-339

`find_radio` in userspace/pds/swift-bt/src/xhci.rs names the stages in the
order reset, slot, addr, desc. The model does not execute that function.

What this leaves out is listed in
.cursor/skills/verify-bt-stage/SKILL.md.
***************************************************************************)

EXTENDS Integers

CONSTANTS
  OnlyForward, ResetOkNeedsFirmware, ResetOkNeedsEvent, PortIsNotResetOk,
  EmptyIsFirmwareErr, VerPaintsWord, IdPaintsDevice, StageLabels,
  PaintsVersion, PaintsFirmware, PaintsBringUp, PaintsFail,
  WitnessCount, WitnessRank, VerLo, VerHi

ASSUME OnlyForward \in BOOLEAN /\ ResetOkNeedsFirmware \in BOOLEAN
ASSUME ResetOkNeedsEvent \in BOOLEAN /\ PortIsNotResetOk \in BOOLEAN
ASSUME EmptyIsFirmwareErr \in BOOLEAN /\ VerPaintsWord \in BOOLEAN
ASSUME IdPaintsDevice \in BOOLEAN /\ StageLabels \in BOOLEAN
ASSUME PaintsVersion \in BOOLEAN /\ PaintsFirmware \in BOOLEAN
ASSUME PaintsBringUp \in BOOLEAN /\ PaintsFail \in BOOLEAN
ASSUME WitnessCount = 3 /\ WitnessRank = 4
ASSUME VerLo = 256 /\ VerHi = 288 /\ VerLo < VerHi

\* Ranks are the stage words: 1 reset, 2 slot, 3 addr, 4 desc.
\* 0 means no port has reported a stage.

VARIABLES
  phase, firmware, live, foreign,
  connected, rank, peak, kind, eventAck

vars == <<
  phase, firmware, live, foreign,
  connected, rank, peak, kind, eventAck
>>

Init ==
  /\ phase = "boot"
  /\ \/ /\ live = TRUE /\ foreign = FALSE /\ firmware \in BOOLEAN
     \/ /\ live = FALSE /\ foreign \in BOOLEAN /\ firmware \in BOOLEAN
  /\ connected = 0
  /\ rank = 0
  /\ peak = 0
  /\ kind = "none"
  /\ eventAck = FALSE

\* A dead controller. An out-of-range capability word paints `bt: ver`.
ClassifyDead ==
  /\ phase = "boot"
  /\ ~live
  /\ phase' = "done"
  /\ IF foreign THEN kind' = "ver" ELSE kind' = "absent"
  /\ UNCHANGED <<firmware, live, foreign, connected, rank, peak, eventAck>>

\* An empty firmware blob. This line is `bt: firmware err`, not reset ok.
ClassifyEmpty ==
  /\ phase = "boot"
  /\ live
  /\ ~firmware
  /\ phase' = "done"
  /\ kind' = "fw"
  /\ UNCHANGED <<firmware, live, foreign, connected, rank, peak, eventAck>>

\* Firmware is present, so a transfer may run. Nothing is painted yet.
OpenPorts ==
  /\ phase = "boot"
  /\ live
  /\ firmware
  /\ phase' = "ports"
  /\ UNCHANGED <<firmware, live, foreign, connected, rank, peak, kind, eventAck>>

\* One more connected port, stopping at rank r.
\* OnlyForward keeps the furthest stage. A later, earlier failure does not
\* replace it. The count is how many ports reported a connection.
SeeStage(r) ==
  /\ phase = "ports"
  /\ kind \in {"none", "port"}
  /\ connected < WitnessCount
  /\ r \in 1..WitnessRank
  /\ connected' = connected + 1
  /\ LET furthest == IF r > peak THEN r ELSE peak
     IN /\ peak' = furthest
        /\ rank' = IF OnlyForward THEN furthest ELSE r
  /\ kind' = "port"
  /\ UNCHANGED <<phase, firmware, live, foreign, eventAck>>

\* An addressed device that is not the HCI interface.
SeeId ==
  /\ phase = "ports"
  /\ kind \in {"none", "port"}
  /\ kind' = "id"
  /\ UNCHANGED <<phase, firmware, live, foreign, connected, rank, peak, eventAck>>

\* The reset did not complete. This is `bt: event`, not `bt: reset ok`.
SeeEvent ==
  /\ phase = "ports"
  /\ kind \in {"none", "port", "id"}
  /\ kind' = "event"
  /\ UNCHANGED <<phase, firmware, live, foreign, connected, rank, peak, eventAck>>

\* Command-complete while firmware is present. A stage word does not set this.
AckEvent ==
  /\ phase = "ports"
  /\ firmware
  /\ ~eventAck
  /\ eventAck' = TRUE
  /\ UNCHANGED <<phase, firmware, live, foreign, connected, rank, peak, kind>>

\* The only step to `bt: reset ok`.
PaintOk ==
  /\ phase = "ports"
  /\ eventAck
  /\ firmware
  /\ live
  /\ ResetOkNeedsFirmware
  /\ ResetOkNeedsEvent
  /\ PaintsBringUp
  /\ phase' = "done"
  /\ kind' = "ok"
  /\ UNCHANGED <<firmware, live, foreign, connected, rank, peak, eventAck>>

\* A finished failure line. Three ports at desc is one of these, and it does
\* not continue into reset ok.
PaintFail ==
  /\ phase = "ports"
  /\ ~eventAck
  /\ kind \in {"port", "id", "event"}
  /\ phase' = "done"
  /\ UNCHANGED <<firmware, live, foreign, connected, rank, peak, kind, eventAck>>

PaintHalt ==
  /\ phase = "ports"
  /\ ~eventAck
  /\ kind = "none"
  /\ phase' = "done"
  /\ kind' = "halt"
  /\ UNCHANGED <<firmware, live, foreign, connected, rank, peak, eventAck>>

\* No port reported a connection, so the line has a count and no stage word.
PaintUnconnected ==
  /\ phase = "ports"
  /\ ~eventAck
  /\ kind = "none"
  /\ connected = 0
  /\ phase' = "done"
  /\ kind' = "port"
  /\ UNCHANGED <<firmware, live, foreign, connected, rank, peak, eventAck>>

Terminated == phase = "done"

Park == Terminated /\ UNCHANGED vars

Next ==
  \/ Park
  \/ ClassifyDead
  \/ ClassifyEmpty
  \/ OpenPorts
  \/ \E r \in 1..WitnessRank : SeeStage(r)
  \/ SeeId
  \/ SeeEvent
  \/ AckEvent
  \/ PaintOk
  \/ PaintFail
  \/ PaintHalt
  \/ PaintUnconnected

Spec == Init /\ [][Next]_vars /\ WF_vars(Next)

\* The painted port stage is the furthest one, and the count is the number
\* of ports that reported a connection. Rank 0 paints no stage word.
PortLine ==
  \/ kind /= "port"
  \/ /\ rank = peak
     /\ IF rank = 0
        THEN connected = 0
        ELSE /\ connected \in 1..WitnessCount
             /\ rank \in 1..WitnessRank

\* A finished port line is not the command-complete success.
PortIsNotSuccess ==
  /\ kind /= "port" \/ PortIsNotResetOk
  /\ ~(phase = "done" /\ kind = "port" /\ eventAck)

\* `bt: reset ok` is only the command-complete event after firmware is present.
ResetOkSeparate ==
  \/ kind /= "ok"
  \/ /\ firmware /\ live /\ eventAck
     /\ ResetOkNeedsFirmware /\ ResetOkNeedsEvent /\ PaintsBringUp

EmptyBlob ==
  \/ kind /= "fw"
  \/ /\ live /\ ~firmware /\ EmptyIsFirmwareErr /\ PaintsFirmware

VersionWord ==
  \/ kind /= "ver"
  \/ /\ ~live /\ foreign /\ VerPaintsWord /\ PaintsVersion

IdDevice ==
  \/ kind /= "id"
  \/ IdPaintsDevice

FailureUsesFormatter ==
  \/ kind \notin {"port", "id", "event", "halt"}
  \/ PaintsFail

Labels == StageLabels

EventuallyDone == <>Terminated

=============================================================================
