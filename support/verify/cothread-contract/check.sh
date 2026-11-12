#!/usr/bin/env bash
# Check the cothread contract.
# Exit 0: the waiter runs before signal and recv return, a full pool is
# NULL_HANDLE, and the contract breaks have no next state.
# Exit 1: the runtime disagrees with that contract.
# Exit 2: the source shape cannot be read, the probed constants differ from
# the committed configs, the replay tests fail, or TLC disagrees with a Rust
# run that itself accepted the schedule.
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$root/../../.." && pwd)"
jar_sum="936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88"
cache="${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc"
jar="$cache/tla2tools-1.7.4.jar"
meta="$(mktemp -d)"
trap 'rm -rf "$meta"; rm -f "$root/wake.probed.cfg" "$root/full.probed.cfg"' EXIT

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
  local mode="$1"
  local probe="$2"
  local cfg="$3"
  if printf '%s\n' "$probe" | grep -q '^error='; then
    printf '%s\n' "$probe"
    exit 2
  fi
  # TLC's config parser rejects a negative literal, so -1 is MinusOne in the module.
  if [[ "$(field "$probe" null_handle)" != "-1" ]]; then
    echo "check: probed null_handle is not -1"
    exit 2
  fi
  cat >"$cfg" <<EOF
SPECIFICATION Spec
CONSTANTS
  Mode = "${mode}"
  Pool = $(field "$probe" pool)
  NullHandle <- MinusOne
  RootHandle = $(field "$probe" root_handle)
  RecvSwitched = $(tla_bool "$(field "$probe" recv_switched)")
  SignalSwitched = $(tla_bool "$(field "$probe" signal_switched)")
  FullReturnsNull = $(tla_bool "$(field "$probe" full_returns_null)")
  SecondInitPanics = $(tla_bool "$(field "$probe" second_init_panics)")
  BadHandlePanics = $(tla_bool "$(field "$probe" bad_handle_panics)")
  RootWaitPanics = $(tla_bool "$(field "$probe" root_wait_panics)")
  WorkerRecvPanics = $(tla_bool "$(field "$probe" worker_recv_panics)")
  DestroyBlockedPanics = $(tla_bool "$(field "$probe" destroy_blocked_panics)")
  LockAcrossSwitch = $(tla_bool "$(field "$probe" lock_across_switch)")
  WorkerPanicked = $(tla_bool "$(field "$probe" worker_panicked)")
INVARIANT TypeOK
INVARIANT NoForbidden
INVARIANT SwitchBeforeReturn
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
  local stem
  stem="$(basename "$cfg" .cfg)"
  # The resolver joins the spec directory to the config name. A file outside
  # that directory is reported missing even when the absolute path exists.
  local placed="$root/$stem.probed.cfg"
  cp "$cfg" "$placed"
  mkdir -p "$meta/$stem-meta"
  java -XX:+UseParallelGC -cp "$jar" tlc2.TLC -nowarning -workers 1 \
    -metadir "$meta/$stem-meta" -config "$placed" \
    "$root/CothreadContract.tla" >"$log" 2>&1
  local status=$?
  rm -f "$placed"
  return "$status"
}

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

wake_probe="$(run_replay probe wake)"
full_probe="$(run_replay probe full)"
printf '%s\n' "$wake_probe"
printf '%s\n' "$full_probe"
write_cfg wake "$wake_probe" "$meta/wake.cfg"
write_cfg full "$full_probe" "$meta/full.cfg"

set +e
run_replay check wake >"$meta/wake-rust.txt" 2>&1
wake_rust=$?
run_replay check full >"$meta/full-rust.txt" 2>&1
full_rust=$?
run_tlc "$meta/wake.cfg" "$meta/wake-tlc.txt"
wake_tlc=$?
run_tlc "$meta/full.cfg" "$meta/full-tlc.txt"
full_tlc=$?
set -e

echo "----- Rust wake (exit $wake_rust) -----"
cat "$meta/wake-rust.txt"
echo "----- Rust full (exit $full_rust) -----"
cat "$meta/full-rust.txt"
echo "----- TLC wake (exit $wake_tlc) -----"
cat "$meta/wake-tlc.txt"
echo "----- TLC full (exit $full_tlc) -----"
cat "$meta/full-tlc.txt"

rust_property=0
if grep -q '^verdict=contract$' "$meta/wake-rust.txt" "$meta/full-rust.txt"; then
  rust_property=1
fi
if [[ "$wake_rust" -eq 1 || "$full_rust" -eq 1 ]]; then
  rust_property=1
fi

if [[ "$rust_property" -eq 1 ]]; then
  echo "check: the runtime disagrees with the cothread contract"
  exit 1
fi

if ! cmp -s "$meta/wake.cfg" "$root/wake.cfg" || ! cmp -s "$meta/full.cfg" "$root/full.cfg"; then
  echo "check: probed constants differ from wake.cfg or full.cfg"
  diff -u "$root/wake.cfg" "$meta/wake.cfg" || true
  diff -u "$root/full.cfg" "$meta/full.cfg" || true
  exit 2
fi

if [[ "$wake_rust" -ne 0 || "$full_rust" -ne 0 || "$wake_tlc" -ne 0 || "$full_tlc" -ne 0 ]]; then
  echo "check: the Rust schedule and TLC disagree, or a run could not be read"
  exit 2
fi

echo "check: the waiter runs before signal and recv return, a full pool is NULL_HANDLE, and the contract breaks have no next state"
exit 0
