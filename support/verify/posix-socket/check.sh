#!/usr/bin/env bash
# Check the one-socket table.
# Exit 0: the documented errnos are the only exits, and close leaves the table empty.
# Exit 1: a call returns a different errno, close leaves the entry occupied,
#         or TLC reports a syntax error or a broken property.
# Exit 2: the source is not a shape this model can read, the replay tests fail,
#         or the probed constants differ from socket.cfg.
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$root/../../.." && pwd)"
jar_sum="936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88"
cache="${XDG_CACHE_HOME:-$HOME/.cache}/lerux-tlc"
jar="$cache/tla2tools-1.7.4.jar"
meta="$(mktemp -d)"
trap 'rm -rf "$meta"; rm -f "$root/socket.probed.cfg"' EXIT

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
  FirstFd = $(field "$probe" first_fd)
  SecondSocket = $(field "$probe" second_socket)
  ListenUnsupported = $(field "$probe" listen_unsupported)
  ListenInUse = $(field "$probe" listen_in_use)
  SecondConnect = $(field "$probe" second_connect)
  SendBefore = $(field "$probe" send_before)
  Busy = $(field "$probe" busy)
  StdioStored = $(tla_bool "$(field "$probe" stdio_stored)")
INVARIANT TypeOK
INVARIANT OneSocket
INVARIANT OnlyExits
INVARIANT EmptyAfterClose
INVARIANT ClosedNotEmpty
INVARIANT ListenSameSocket
INVARIANT AcceptWaits
INVARIANT StdioFree
INVARIANT FileBusy
INVARIANT FileEmptyAfterClose
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
  local placed="$root/socket.probed.cfg"
  cp "$cfg" "$placed"
  mkdir -p "$meta/socket-meta"
  java -XX:+UseParallelGC -cp "$jar" tlc2.TLC -nowarning -workers 1 \
    -metadir "$meta/socket-meta" -config "$placed" \
    "$root/PosixSocket.tla" >"$log" 2>&1
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

set +e
probe_text="$(run_replay probe 2>"$meta/probe.err")"
probe_status=$?
set -e
cat "$meta/probe.err" >&2
printf '%s\n' "$probe_text"
if [[ "$probe_status" -ne 0 ]]; then
  echo "check: probe could not read the source"
  exit 2
fi
write_cfg "$probe_text" "$meta/socket.cfg"

set +e
run_replay check >"$meta/rust.txt" 2>&1
rust_status=$?
run_tlc "$meta/socket.cfg" "$meta/tlc.txt"
tlc_status=$?
set -e

echo "----- Rust (exit $rust_status) -----"
cat "$meta/rust.txt"
echo "----- TLC (exit $tlc_status) -----"
cat "$meta/tlc.txt"

if grep -q '^verdict=errno$\|^verdict=table$' "$meta/rust.txt" || [[ "$rust_status" -eq 1 ]]; then
  echo "check: an exit errno disagrees, or close leaves the table occupied"
  exit 1
fi

if ! cmp -s "$meta/socket.cfg" "$root/socket.cfg"; then
  echo "check: probed constants differ from socket.cfg"
  diff -u "$root/socket.cfg" "$meta/socket.cfg" || true
  exit 2
fi

if [[ "$tlc_status" -ne 0 ]]; then
  echo "check: TLC reported a syntax error or a broken property"
  exit 1
fi

if [[ "$rust_status" -ne 0 ]]; then
  echo "check: the replay could not be read"
  exit 2
fi

echo "check: the one-socket table returns the documented errnos, and close leaves it empty"
exit 0
