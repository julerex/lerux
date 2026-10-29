#!/usr/bin/env bash
# Host fmt, clippy, and tests. CARGO_TARGET_DIR is build/host.
# This file is the repro artifact for a red host-check job.
# Re-run it from the repo root: bash support/host-check.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export CARGO_TARGET_DIR="${ROOT}/build/host"

cargo fmt --all --check
cargo clippy -p lerux-cli -p lerux-tls -p lerux-html -p lerux-input -p lerux-desk -p lerux-swift -p lerux-web -p lerux-ladybird-url -p lerux-ladybird-gfx -p lerux-ladybird-html -p lerux-ladybird-web -p lerux-ladybird-http -p lerux-ladybird-js -p lerux-ladybird-image -p lerux-wasm -p lerux-rtw8852be -p lerux-sddf -p lerux-ring -p lerux-cothread -p lerux-calc -p lerux-gate -p lerux-posix -p lerux-serial-queue --all-targets -- -D warnings
# The console-driver binary is no_std. Host clippy builds its tests only.
cargo clippy -p console-driver --tests -- -D warnings
cargo clippy -p lerux-machine --features system-description --all-targets -- -D warnings
cargo clippy -p lerux-image --all-targets -- -D warnings
cargo test -p lerux-tls -p lerux-html -p lerux-input -p lerux-desk -p lerux-swift -p lerux-web -p lerux-ladybird-url -p lerux-ladybird-gfx -p lerux-ladybird-html -p lerux-ladybird-web -p lerux-ladybird-http -p lerux-ladybird-js -p lerux-ladybird-image -p lerux-wasm -p lerux-rtw8852be -p lerux-cli -p lerux-sddf -p lerux-ring -p lerux-cothread -p lerux-calc -p lerux-gate -p lerux-posix -p lerux-serial-queue -p console-driver
python3 support/check-unsafe.py --self-test
python3 support/check-unsafe.py
python3 support/check-shared-crates.py
python3 support/check-public-api.py
python3 support/check-spawn.py --self-test
python3 support/check-plan-owner.py --self-test
python3 support/check-sibling-smoke.py --self-test
python3 support/check-sibling-smoke.py
cargo test -p lerux-machine --features system-description
cargo test -p lerux-image
# Phase 60 Track D: PPC priority + service-class band checks (host, no QEMU)
cargo run -q -p lerux-cli -- profile check-qos
