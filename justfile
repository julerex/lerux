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

export RUST_TARGET_PATH := root + "/support/targets"
export RUSTC_BOOTSTRAP := "1"

default: build

# Clone seL4 + microkit into deps/workspace/
fetch:
    bash scripts/fetch.sh

# Build Microkit SDK from source (compiles seL4 kernel per board; needs aarch64-none-elf-gcc)
build-sdk: fetch
    bash scripts/build-sdk.sh

# Download official prebuilt Microkit SDK (fallback when ARM toolchain is unavailable)
fetch-sdk:
    bash scripts/fetch-sdk.sh

# Resolve MICROKIT_SDK path (written by build-sdk, or set explicitly)
sdk-path:
    @if [ -n "${MICROKIT_SDK:-}" ]; then echo "${MICROKIT_SDK}"; \
    elif [ -f {{sdk_path_file}} ]; then cat {{sdk_path_file}}; \
    else echo "error: run 'just build-sdk' or set MICROKIT_SDK" >&2; exit 1; fi

# Render board-specific Microkit system description
system:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{board_build}}"
    python3 "{{root}}/scripts/generate-system.py" \
        --board "{{board}}" \
        -o "{{system_file}}"

# Build all protection-domain ELFs for the selected board
build: system
    #!/usr/bin/env bash
    set -euo pipefail
    pds="$(python3 "{{root}}/scripts/board_config.py" "{{board}}" pds)"
    for crate in ${pds}; do
        just build-pd "${crate}"
    done

build-pd crate:
    #!/usr/bin/env bash
    set -euo pipefail
    sdk="$(just sdk-path)"
    target_triple="$(python3 "{{root}}/scripts/board_config.py" "{{board}}" target_triple)"
    target_spec="{{root}}/support/targets/${target_triple}.json"
    source "{{root}}/scripts/libclang-env.sh"
    mkdir -p "{{board_build}}"
    features=()
    case "{{crate}}" in
        hello|serial-driver|virtio-blk-driver|virtio-net-driver)
            features+=(--features "board-{{board}}")
            ;;
    esac
    microkit_board="$(python3 "{{root}}/scripts/board_config.py" "{{board}}" microkit_board)"
    SEL4_INCLUDE_DIRS="${sdk}/board/${microkit_board}/{{config}}/include" \
        cargo build --release -p {{crate}} \
            "${features[@]}" \
            --target-dir "{{board_build}}/target" \
            --target "${target_spec}" \
            -Z json-target-spec \
            -Z build-std=core,alloc,compiler_builtins \
            -Z build-std-features=compiler-builtins-mem
    cp "{{board_build}}/target/${target_triple}/release/{{crate}}.elf" "{{board_build}}/"

# Assemble loader.img via the Microkit tool
image: build
    #!/usr/bin/env bash
    set -euo pipefail
    sdk="$(just sdk-path)"
    microkit_board="$(python3 "{{root}}/scripts/board_config.py" "{{board}}" microkit_board)"
    "${sdk}/bin/microkit" "{{system_file}}" \
        --search-path "{{board_build}}" \
        --board "${microkit_board}" \
        --config "{{config}}" \
        -r "{{board_build}}/report.txt" \
        -o "{{board_build}}/loader.img"

# Boot in QEMU for the selected board
run: image
    #!/usr/bin/env bash
    set -euo pipefail
    qemu="$(python3 "{{root}}/scripts/board_config.py" "{{board}}" qemu)"
    case "${qemu}" in
        aarch64) just qemu-aarch64 ;;
        aarch64_init) just qemu-aarch64-init ;;
        aarch64_virtio) just qemu-aarch64-virtio ;;
        riscv64) just qemu-riscv64 ;;
        riscv64_virtio) just qemu-riscv64-virtio ;;
        x86_64) just qemu-x86_64 ;;
        *) echo "error: unsupported qemu profile ${qemu}" >&2; exit 1 ;;
    esac

qemu-aarch64:
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="$(bash scripts/host-path.sh)"
    exec qemu-system-aarch64 \
        -machine virt,virtualization=on -cpu cortex-a53 -m size=2G \
        -serial mon:stdio -nographic \
        -device loader,file={{board_build}}/loader.img,addr=0x70000000,cpu-num=0

# aarch64 virt with SP804 timers at 0x90d0000 (patched QEMU; see support/qemu/)
qemu-aarch64-init:
    #!/usr/bin/env bash
    set -euo pipefail
    sp804_qemu="$(bash scripts/install-qemu-sp804.sh)"
    export PATH="${sp804_qemu}:$(bash scripts/host-path.sh)"
    exec qemu-system-aarch64 \
        -machine virt,virtualization=on -cpu cortex-a53 -m size=2G \
        -serial mon:stdio -nographic \
        -device loader,file={{board_build}}/loader.img,addr=0x70000000,cpu-num=0

