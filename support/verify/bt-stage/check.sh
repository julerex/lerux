#!/usr/bin/env bash
# Check the Swift Bluetooth stage line.
# Exit 0: the furthest stage is the painted port line, and reset ok stays
#         the command-complete line.
# Exit 1: a formatter disagrees with the milestone, or the protection domain
#         paints a different call than that formatter.
# Exit 2: the source shape is not one this model can read, the probed
#         constants differ from boot.cfg, or TLC disagrees with a Rust run
#         that itself accepted the lines.
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$root/../../.." && pwd)"
jar_sum="936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88"
cache="${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc"
jar="$cache/tla2tools-1.7.4.jar"
meta="$(mktemp -d)"
trap 'rm -rf "$meta"; rm -f "$root/boot.probed.cfg"' EXIT

if [[ ! -f "$jar" ]]; then
  mkdir -p "$cache"
  curl -fsSL -o "$jar.partial" \
    https://github.com/tlaplus/tlaplus/releases/download/v1.7.4/tla2tools.jar
  echo "$jar_sum  $jar.partial" | sha256sum -c -
  mv "$jar.partial" "$jar"
fi

tla_bool() {
  case "$1" in
    true) echo TRUE ;;
    false) echo FALSE ;;
    *)
      echo "check: expected true or false, got '$1'" >&2
      exit 2
      ;;
  esac
}

field() {
  local text="$1"
  local key="$2"
  local line
  line="$(printf '%s\n' "$text" | sed -n "s/^${key}=//p" | head -n 1)"
  if [[ -z "$line" ]]; then
    echo "check: probe did not print ${key}" >&2
    printf '%s\n' "$text" >&2
    exit 2
  fi
  printf '%s' "$line"
}

write_cfg() {
  local probe="$1"
  local cfg="$2"
  if printf '%s\n' "$probe" | grep -q '^error='; then
    printf '%s\n' "$probe"
    exit 2
  fi
  cat >"$cfg" <<EOF
SPECIFICATION Spec
CONSTANTS
  OnlyForward = $(tla_bool "$(field "$probe" only_forward)")
  ResetOkNeedsFirmware = $(tla_bool "$(field "$probe" reset_ok_needs_firmware)")
  ResetOkNeedsEvent = $(tla_bool "$(field "$probe" reset_ok_needs_event)")
  PortIsNotResetOk = $(tla_bool "$(field "$probe" port_is_not_reset_ok)")
  EmptyIsFirmwareErr = $(tla_bool "$(field "$probe" empty_is_firmware_err)")
  VerPaintsWord = $(tla_bool "$(field "$probe" ver_paints_word)")
  IdPaintsDevice = $(tla_bool "$(field "$probe" id_paints_device)")
  StageLabels = $(tla_bool "$(field "$probe" stage_labels)")
  PaintsVersion = $(tla_bool "$(field "$probe" paints_version)")
  PaintsFirmware = $(tla_bool "$(field "$probe" paints_firmware)")
  PaintsBringUp = $(tla_bool "$(field "$probe" paints_bring_up)")
  PaintsFail = $(tla_bool "$(field "$probe" paints_fail)")
  WitnessCount = $(field "$probe" witness_count)
  WitnessRank = $(field "$probe" witness_rank)
  VerLo = $(field "$probe" ver_lo)
  VerHi = $(field "$probe" ver_hi)
INVARIANT PortLine
INVARIANT PortIsNotSuccess
INVARIANT ResetOkSeparate
INVARIANT EmptyBlob
INVARIANT VersionWord
INVARIANT IdDevice
INVARIANT FailureUsesFormatter
INVARIANT Labels
PROPERTY EventuallyDone
EOF
}

run_replay() {
  CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$meta/target}" RUSTFLAGS="-D warnings" \
    cargo run --quiet --manifest-path "$root/replay/Cargo.toml" -- "$@"
}

run_tlc() {
  local cfg="$1"
  local log="$2"
  # The resolver joins the spec directory to the config name. A file outside
  # that directory is reported missing even when the absolute path exists.
  local placed="$root/boot.probed.cfg"
  cp "$cfg" "$placed"
  mkdir -p "$meta/tlc"
  java -XX:+UseParallelGC -cp "$jar" tlc2.TLC -nowarning -workers 1 \
    -metadir "$meta/tlc" -config "$placed" \
    "$root/BtStage.tla" >"$log" 2>&1
  local status=$?
  rm -f "$placed"
  return "$status"
}

# rustup selects the toolchain from the current directory. The repo pins a
# nightly that understands edition 2024. Stay there for every Cargo call.
cd "$repo"
set +e
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$meta/target}" RUSTFLAGS="-D warnings" \
  cargo test --quiet --manifest-path "$root/replay/Cargo.toml" >"$meta/test.txt" 2>&1
test_status=$?
set -e
cat "$meta/test.txt"
if [[ "$test_status" -ne 0 ]]; then
  echo "check: replay tests failed"
  exit 2
fi

probe="$(run_replay probe)"
printf '%s\n' "$probe"
write_cfg "$probe" "$meta/boot.cfg"

set +e
run_replay check >"$meta/rust.txt" 2>&1
rust_status=$?
run_tlc "$meta/boot.cfg" "$meta/tlc.txt"
tlc_status=$?
set -e

echo "----- Rust (exit $rust_status) -----"
cat "$meta/rust.txt"
echo "----- TLC (exit $tlc_status) -----"
cat "$meta/tlc.txt"

if grep -q '^verdict=line$\|^verdict=paint$' "$meta/rust.txt" || [[ "$rust_status" -eq 1 ]]; then
  echo "check: the stage line disagrees with the milestone"
  exit 1
fi

if ! cmp -s "$meta/boot.cfg" "$root/boot.cfg"; then
  echo "check: probed constants differ from boot.cfg"
  diff -u "$root/boot.cfg" "$meta/boot.cfg" || true
  exit 2
fi

if [[ "$rust_status" -ne 0 || "$tlc_status" -ne 0 ]]; then
  echo "check: the Rust schedule and TLC disagree, or a run could not be read"
  exit 2
fi

echo "check: bt: port 3 desc stays a finished stage line, and bt: reset ok still waits on a command-complete event"
exit 0
