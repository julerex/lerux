#!/usr/bin/env bash
# Check the agent browse handshake.
# Exit 0: a fitting name returns ok, and a name that does not fit returns err.
# Exit 1: the shell stays blocked, or the finished page is wrong.
# Exit 2: the source shape is not one this model can read, the probed
# constants differ from the committed configs, or TLC disagrees with a Rust
# run that itself accepted the schedule.
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$root/../../.." && pwd)"
jar_sum="936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88"
cache="${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc"
jar="$cache/tla2tools-1.7.4.jar"
meta="$(mktemp -d)"
trap 'rm -rf "$meta"; rm -f "$root/fits.probed.cfg" "$root/reject.probed.cfg"' EXIT

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
  NameLen = $(field "$probe" name_len)
  ExpectOk = $(tla_bool "$(field "$probe" expect_ok)")
  DoNotify = $(tla_bool "$(field "$probe" do_notify)")
  BrowseNotifies = $(tla_bool "$(field "$probe" browse_notifies)")
  WorkerNotifies = $(tla_bool "$(field "$probe" worker_notifies)")
  ReplyState = $(field "$probe" reply_state)
  ReplyLen = $(field "$probe" reply_len)
  FinalState = $(field "$probe" final_state)
  FinalLen = $(field "$probe" final_len)
INVARIANT NotStuck
INVARIANT Result
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
    "$root/AgentBrowse.tla" >"$log" 2>&1
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

fits_probe="$(run_replay probe fits)"
reject_probe="$(run_replay probe reject)"
printf '%s\n' "$fits_probe"
printf '%s\n' "$reject_probe"
write_cfg "$fits_probe" "$meta/fits.cfg"
write_cfg "$reject_probe" "$meta/reject.cfg"

set +e
run_replay check fits >"$meta/fits-rust.txt" 2>&1
fits_rust=$?
run_replay check reject >"$meta/reject-rust.txt" 2>&1
reject_rust=$?
run_tlc "$meta/fits.cfg" "$meta/fits-tlc.txt"
fits_tlc=$?
run_tlc "$meta/reject.cfg" "$meta/reject-tlc.txt"
reject_tlc=$?
set -e

echo "----- Rust fits (exit $fits_rust) -----"
cat "$meta/fits-rust.txt"
echo "----- Rust reject (exit $reject_rust) -----"
cat "$meta/reject-rust.txt"
echo "----- TLC fits (exit $fits_tlc) -----"
cat "$meta/fits-tlc.txt"
echo "----- TLC reject (exit $reject_tlc) -----"
cat "$meta/reject-tlc.txt"

rust_property=0
if grep -q '^verdict=stuck$\|^verdict=result$' "$meta/fits-rust.txt" "$meta/reject-rust.txt"; then
  rust_property=1
fi
if [[ "$fits_rust" -eq 1 || "$reject_rust" -eq 1 ]]; then
  rust_property=1
fi

if [[ "$rust_property" -eq 1 ]]; then
  echo "check: browse leaves the shell blocked, or the finished page is wrong"
  exit 1
fi

if ! cmp -s "$meta/fits.cfg" "$root/fits.cfg" || ! cmp -s "$meta/reject.cfg" "$root/reject.cfg"; then
  echo "check: probed constants differ from fits.cfg or reject.cfg"
  diff -u "$root/fits.cfg" "$meta/fits.cfg" || true
  diff -u "$root/reject.cfg" "$meta/reject.cfg" || true
  exit 2
fi

if [[ "$fits_rust" -ne 0 || "$reject_rust" -ne 0 || "$fits_tlc" -ne 0 || "$reject_tlc" -ne 0 ]]; then
  echo "check: the Rust schedule and TLC disagree, or a run could not be read"
  exit 2
fi

echo "check: a fitting name returns ok, and a name that does not fit returns err"
exit 0
