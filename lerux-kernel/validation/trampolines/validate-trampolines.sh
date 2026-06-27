#!/usr/bin/env bash
#
# Validate SMP trampoline NASM sources under lerux-kernel/src/asm/.
#
# Usage:
#   ./validate-trampolines.sh              # check (default)
#
# Requires: nasm
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PY="$SCRIPT_DIR/compare_trampoline_bytes.py"

cmd="${1:-check}"
case "$cmd" in
  check) exec python3 "$PY" check ;;
  *)
    echo "usage: $0 [check]" >&2
    exit 2
    ;;
esac