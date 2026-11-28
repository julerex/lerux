#!/usr/bin/env bash
# Check the sDDF client transmit backpressure model.
# Exit 0: both models accept every byte and the client returns.
# Exit 1: byte order holds, and the client blocks with nothing left to wake it.
# Exit 2: the two models disagree, a short emit fails, or a byte is lost.
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
jar_sum="936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88"
cache="${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc"
jar="$cache/tla2tools-1.7.4.jar"
meta="$(mktemp -d)"
trap 'rm -rf "$meta"' EXIT

if [[ ! -f "$jar" ]]; then
  mkdir -p "$cache"
  curl -fsSL -o "$jar.partial" \
    https://github.com/tlaplus/tlaplus/releases/download/v1.7.4/tla2tools.jar
  echo "$jar_sum  $jar.partial" | sha256sum -c -
  mv "$jar.partial" "$jar"
fi

run_tlc() {
  local cfg="$1"
  local log="$2"
  java -XX:+UseParallelGC -cp "$jar" tlc2.TLC -nowarning -workers 1 \
    -metadir "$meta/$cfg" -config "$root/$cfg" \
    "$root/SerialTx.tla" >"$log" 2>&1
}

rust_log="$meta/rust.txt"
tlc_fits="$meta/fits.txt"
tlc_over="$meta/overflow.txt"
mkdir -p "$meta/fits.cfg" "$meta/overflow.cfg"

set +e
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$meta/target}" RUSTFLAGS="-D warnings" \
  cargo run --quiet --manifest-path "$root/replay/Cargo.toml" -- fits \
  >"$rust_log" 2>&1
rust_fits=$?
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$meta/target}" RUSTFLAGS="-D warnings" \
  cargo run --quiet --manifest-path "$root/replay/Cargo.toml" -- overflow \
  >>"$rust_log" 2>&1
rust_over=$?
run_tlc fits.cfg "$tlc_fits"
tlc_fits_status=$?
run_tlc overflow.cfg "$tlc_over"
tlc_over_status=$?
set -e

cat "$rust_log"
echo "----- TLC fits (exit $tlc_fits_status) -----"
cat "$tlc_fits"
echo "----- TLC overflow (exit $tlc_over_status) -----"
cat "$tlc_over"

order_broken=0
if grep -q "Invariant Order is violated" "$tlc_fits" "$tlc_over"; then
  order_broken=1
fi
not_stuck_broken=0
if grep -q "Invariant NotStuck is violated" "$tlc_over"; then
  not_stuck_broken=1
fi

if [[ "$rust_fits" -ne 0 || "$tlc_fits_status" -ne 0 || "$order_broken" -ne 0 ]]; then
  echo "check: short emit or byte order failed"
  exit 2
fi

if [[ "$rust_over" -eq 1 && "$tlc_over_status" -ne 0 && "$not_stuck_broken" -eq 1 ]]; then
  echo "check: client blocks after the ring fills, and no virtualiser step is enabled"
  exit 1
fi

if [[ "$rust_over" -eq 0 && "$tlc_over_status" -eq 0 ]]; then
  echo "check: every accepted byte reaches the UART and the client returns"
  exit 0
fi

echo "check: the Rust schedule and the TLA model disagree"
exit 2
