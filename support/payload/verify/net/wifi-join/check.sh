#!/usr/bin/env bash
set -euo pipefail
dir="$(cd "$(dirname "$0")" && pwd)"
exec "$dir/../implicit/run.sh" "$dir" WifiJoin wifi \
  "OwnLabel:TRUE,EapolChecksReply:TRUE,KeyChecked:TRUE,FirmwareWaits:TRUE" \
  "OnlineIsFinished,ShortKeyStays"
