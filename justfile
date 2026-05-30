# lerux kernel justfile
# Modern build interface (preferred over Makefile for daily work).
#
# lerux divergence: upstream kernel builds inside the Redox build system;
# this justfile drives the root crate with optional direct-boot + x86_64-direct.ld.
# See VENDORED.md.

# Where to put build artifacts
build_dir := env_var_or_default("BUILD", "build")

# Extra cargo features (override with KERNEL_CARGO_FEATURES=...)
features := env_var_or_default("KERNEL_CARGO_FEATURES", "")

# Allow overriding objcopy (useful without Redox cross-binutils)
objcopy := env_var_or_default("OBJCOPY", "llvm-objcopy")

export RUST_TARGET_PATH := justfile_directory() + "/targets"

target_spec := justfile_directory() + "/targets/x86_64-unknown-kernel.json"
manifest := justfile_directory() + "/Cargo.toml"

# PVH stub + note for qemu -kernel when using direct-boot (lerux-only linker script)
link_script := if features == "direct-boot" { "linkers/x86_64-direct.ld" } else { "linkers/x86_64.ld" }

# Default recipe
default: build

# Build the kernel (release)
build:
    @mkdir -p "{{build_dir}}"
    @echo "Building kernel (features: {{features}}, linker: {{link_script}})"
    cargo rustc \
        --bin kernel \
        --manifest-path "{{manifest}}" \
        --target "{{target_spec}}" \
        --release \
        -Z build-std=core,alloc -Zbuild-std-features=compiler-builtins-mem \
        -Z json-target-spec \
        --features={{features}} \
        -- \
        -C link-arg=-T -Clink-arg={{link_script}} \
        -C link-arg=-z -Clink-arg=max-page-size=0x1000 \
        --emit link={{build_dir}}/kernel.all

    {{objcopy}} --strip-debug {{build_dir}}/kernel.all {{build_dir}}/kernel
    {{objcopy}} --only-keep-debug {{build_dir}}/kernel.all {{build_dir}}/kernel.sym

toolchain_dir := justfile_directory() + "/.toolchain"
redox_lib := toolchain_dir + "/x86_64-unknown-redox/lib"

# Build the initfs image from the minimal staging directory (Phase A).
# Uses build/bootstrap.elf when present (Phase B), else the staging dummy ELF.
build-initfs:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{build_dir}}"
    bootstrap="userspace/initfs-staging/bootstrap.elf"
    if [ -f "{{build_dir}}/bootstrap.elf" ]; then bootstrap="{{build_dir}}/bootstrap.elf"; fi
    cargo run --release \
        --manifest-path userspace/initfs-tools/Cargo.toml \
        --bin redox-initfs-ar -- \
        userspace/initfs-staging \
        "$bootstrap" \
        -o "{{build_dir}}/initfs.bin"

# Host round-trip test for initfs archiver + reader.
test-initfs:
    cargo test --manifest-path userspace/initfs-tools/Cargo.toml

# Cross-build bootstrap for x86_64-unknown-redox (Phase B).
# Requires: rustup target add x86_64-unknown-redox (nightly-2026-05-24)
# Optional: .toolchain/ from https://static.redox-os.org/toolchain/x86_64-unknown-redox/
#   (relibc-install.tar.gz + gcc-install.tar.gz) for linking; host glibc >= 2.38 for redox-gcc.
build-bootstrap:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{build_dir}}"
    export RUSTUP_TOOLCHAIN=nightly-2026-05-24
    linker="${CARGO_TARGET_X86_64_UNKNOWN_REDOX_LINKER:-}"
    if [ -z "$linker" ] && [ -x "{{toolchain_dir}}/bin/x86_64-unknown-redox-gcc" ]; then
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_LINKER="{{toolchain_dir}}/bin/x86_64-unknown-redox-gcc"
    elif [ -z "$linker" ]; then
    export RUSTFLAGS="${RUSTFLAGS:-} -C linker=rust-lld"
    fi
    cargo build --release \
        --manifest-path userspace/bootstrap/Cargo.toml \
        --target x86_64-unknown-redox
    cp userspace/bootstrap/target/x86_64-unknown-redox/release/bootstrap "{{build_dir}}/bootstrap.elf"

# Build initfs with cross-built bootstrap, then direct-boot kernel with userspace enabled.
build-direct-userspace: build-bootstrap build-initfs
    KERNEL_CARGO_FEATURES="direct-boot,direct-boot-userspace" just build

# Build with the direct-boot feature (for fast QEMU -kernel testing)
build-direct: build-initfs
    KERNEL_CARGO_FEATURES=direct-boot just build

# Boot the kernel directly in QEMU using the direct-boot feature.
# No full Redox bootloader or userspace image is required.
#
# Examples:
#   just qemu-direct
#   just qemu-direct -- -s -S          # GDB stub, stopped at entry
#   just qemu-direct -- -m 1G -smp 4
qemu-direct *QEMU_ARGS:
    just build-direct
    @echo "Launching QEMU (direct-boot mode)..."
    qemu-system-x86_64 \
        -kernel {{build_dir}}/kernel \
        -m 512 \
        -serial mon:stdio \
        -display none \
        -no-reboot \
        {{QEMU_ARGS}}

# Direct-boot serial smoke test (CI): boot headless, assert the kmain idle loop.
smoke: build-direct
    "{{justfile_directory()}}/qemu/smoke-test.sh" --no-build

# Attach GDB to a QEMU instance started with -s or -s -S
#
# Typical workflow:
#   Terminal 1: just qemu-direct -- -s -S
#   Terminal 2: just gdb
gdb *GDB_ARGS:
    @echo "Connecting GDB to localhost:1234..."
    gdb \
        -ex "symbol-file {{build_dir}}/kernel.sym" \
        -ex "target remote localhost:1234" \
        -ex "set pagination off" \
        -ex "set confirm off" \
        {{GDB_ARGS}}

# Quick check (no binary produced)
check:
    @mkdir -p "{{build_dir}}"
    cargo check \
        --bin kernel \
        --manifest-path "{{manifest}}" \
        --target "{{target_spec}}" \
        -Z build-std=core,alloc -Zbuild-std-features=compiler-builtins-mem \
        -Z json-target-spec \
        --features={{features}}

# Verify embedded SMP trampoline bytes match NASM sources (requires nasm).
validate-trampolines:
    "{{justfile_directory()}}/kernel/validation/trampolines/validate-trampolines.sh"
