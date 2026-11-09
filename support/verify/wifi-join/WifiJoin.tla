---- MODULE WifiJoin ----
(* The Swift menu asks for a network and a passphrase, then the radio
   walks scan, auth, assoc, the 4-way handshake, the address, and the ping.
   `dispatch_psk` calls `key_ok` before `post_wifi`.
   `Mac::associate` returns the stage it just entered when that step fails.
   `wifi::associate` rejects a handshake reply other than one byte `1`.
   `ready` stays false until `stage` is `Wait`. *)

CONSTANTS OwnLabel, EapolChecksReply, KeyChecked, FirmwareWaits

VARIABLES screen, radio, via, pass, booted

vars == <<screen, radio, via, pass, booted>>

Running == {"scan", "auth", "assoc", "eapol", "dhcp", "ping"}

Init ==
  /\ screen = "menu"
  /\ radio = "idle"
  /\ via = "run"
  /\ pass = "none"
  /\ booted = FALSE

TheFirmwareFinishes ==
  /\ booted = FALSE
  /\ booted' = TRUE
  /\ IF FirmwareWaits
     THEN UNCHANGED <<screen, radio, via, pass>>
     ELSE /\ radio' = "online"
          /\ via' = "fail"
          /\ UNCHANGED <<screen, pass>>

YouChooseWifi ==
  /\ screen = "menu"
  /\ booted
  /\ radio = "idle"
  /\ via = "run"
  /\ screen' = "ssid"
  /\ UNCHANGED <<radio, via, pass, booted>>

YouQuit ==
  /\ screen \in {"ssid", "psk"}
  /\ screen' = "menu"
  /\ radio' = "idle"
  /\ via' = "run"
  /\ pass' = "none"
  /\ UNCHANGED booted

YouNameTheNetwork ==
  /\ screen = "ssid"
  /\ screen' = "psk"
  /\ UNCHANGED <<radio, via, pass, booted>>

YouTypeAShortPassphrase ==
  /\ screen = "psk"
  /\ pass' = "short"
  /\ UNCHANGED booted
  /\ IF KeyChecked
     THEN (screen' = "menu" /\ radio' = "idle" /\ via' = "run")
     ELSE (screen' = "radio" /\ radio' = "scan" /\ via' = "run")

YouTypeAPassphrase ==
  /\ screen = "psk"
  /\ booted
  /\ pass' = "good"
  /\ screen' = "radio"
  /\ radio' = "scan"
  /\ via' = "run"
  /\ UNCHANGED booted

Advance(here, there) ==
  /\ screen = "radio"
  /\ via = "run"
  /\ radio = here
  /\ radio' = there
  /\ UNCHANGED <<screen, via, pass, booted>>

EnterAuth == Advance("scan", "auth")
EnterAssoc == Advance("auth", "assoc")
EnterEapol == Advance("assoc", "eapol")
EnterDhcp == Advance("eapol", "dhcp")
EnterPing == Advance("dhcp", "ping")

ThePingAnswers ==
  /\ screen = "radio"
  /\ via = "run"
  /\ radio = "ping"
  /\ radio' = "online"
  /\ via' = "ok"
  /\ UNCHANGED <<screen, pass, booted>>

TheRadioStops ==
  /\ screen = "radio"
  /\ via = "run"
  /\ radio \in Running
  /\ UNCHANGED <<screen, pass, booted>>
  /\ IF radio = "eapol" /\ ~EapolChecksReply
     THEN (radio' = "online" /\ via' = "fail")
     ELSE IF ~OwnLabel
          THEN (radio' = "online" /\ via' = "fail")
          ELSE (via' = "fail" /\ UNCHANGED radio)

Park ==
  /\ via \in {"ok", "fail"}
  /\ UNCHANGED vars

Next ==
  \/ TheFirmwareFinishes
  \/ YouChooseWifi
  \/ YouQuit
  \/ YouNameTheNetwork
  \/ YouTypeAShortPassphrase
  \/ YouTypeAPassphrase
  \/ EnterAuth
  \/ EnterAssoc
  \/ EnterEapol
  \/ EnterDhcp
  \/ EnterPing
  \/ ThePingAnswers
  \/ TheRadioStops
  \/ Park

Spec == Init /\ [][Next]_vars

OnlineIsFinished == (radio = "online") => (via = "ok")
ShortKeyStays == (screen # "radio") \/ (pass = "good")

====
