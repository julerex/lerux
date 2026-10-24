# lerux — Rust userspace on seL4 Microkit
# Kernel built from upstream repos in deps/workspace/ (not vendored).

build_dir := env_var_or_default("BUILD", "build")
board := env_var_or_default("BOARD", "qemu_virt_aarch64")
config := env_var_or_default("CONFIG", "debug")
board_build := build_dir + "/" + board
system_file := board_build + "/system.system"

root := justfile_directory()
workspace := root + "/deps/workspace"
sdk_path_file := root + "/deps/.sdk-path"
lerux := "cargo run -q -p lerux-cli --"

export RUST_TARGET_PATH := root + "/support/targets"
export RUSTC_BOOTSTRAP := "1"

default: build

# Format, clippy, and host tests for host crates (no SDK required).
# The command is support/host-check.sh. It sets CARGO_TARGET_DIR=build/host.
check:
    bash support/host-check.sh

# One host-only crate. Does not start the protection-domain clippy gate.
check-host crate:
    CARGO_TARGET_DIR={{root}}/build/host cargo clippy -p {{crate}} --all-targets -- -D warnings
    CARGO_TARGET_DIR={{root}}/build/host cargo test -p {{crate}}

test-host crate:
    CARGO_TARGET_DIR={{root}}/build/host cargo test -p {{crate}}

# Cross-target clippy for PD + shared userspace crates (requires SDK)
check-pd:
    {{lerux}} clippy

check-all: check check-pd

# Prove userspace/crates/lerux-ring with the pinned Verus release.
# Downloads the release into deps/verus on first use. A host whose C library
# is older than the release binary re-runs the proof inside Ubuntu 24.04.
verify-ring:
    bash support/verify-ring.sh

# Rewrite support/lionsos-layout.toml from the LionsOS 0.4.0 headers.
lionsos-layout:
    LERUX_LIONSOS_LAYOUT=1 LERUX_LIONSOS_LAYOUT_WRITE=1 cargo test -p lerux-sddf --test layout

# Clone seL4 + microkit into deps/workspace/
fetch:
    {{lerux}} fetch

# Build Microkit SDK from source (compiles seL4 kernel per board; needs aarch64-none-elf-gcc)
build-sdk: fetch
    {{lerux}} build-sdk

# Download official prebuilt Microkit SDK (fallback when ARM toolchain is unavailable)
fetch-sdk:
    {{lerux}} fetch-sdk

# Resolve MICROKIT_SDK path (written by build-sdk, or set explicitly)
sdk-path:
    {{lerux}} sdk-path

# Render board-specific Microkit system description
system:
    {{lerux}} system --board {{board}} -o {{system_file}}

# Build all protection-domain ELFs for the selected board
build: system
    {{lerux}} build --board {{board}} --build-dir {{build_dir}} --config {{config}}

build-pd crate:
    {{lerux}} build-pd {{crate}} --board {{board}} --build-dir {{build_dir}} --config {{config}}

# Assemble loader.img via the Microkit tool
image: build
    {{lerux}} image --board {{board}} --build-dir {{build_dir}} --config {{config}}

# Boot in QEMU for the selected board
run: image
    {{lerux}} run --board {{board}} --build-dir {{build_dir}} --config {{config}}

# Serial and filesystem shell in QEMU. Quit with Ctrl-A x.
qemu: disk-img
    BOARD=qemu_virt_aarch64_fs_sddf just run

# Same shell on the x86-64 (q35) filesystem image.
qemu-x86-64: disk-img
    BOARD=x86_64_generic_fs_sddf just run

alias qemu-x86 := qemu-x86-64

# Bare `just test` boots the serial and filesystem image.
# A board with a disk rebuilds support/disk.img before QEMU, so a stale
# local format cannot fail mkdir with "already exists".
# `BOARD=... just test` still smokes that board. `just build` keeps the hello default.
test:
    #!/usr/bin/env bash
    set -euo pipefail
    board="${BOARD:-qemu_virt_aarch64_fs_sddf}"
    export BOARD="$board"
    just image
    {{lerux}} test --board "$board" --build-dir {{build_dir}} --config {{config}}

