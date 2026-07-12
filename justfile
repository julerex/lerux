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

# Format and clippy for host crates (no SDK required)
check:
    cargo fmt --all --check
    CARGO_TARGET_DIR={{root}}/build/host cargo clippy -p lerux-cli -p lerux-interface-types --all-targets -- -D warnings

# Cross-target clippy for PD + shared userspace crates (requires SDK)
check-pd:
    {{lerux}} clippy

check-all: check check-pd

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

# Serial smoke test (QEMU or hw-serial when LERUX_HW_SERIAL is set on hardware boards)
test: image
    {{lerux}} test --board {{board}} --build-dir {{build_dir}} --config {{config}}

# Phase 47: force hardware serial smoke (requires LERUX_HW_SERIAL=/dev/ttyUSB0 etc.)
# Example: LERUX_HW_SERIAL=/dev/ttyUSB0 just test-hw
# Optional: BOARD=rpi4b_4gb_net LERUX_HW_BAUD=115200 LERUX_HW_LOCK_DIR=/tmp/locks just test-hw
test-hw:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -z "${LERUX_HW_SERIAL:-}" ]]; then
      echo "error: LERUX_HW_SERIAL is required (e.g. /dev/ttyUSB0)" >&2
      exit 1
    fi
    BOARD="${BOARD:-rpi4b_4gb_workstation}"
    {{lerux}} test --board "$BOARD" --build-dir {{build_dir}} --config {{config}} --mode hw-serial

# Virtio smoke test (serial + virtio-blk + virtio-net on aarch64 virt)
test-virtio:
    BOARD=qemu_virt_aarch64_virtio just test

# Custom IPC smoke test (echo-server + echo-client on aarch64 virt)
test-echo:
    BOARD=qemu_virt_aarch64_echo just test

# Echo IPC smoke test on x86_64 generic PC
test-x86-echo:
    BOARD=x86_64_generic_echo just test

# Virtio smoke test on x86_64 q35 (PCI virtio-blk + virtio-net)
test-x86-virtio:
    BOARD=x86_64_generic_virtio just test

# HTTP smoke test on x86_64 q35 (PCI virtio-net)
test-x86-http:
    BOARD=x86_64_generic_http just test

# RISC-V virt smoke test (NS16550 MMIO UART)
test-riscv:
    BOARD=qemu_virt_riscv64 just test

# RISC-V echo IPC smoke test
test-riscv-echo:
    BOARD=qemu_virt_riscv64_echo just test

# RISC-V virtio smoke test
test-riscv-virtio:
    BOARD=qemu_virt_riscv64_virtio just test

# RISC-V HTTP smoke test (virtio-net + hostfwd)
test-riscv-http:
    BOARD=qemu_virt_riscv64_http just test

# Block IPC smoke test (blk-server + blk-client on aarch64 virt)
test-blk:
    just disk-img && BOARD=qemu_virt_aarch64_blk just test

# Block IPC smoke test on RISC-V virt
test-riscv-blk:
    just disk-img && BOARD=qemu_virt_riscv64_blk just test

# Block IPC smoke test on x86_64 q35 (PCI virtio-blk)
test-x86-blk:
    just disk-img && BOARD=x86_64_generic_blk just test

# Timer/RTC/init smoke test (PL031 + SP804 via patched QEMU; see support/qemu/)
test-init:
    BOARD=qemu_virt_aarch64_init just test

# Phase 56: init/time on RISC-V virt (Goldfish RTC + rdtime; stock QEMU)
test-init-riscv:
    BOARD=qemu_virt_riscv64_init just test

# Phase 56: init/time on x86_64 (CMOS RTC + TSC; stock QEMU)
test-init-x86:
    BOARD=x86_64_generic_init just test

# Composed smoke: boot-init + hello virtio (both serial IPC; gated on init notify)
test-composed:
    BOARD=qemu_virt_aarch64_composed just test

# Init + block IPC over virtio-blk (boot-init notify gate before blk probe)
test-blk-composed:
    BOARD=qemu_virt_aarch64_blk_composed just test

# Net IPC smoke test (net-server + net-client on aarch64 virt)
test-net:
    BOARD=qemu_virt_aarch64_net just test

