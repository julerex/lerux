---- MODULE DeviceCompletion ----
(* One ownership step, four devices. The constant is true only when all
   four checks are in the source: e1000e descriptor done, GENET `DMA_OWN`,
   the NVMe completion phase bit, and AHCI `poll_pxci` before the task file. *)

CONSTANTS CopyChecks

VARIABLES done, copied

vars == <<done, copied>>

Init ==
  /\ done = FALSE
  /\ copied = FALSE

TheDeviceFinishes ==
  /\ ~done
  /\ done' = TRUE
  /\ UNCHANGED copied

SoftwareCopies ==
  /\ ~copied
  /\ IF done \/ ~CopyChecks
     THEN copied' = TRUE /\ UNCHANGED done
     ELSE UNCHANGED vars

Park ==
  /\ copied
  /\ UNCHANGED vars

Next ==
  \/ TheDeviceFinishes
  \/ SoftwareCopies
  \/ Park

Spec == Init /\ [][Next]_vars

CopyAfterDone == (~copied) \/ done

====
