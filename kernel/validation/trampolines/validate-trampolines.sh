#!/usr/bin/env bash
#
# Validate embedded SMP trampoline bytes against the original NASM sources.
#
# Usage:
#   ./validate-trampolines.sh              # check (default)
#   ./validate-trampolines.sh refresh      # regenerate expected/*.bin from asm/
#   ./validate-trampolines.sh print-rust   # print Rust arrays for manual paste
#
# Requires: nasm (not part of the kernel build; dev/CI validation only)
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PY="$SCRIPT_DIR/compare_trampoline_bytes.py"

cmd="${1:-check}"
case "$cmd" in
  check)           exec python3 "$PY" check ;;
  refresh|refresh-expected) exec python3 "$PY" refresh-expected ;;
  print-rust|update) exec python3 "$PY" print-rust ;;
  *)
    echo "usage: $0 [check|refresh|print-rust]" >&2
    exit 2
    ;;
esac