qemu-aarch64-virtio:
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="$(bash scripts/host-path.sh)"
    disk="{{root}}/support/disk.img"
    if [[ ! -f "${disk}" ]]; then
        echo "error: missing ${disk}; run 'just disk-img'" >&2
        exit 1
    fi
    exec qemu-system-aarch64 \
        -machine virt,virtualization=on -cpu cortex-a53 -m size=2G \
        -serial mon:stdio -nographic \
        -device loader,file={{board_build}}/loader.img,addr=0x70000000,cpu-num=0 \
        -device virtio-net-device,netdev=netdev0 \
        -netdev user,id=netdev0 \
        -device virtio-blk-device,drive=blkdev0 \
        -blockdev node-name=blkdev0,read-only=on,driver=file,filename="${disk}"

qemu-riscv64:
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="$(bash scripts/host-path.sh)"
    exec qemu-system-riscv64 \
        -machine virt -m size=2G \
        -nographic -serial mon:stdio \
        -kernel {{board_build}}/loader.img

qemu-riscv64-virtio:
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="$(bash scripts/host-path.sh)"
    disk="{{root}}/support/disk.img"
    if [[ ! -f "${disk}" ]]; then
        echo "error: missing ${disk}; run 'just disk-img'" >&2
        exit 1
    fi
    exec qemu-system-riscv64 \
        -machine virt -m size=2G \
        -nographic -serial mon:stdio \
        -kernel {{board_build}}/loader.img \
        -device virtio-blk-device,bus=virtio-mmio-bus.0,drive=blkdev0 \
        -blockdev node-name=blkdev0,read-only=on,driver=file,filename="${disk}" \
        -device virtio-net-device,bus=virtio-mmio-bus.1,netdev=netdev0 \
        -netdev user,id=netdev0

qemu-x86_64:
    #!/usr/bin/env bash
    set -euo pipefail
    sdk="$(just sdk-path)"
    microkit_board="$(python3 "{{root}}/scripts/board_config.py" "{{board}}" microkit_board)"
    kernel="${sdk}/board/${microkit_board}/{{config}}/elf/sel4_32.elf"
    if [[ ! -f "${kernel}" ]]; then
        echo "error: missing ${kernel}; run MICROKIT_BOARDS=${microkit_board} just build-sdk" >&2
        exit 1
    fi
    exec qemu-system-x86_64 \
        -cpu qemu64,+fsgsbase,+pdpe1gb,+xsaveopt,+xsave \
        -m 2G \
        -display none \
        -serial mon:stdio \
        -kernel "${kernel}" \
        -initrd {{board_build}}/loader.img