# Phase 47: force hardware serial smoke (requires LERUX_HW_SERIAL=/dev/ttyUSB0 etc.)
# Example: LERUX_HW_SERIAL=/dev/ttyUSB0 just test-hw
# Optional: BOARD=rpi4b_4gb LERUX_HW_BAUD=115200 LERUX_HW_LOCK_DIR=/tmp/locks just test-hw
test-hw:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -z "${LERUX_HW_SERIAL:-}" ]]; then
      echo "error: LERUX_HW_SERIAL is required (e.g. /dev/ttyUSB0)" >&2
      exit 1
    fi
    BOARD="${BOARD:-rpi4b_4gb}"
    {{lerux}} test --board "$BOARD" --build-dir {{build_dir}} --config {{config}} --mode hw-serial

# Virtio smoke test (serial + virtio-blk + virtio-net on aarch64 virt)
test-virtio:
    BOARD=qemu_virt_aarch64_virtio just test

# Custom IPC smoke test (echo-server + echo-client on aarch64 virt)
# LionsOS serial roles: driver, transmit virtualiser, receive virtualiser, one client
test-serial-sddf:
    BOARD=qemu_virt_aarch64_serial_sddf just test

test-serial-sddf-riscv:
    BOARD=qemu_virt_riscv64_serial_sddf just test

test-serial-sddf-x86:
    BOARD=x86_64_generic_serial_sddf just test

# LionsOS block and filesystem roles: driver, virtualiser, FAT server, one client
test-fs-sddf:
    BOARD=qemu_virt_aarch64_fs_sddf just test

test-fs-sddf-riscv:
    BOARD=qemu_virt_riscv64_fs_sddf just test

test-fs-sddf-x86:
    BOARD=x86_64_generic_fs_sddf just test

test-x86-ahci:
    BOARD=x86_64_generic_fs_ahci just test

# LionsOS network roles: driver, virtualisers, one copier, one smoltcp client
test-net-sddf:
    BOARD=qemu_virt_aarch64_net_sddf just test

test-net-sddf-riscv:
    BOARD=qemu_virt_riscv64_net_sddf just test

test-net-sddf-x86:
    BOARD=x86_64_generic_net_sddf just test

# Milestone 8: shell fetches the https-one fixture over the client's smoltcp
test-fetch-sddf:
    BOARD=qemu_virt_aarch64_fetch_sddf just test

test-fetch-sddf-riscv:
    BOARD=qemu_virt_riscv64_fetch_sddf just test

test-fetch-sddf-x86:
    BOARD=x86_64_generic_fetch_sddf just test

# Milestone 11: shell serves the seeded hostname file over hostfwd
test-web-sddf:
    BOARD=qemu_virt_aarch64_web_sddf just test

test-web-sddf-riscv:
    BOARD=qemu_virt_riscv64_web_sddf just test

test-web-sddf-x86:
    BOARD=x86_64_generic_web_sddf just test

# Milestone 12: shell fetches the fixture and paints through the graphics client
test-browser-sddf:
    BOARD=qemu_virt_aarch64_browser_sddf just test

test-browser-sddf-riscv:
    BOARD=qemu_virt_riscv64_browser_sddf just test

# Milestone 13: grok notifies the agent, which writes through the filesystem queue
test-agent-sddf:
    BOARD=qemu_virt_aarch64_agent_sddf just test

test-agent-sddf-riscv:
    BOARD=qemu_virt_riscv64_agent_sddf just test

test-agent-sddf-x86:
    BOARD=x86_64_generic_agent_sddf just test

# Milestone 14: signed ordinary WebAssembly with filesystem and socket access
test-program-sddf:
    BOARD=qemu_virt_aarch64_program_sddf just test

