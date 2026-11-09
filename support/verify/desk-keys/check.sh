#!/usr/bin/env bash
set -euo pipefail
dir="$(cd "$(dirname "$0")" && pwd)"
exec "$dir/../implicit/run.sh" "$dir" DeskKeys keys \
  "ClearsExtended:TRUE,SuppressHeld:TRUE,PollChecksReady:TRUE,OnlyForward:TRUE" \
  "LetterAfterPrefix,SecondReportSilent,PollWaitsReady,RankOnlyForward"
