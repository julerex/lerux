#!/usr/bin/env bash
# Shared runner for one implicit-machine check.
# Arguments: spec-dir module probe-id Const:POLARITY,... Invariant,...
# Exit 0: the source and the model agree, and every invariant holds.
# Exit 1: the source broke the property (a probe constant flipped, a host
# call disagreed, or TLC found a violating state for that flipped constant).
# Exit 2: the probe could not read the source, the committed config drifted,
# or TLC failed while the source still matched the committed config.
set -euo pipefail

spec_dir="$(cd "$1" && pwd)"
module="$2"
probe_id="$3"
const_spec="$4"
inv_spec="$5"
root="$(cd "$spec_dir/../../.." && pwd)"
replay="$root/support/verify/implicit/replay/Cargo.toml"
jar_sum="936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88"
cache="${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc"
jar="$cache/tla2tools-1.7.4.jar"
meta="$(mktemp -d)"
probed="$spec_dir/$module.probed.cfg"
trap 'rm -rf "$meta"; rm -f "$probed"' EXIT

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

run_replay() {
  CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/lerux-implicit-target}" \
    RUSTFLAGS="-D warnings" \
    cargo run --quiet --manifest-path "$replay" -- "$@"
}

# rustup selects the toolchain from the current directory. The repo pins a
# nightly that understands edition 2024. Stay there for every Cargo call.
cd "$root"

run_tlc() {
  local cfg="$1"
  java -XX:+UseParallelGC -cp "$jar" tlc2.TLC \
    -nowarning -workers 1 -metadir "$meta/tlc" \
    -config "$cfg" "$spec_dir/$module.tla"
}

test_code=0
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/lerux-implicit-target}" \
  RUSTFLAGS="-D warnings" \
  cargo test --quiet --manifest-path "$replay" || test_code=$?
if [[ $test_code -ne 0 ]]; then
  echo "check: replay tests failed" >&2
  exit 2
fi

if ! probe_out="$(run_replay probe "$probe_id")"; then
  printf '%s\n' "$probe_out" >&2
  exit 2
fi

check_code=0
check_out="$(run_replay check "$probe_id" 2>&1)" || check_code=$?
if [[ $check_code -ne 0 && $check_code -ne 1 ]]; then
  printf '%s\n' "$check_out" >&2
  exit 2
fi

bad=0
lines=()
IFS=',' read -ra pairs <<< "$const_spec"
for pair in "${pairs[@]}"; do
  name="${pair%%:*}"
  expect="${pair##*:}"
  got="$(tla_bool "$(field "$probe_out" "$name")")"
  lines+=("$(printf '  %s = %s' "$name" "$got")")
  if [[ "$got" != "$expect" ]]; then
    bad=1
  fi
done
{
  echo "SPECIFICATION Spec"
  echo "CONSTANTS"
  printf '%s\n' "${lines[@]}"
  IFS=',' read -ra invs <<< "$inv_spec"
  for inv in "${invs[@]}"; do
    echo "INVARIANT $inv"
  done
} >"$probed"

if [[ $bad -eq 1 ]]; then
  printf '%s\n' "$check_out" >&2
  tlc_code=0
  run_tlc "$probed" || tlc_code=$?
  if [[ $tlc_code -eq 0 ]]; then
    echo "check: the source broke the property and the model still passed" >&2
    exit 2
  fi
  exit 1
fi

if ! cmp -s "$probed" "$spec_dir/$module.cfg"; then
  echo "check: probed constants differ from $module.cfg" >&2
  diff -u "$spec_dir/$module.cfg" "$probed" >&2 || true
  exit 2
fi

if [[ $check_code -eq 1 ]]; then
  printf '%s\n' "$check_out" >&2
  exit 1
fi

tlc_code=0
run_tlc "$spec_dir/$module.cfg" || tlc_code=$?
if [[ $tlc_code -ne 0 ]]; then
  echo "check: TLC failed while the source still matched the model" >&2
  exit 2
fi