test-program-sddf-riscv:
    BOARD=qemu_virt_riscv64_program_sddf just test

test-program-sddf-x86:
    BOARD=x86_64_generic_program_sddf just test

# Milestone 3: cothread blocks on a channel until a Microkit notification
test-cothread:
    BOARD=qemu_virt_aarch64_cothread just test

test-cothread-riscv:
    BOARD=qemu_virt_riscv64_cothread just test

# Virtio smoke test on x86_64 q35 (PCI virtio-blk + virtio-net)
test-x86-virtio:
    BOARD=x86_64_generic_virtio just test

# HTTP smoke test on x86_64 q35 (PCI virtio-net)
test-x86-http:
    BOARD=x86_64_generic_http just test

# RISC-V virt smoke test (NS16550 MMIO UART)
test-riscv:
    BOARD=qemu_virt_riscv64 just test

# RISC-V virtio smoke test
test-riscv-virtio:
    BOARD=qemu_virt_riscv64_virtio just test

# RISC-V HTTP smoke test (virtio-net + hostfwd)
test-riscv-http:
    BOARD=qemu_virt_riscv64_http just test

# Phase 74: subset HTML tokenizer + DOM dump
test-html:
    BOARD=qemu_virt_aarch64_html just test

# Phase 46 / 66: parent fault handler + deliberate child crash
test-debug:
    BOARD=qemu_virt_aarch64_debug just test

test-debug-riscv:
    BOARD=qemu_virt_riscv64_debug just test

test-debug-x86:
    BOARD=x86_64_generic_debug just test

# Hardware slice (Phase 37): build image (no QEMU).
# `just test` / `lerux test` for hardware boards does build verification + manual note.
# Deploy e.g. build/rpi4b_4gb/loader.img via U-Boot on RPi.
hardware-rpi4:
    BOARD=rpi4b_4gb just image

# Verify build + manual smoke path for the hardware board
test-hardware-rpi4:
    BOARD=rpi4b_4gb just test

# HTTP smoke: GET / on virtio-net (host port 18080 -> guest :8080)
test-http:
    BOARD=qemu_virt_aarch64_http just test

# Run all CI smoke tests (requires SDK with aarch64 + x86_64 + riscv64 boards)
test-all:
    {{lerux}} test-all --build-dir {{build_dir}} --config {{config}}

# Host microbench runner. No boards are registered.
# Writes build/bench/bench-results.{md,json} and docs/bench-results.latest.md
bench:
    just disk-img
    {{lerux}} bench --build-dir {{build_dir}} --config {{config}}

# Phase 57: microbenches + threshold gate (support/bench-thresholds.toml)
bench-check:
    just disk-img
    {{lerux}} bench --build-dir {{build_dir}} --config {{config}} --check

# Disk image for virtio-blk QEMU device (MBR boot signature at bytes 510–511)
disk-img:
    {{lerux}} disk-img

# Phase 54: print config schema / QEMU defaults / preseed support/disk.img
config-schema:
    {{lerux}} config schema

config-seed-disk:
    {{lerux}} config seed-disk

# Remove all build artifacts (shared target cache + per-board outputs).
clean:
    rm -rf {{build_dir}} target deps/.sdk-path

# Drop per-board outputs (system.system, loader.img, *.elf) but keep build/target/.
prune-boards:
    #!/usr/bin/env bash
    set -euo pipefail
    for dir in {{build_dir}}/*; do
        [[ -d "$dir" ]] || continue
        base="$(basename "$dir")"
        [[ "$base" == "target" || "$base" == "host" ]] && continue
        rm -rf "$dir"
    done

# Remove legacy per-arch clippy target trees (pre shared-target layout).
clean-legacy:
    rm -rf {{build_dir}}/clippy

clean-deps:
    rm -rf deps/workspace deps/.sdk-path