# Serial smoke test
test: image
    #!/usr/bin/env bash
    set -euo pipefail
    qemu="$(python3 "{{root}}/scripts/board_config.py" "{{board}}" qemu)"
    export PATH="$(bash scripts/host-path.sh)"
    case "${qemu}" in
        aarch64)
            expects=(--expect "lerux: Hello from Rust on seL4 Microkit!")
            if [[ "{{board}}" == "qemu_virt_aarch64_echo" ]]; then
                expects=(
                    --expect "lerux-echo: pong"
                    --expect "lerux-echo: lerux"
                )
            fi
            export PATH="$(bash scripts/host-path.sh)"
            exec python3 scripts/test.py \
                "${expects[@]}" \
                qemu-system-aarch64 \
                -machine virt,virtualization=on -cpu cortex-a53 -m size=2G \
                -serial mon:stdio -nographic \
                -device loader,file={{board_build}}/loader.img,addr=0x70000000,cpu-num=0
            ;;
        aarch64_init)
            sp804_qemu="$(bash scripts/install-qemu-sp804.sh)"
            export PATH="${sp804_qemu}:$(bash scripts/host-path.sh)"
            exec python3 scripts/test.py \
                --expect "lerux-init: RTC" \
                --expect "lerux-init: timer ok" \
                --expect "lerux-init: init ok" \
                qemu-system-aarch64 \
                -machine virt,virtualization=on -cpu cortex-a53 -m size=2G \
                -serial mon:stdio -nographic \
                -device loader,file={{board_build}}/loader.img,addr=0x70000000,cpu-num=0
            ;;
        aarch64_virtio)
            disk="{{root}}/support/disk.img"
            if [[ ! -f "${disk}" ]]; then
                just disk-img
            fi
            python3 scripts/tcp-echo-server.py 18080 &
            tcp_echo_pid=$!
            trap 'kill "${tcp_echo_pid}" 2>/dev/null || true' EXIT
            for _ in $(seq 1 100); do
                if python3 scripts/tcp-echo-server.py --probe 18080; then break; fi
                sleep 0.05
            done
            exec python3 scripts/test.py \
                --expect "lerux: Hello from Rust on seL4 Microkit!" \
                --expect "virtio-blk:" \
                --expect "virtio-net: MAC" \
                --expect "virtio-net: TX ok" \
                --expect "virtio-net: TCP RX ok" \
                --expect "virtio-blk: MBR sig" \
                qemu-system-aarch64 \
                -machine virt,virtualization=on -cpu cortex-a53 -m size=2G \
                -serial mon:stdio -nographic \
                -device loader,file={{board_build}}/loader.img,addr=0x70000000,cpu-num=0 \
                -device virtio-net-device,netdev=netdev0 \
                -netdev user,id=netdev0 \
                -device virtio-blk-device,drive=blkdev0 \
                -blockdev node-name=blkdev0,read-only=on,driver=file,filename="${disk}"
            ;;
        riscv64)
            expects=(--expect "lerux: Hello from Rust on seL4 Microkit!")
            if [[ "{{board}}" == "qemu_virt_riscv64_echo" ]]; then
                expects=(
                    --expect "lerux-echo: pong"
                    --expect "lerux-echo: lerux"
                )
            fi
            exec python3 scripts/test.py \
                "${expects[@]}" \
                qemu-system-riscv64 \
                -machine virt -m size=2G \
                -nographic -serial mon:stdio \
                -kernel {{board_build}}/loader.img
            ;;
        riscv64_virtio)
            disk="{{root}}/support/disk.img"
            if [[ ! -f "${disk}" ]]; then
                just disk-img
            fi
            python3 scripts/tcp-echo-server.py 18080 &
            tcp_echo_pid=$!
            trap 'kill "${tcp_echo_pid}" 2>/dev/null || true' EXIT
            for _ in $(seq 1 100); do
                if python3 scripts/tcp-echo-server.py --probe 18080; then break; fi
                sleep 0.05
            done
            exec python3 scripts/test.py \
                --expect "lerux: Hello from Rust on seL4 Microkit!" \
                --expect "virtio-blk:" \
                --expect "virtio-net: MAC" \
                --expect "virtio-net: TX ok" \
                --expect "virtio-net: TCP RX ok" \
                --expect "virtio-blk: MBR sig" \
                qemu-system-riscv64 \
                -machine virt -m size=2G \
                -nographic -serial mon:stdio \
                -kernel {{board_build}}/loader.img \
                -device virtio-blk-device,bus=virtio-mmio-bus.0,drive=blkdev0 \
                -blockdev node-name=blkdev0,read-only=on,driver=file,filename="${disk}" \
                -device virtio-net-device,bus=virtio-mmio-bus.1,netdev=netdev0 \
                -netdev user,id=netdev0
            ;;
        x86_64)
            sdk="$(just sdk-path)"
            microkit_board="$(python3 "{{root}}/scripts/board_config.py" "{{board}}" microkit_board)"
            kernel="${sdk}/board/${microkit_board}/{{config}}/elf/sel4_32.elf"
            expects=(--expect "lerux: Hello from Rust on seL4 Microkit!")
            if [[ "{{board}}" == "x86_64_generic_echo" ]]; then
                expects=(
                    --expect "lerux-echo: pong"
                    --expect "lerux-echo: lerux"
                )
            fi
            exec python3 scripts/test.py \
                "${expects[@]}" \
                qemu-system-x86_64 \
                -cpu qemu64,+fsgsbase,+pdpe1gb,+xsaveopt,+xsave \
                -m 2G \
                -display none \
                -serial mon:stdio \
                -kernel "${kernel}" \
                -initrd {{board_build}}/loader.img
            ;;
        *)
            echo "error: unsupported qemu profile ${qemu}" >&2
            exit 1
            ;;
    esac

# Virtio smoke test (serial + virtio-blk + virtio-net on aarch64 virt)
test-virtio:
    BOARD=qemu_virt_aarch64_virtio just test

# Custom IPC smoke test (echo-server + echo-client on aarch64 virt)
test-echo:
    BOARD=qemu_virt_aarch64_echo just test

# Echo IPC smoke test on x86_64 generic PC
test-x86-echo:
    BOARD=x86_64_generic_echo just test

# RISC-V virt smoke test (NS16550 MMIO UART)
test-riscv:
    BOARD=qemu_virt_riscv64 just test

# RISC-V echo IPC smoke test
test-riscv-echo:
    BOARD=qemu_virt_riscv64_echo just test

# RISC-V virtio smoke test
test-riscv-virtio:
    BOARD=qemu_virt_riscv64_virtio just test

# Timer/RTC/init smoke test (PL031 + SP804 via patched QEMU; see support/qemu/)
test-init:
    BOARD=qemu_virt_aarch64_init just test

# Run all CI smoke tests (requires SDK with aarch64 + x86_64 + riscv64 boards)
test-all:
    #!/usr/bin/env bash
    set -euo pipefail
    just test
    BOARD=x86_64_generic just test
    just test-riscv
    just test-riscv-echo
    just test-virtio
    just test-riscv-virtio
    just test-echo
    just test-x86-echo
    just test-init

# Disk image for virtio-blk QEMU device (MBR boot signature at bytes 510–511)
disk-img:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{root}}/support"
    qemu-img create -f raw "{{root}}/support/disk.img" 4M
    printf '\x55\xAA' | dd of="{{root}}/support/disk.img" bs=1 seek=510 conv=notrunc status=none

clean:
    rm -rf {{build_dir}} target deps/.sdk-path

clean-deps:
    rm -rf deps/workspace deps/.sdk-path