# HTTP fetch over net IPC (DNS + TCP via net-server)
test-fetch:
    BOARD=qemu_virt_aarch64_fetch just test

# Filesystem IPC smoke test (fs-server + fs-client on aarch64 virt)
test-fs:
    BOARD=qemu_virt_aarch64_fs just test

# Phase 44: FAT16 backend on same fs SDF / virtio-blk
test-fs-fat:
    BOARD=qemu_virt_aarch64_fs_fat just test

# Phase 46: parent fault handler + deliberate child crash
test-debug:
    BOARD=qemu_virt_aarch64_debug just test

# Net IPC smoke test on RISC-V virt
test-riscv-net:
    BOARD=qemu_virt_riscv64_net just test

# Net IPC smoke test on x86_64 q35 (PCI virtio-net)
test-x86-net:
    BOARD=x86_64_generic_net just test

# Init + net IPC over virtio-net (boot-init notify gate before net probe)
test-net-composed:
    BOARD=qemu_virt_aarch64_net_composed just test

# Init + block and net IPC (boot-init → blk-client → net-client notify chain)
test-ipc-composed:
    BOARD=qemu_virt_aarch64_ipc_composed just test

# Workstation: supervisor + fs + net over virtio (Phase 33)
test-workstation:
    BOARD=qemu_virt_aarch64_workstation just test

# Hardware slice (Phase 37): build image (no QEMU).
# `just test` / `lerux test` for hardware boards does build verification + manual note.
# Deploy e.g. build/rpi4b_4gb/loader.img via U-Boot on RPi.
hardware-rpi4:
    BOARD=rpi4b_4gb just image

# Verify build + manual smoke path for the hardware board
test-hardware-rpi4:
    BOARD=rpi4b_4gb just test

# RPi4 workstation: build image (+ optional serial smoke via LERUX_HW_SERIAL).
# Install path: docs/boards.md#rpi4-workstation-install-path-phase-52
# Prefer: LERUX_HW_SERIAL=/dev/ttyUSB0 just test-hw
test-rpi4-workstation:
    BOARD=rpi4b_4gb_workstation just test

# Phase 52: copy RPi4 workstation loader.img onto a mounted SD FAT boot dir.
# Example: DEST=/media/$USER/boot just deploy-rpi4
# Builds the image if missing. Writes lerux-uboot.txt beside loader.img.
deploy-rpi4 DEST:
    {{lerux}} deploy --board rpi4b_4gb_workstation --dest {{DEST}}

# HTTP smoke: GET / on virtio-net (host port 18080 -> guest :8080)
test-http:
    BOARD=qemu_virt_aarch64_http just test

# Composed + HTTP: boot-init then http-server over virtio-net
test-http-composed:
    BOARD=qemu_virt_aarch64_http_composed just test

# Run all CI smoke tests (requires SDK with aarch64 + x86_64 + riscv64 boards)
test-all:
    {{lerux}} test-all --build-dir {{build_dir}} --config {{config}}

# Phase 49: microbenches (echo RTT, blk IOPS, UDP TX PPS) on QEMU aarch64
# Writes build/bench/bench-results.{md,json} and docs/bench-results.latest.md
bench:
    just disk-img
    {{lerux}} bench --build-dir {{build_dir}} --config {{config}}

# Phase 57: microbenches + threshold gate (support/bench-thresholds.toml)
bench-check:
    just disk-img
    {{lerux}} bench --build-dir {{build_dir}} --config {{config}} --check

# Phase 57: analyze a serial capture (default: last workstation smoke log)
diagnose LOG="build/smoke-logs/qemu_virt_aarch64_workstation.serial.log":
    {{lerux}} diagnose {{LOG}}

# Disk image for virtio-blk QEMU device (MBR boot signature at bytes 510–511)
disk-img:
    {{lerux}} disk-img

# Phase 54: print config schema / QEMU defaults / preseed support/disk.img
config-schema:
    {{lerux}} config schema

config-seed-disk:
    {{lerux}} config seed-disk

# Phase 55: package / profile helpers
package-list:
    {{lerux}} package list

package-search QUERY:
    {{lerux}} package search {{QUERY}}

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