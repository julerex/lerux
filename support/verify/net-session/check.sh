#!/usr/bin/env bash
set -euo pipefail
dir="$(cd "$(dirname "$0")" && pwd)"
exec "$dir/../implicit/run.sh" "$dir" NetSession net \
  "IgnoresDeconfigured:TRUE,ConnectClearsListen:TRUE,ListenClearsConnect:TRUE,HttpStops:TRUE,OnceStops:TRUE" \
  "DeconfiguredStaysBare,FlagsExclusive,OnePage,HelloOnce"
