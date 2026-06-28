# lerux — Rust userspace on seL4 Microkit
# Kernel built from upstream repos in deps/workspace/ (not vendored).

build_dir := env_var_or_default("BUILD", "build")
board := env_var_or_default("BOARD", "qemu_virt_aarch64")
config := env_var_or_default("CONFIG", "debug")
board_build := build_dir + "/" + board

root := justfile_directory()
workspace := root + "/deps/workspace"
sdk_path_file := root + "/deps/.sdk-path"
target_spec := root + "/support/targets/aarch64-sel4-microkit.json"

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

# Build all protection-domain ELFs for the selected board
build: (build-pd "hello")

build-pd crate:
    #!/usr/bin/env bash
    set -euo pipefail
    sdk="$(just sdk-path)"
    source "{{root}}/scripts/libclang-env.sh"
    mkdir -p "{{board_build}}"
    SEL4_INCLUDE_DIRS="${sdk}/board/{{board}}/{{config}}/include" \
        cargo build --release -p {{crate}} \
            --target-dir "{{board_build}}/target" \
            --target "{{target_spec}}" \
            -Z json-target-spec \
            -Z build-std=core,alloc,compiler_builtins \
            -Z build-std-features=compiler-builtins-mem
    cp "{{board_build}}/target/aarch64-sel4-microkit/release/{{crate}}.elf" "{{board_build}}/"

# Assemble loader.img via the Microkit tool
image: build
    #!/usr/bin/env bash
    set -euo pipefail
    sdk="$(just sdk-path)"
    "${sdk}/bin/microkit" "{{root}}/userspace/systems/hello.system" \
        --search-path "{{board_build}}" \
        --board "{{board}}" \
        --config "{{config}}" \
        -r "{{board_build}}/report.txt" \
        -o "{{board_build}}/loader.img"

# Boot in QEMU (aarch64 virt)
run: image
    just qemu-aarch64

qemu-aarch64:
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="$(bash scripts/host-path.sh)"
    exec qemu-system-aarch64 \
        -machine virt,virtualization=on -cpu cortex-a53 -m size=2G \
        -serial mon:stdio -nographic \
        -device loader,file={{board_build}}/loader.img,addr=0x70000000,cpu-num=0

# x86_64 bring-up (after SDK build; requires x86_64-sel4-microkit target spec)
qemu-x86_64:
    @echo "Set BOARD=qemu_x86_64 and add support/targets/x86_64-sel4-microkit.json"
    @echo "Then: qemu-system-x86_64 -m 2G -serial mon:stdio -nographic -kernel {{board_build}}/loader.img"

# Serial smoke test
test: image
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="$(bash scripts/host-path.sh)"
    exec python3 scripts/test.py qemu-system-aarch64 \
        -machine virt,virtualization=on -cpu cortex-a53 -m size=2G \
        -serial mon:stdio -nographic \
        -device loader,file={{board_build}}/loader.img,addr=0x70000000,cpu-num=0

clean:
    rm -rf {{build_dir}} target deps/.sdk-path

clean-deps:
    rm -rf deps/workspace deps/.sdk-path