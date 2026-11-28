#!/usr/bin/env bash
set -euo pipefail
dir="$(cd "$(dirname "$0")" && pwd)"
exec "$dir/../implicit/run.sh" "$dir" TlsDrive tls \
  "EarlyConnected:FALSE" \
  "ConnectedAfterTraffic"
