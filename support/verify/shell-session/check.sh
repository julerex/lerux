#!/usr/bin/env bash
set -euo pipefail
dir="$(cd "$(dirname "$0")" && pwd)"
exec "$dir/../implicit/run.sh" "$dir" ShellSession shell \
  "QuitReturns:TRUE,BrowserReturns:TRUE,NestedRejected:TRUE" \
  "QuitLeftTheMaths,BrowserReturned,NoNestedSource"
