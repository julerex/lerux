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
link_script := if features == "direct-boot" { "linkers/x86_64-direct.ld" } else if features == "direct-boot,direct-boot-userspace" { "linkers/x86_64-direct.ld" } else { "linkers/x86_64.ld" }

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
redox_gcc_lib := toolchain_dir + "/lib/gcc/x86_64-unknown-redox/13.2.0"
home := env_var("HOME")
redox_cargo := home + "/.cargo/bin/cargo"
redox_rustup := home + "/.cargo/bin/rustup"
userspace_target := "x86_64-unknown-redox"
userspace_bins := "init logd zerod randd ramfs rtcd"
# Static link via rust-lld + relibc from .toolchain (no host redox-gcc; glibc 2.38+ not required).
userspace_rustflags := "-C target-feature=+crt-static -C link-arg=" + redox_lib + "/crt1.o -C link-arg=" + redox_lib + "/crti.o -C link-arg=-L" + redox_lib + " -C link-arg=-L" + redox_gcc_lib + " -C link-arg=-lgcc_eh -C link-arg=-lc -C link-arg=" + redox_lib + "/crtn.o -C link-arg=--allow-multiple-definition"
bootstrap_rustflags := "-C linker=rust-lld"
userspace_out := justfile_directory() + "/userspace/target/" + userspace_target + "/release"
staging_bin := justfile_directory() + "/userspace/initfs-staging/bin"
toolchain_url := "https://static.redox-os.org/toolchain/x86_64-unknown-redox/relibc-install.tar.gz"

# One-time: extract Redox relibc sysroot into .toolchain/ (for static-linking init/daemons).
install-toolchain:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -f "{{redox_lib}}/libc.a" ]; then
    echo ".toolchain/ already has relibc; skipping"
    exit 0
    fi
    mkdir -p "{{toolchain_dir}}"
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    echo "Downloading relibc toolchain (large tarball)..."
    curl -fsSL "{{toolchain_url}}" | tar -xzf - -C "$tmp"
    cp -a "$tmp"/* "{{toolchain_dir}}/"
    echo "Installed relibc to {{toolchain_dir}}"

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
# Links with rust-lld + build-std compiler-builtins-mem (works without redox-gcc on host).
build-bootstrap:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{build_dir}}"
    export RUSTUP_TOOLCHAIN=nightly-2026-05-24
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_LINKER=rust-lld
    export RUSTFLAGS="{{bootstrap_rustflags}}"
    {{redox_cargo}} build --release \
        --manifest-path userspace/bootstrap/Cargo.toml \
        --target {{userspace_target}} \
        -Z build-std=core,alloc,compiler_builtins \
        -Z build-std-features=compiler-builtins-mem
    cp userspace/bootstrap/target/{{userspace_target}}/release/bootstrap "{{build_dir}}/bootstrap.elf"

# Cross-build init + minimal early daemons (Phase B).
build-userspace:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -f "{{redox_lib}}/libc.a" ]; then
    echo "Missing {{redox_lib}}/libc.a — run: just install-toolchain" >&2
    exit 1
    fi
    export RUSTUP_TOOLCHAIN=nightly-2026-05-24
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_LINKER=rust-lld
    export RUSTFLAGS="{{userspace_rustflags}}"
    {{redox_cargo}} build --release \
        --manifest-path userspace/Cargo.toml \
        --target {{userspace_target}} \
        -p init -p logd -p zerod -p randd -p ramfs -p rtcd

# Copy cross-built userspace binaries into initfs staging.
stage-userspace: build-userspace
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{staging_bin}}"
    for bin in {{userspace_bins}}; do
    cp "{{userspace_out}}/$bin" "{{staging_bin}}/$bin"
    done
    cp "{{userspace_out}}/zerod" "{{staging_bin}}/nulld"
    staging_lib="{{justfile_directory()}}/userspace/initfs-staging/lib"
    mkdir -p "$staging_lib"
    cp "{{redox_lib}}/libc.so" "{{redox_lib}}/ld64.so.1" "$staging_lib/"

# Build initfs with cross-built bootstrap, then direct-boot kernel with userspace enabled.
build-direct-userspace: build-bootstrap stage-userspace build-initfs
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

# Boot direct-boot kernel with userspace spawn enabled (Phase B).
qemu-direct-userspace *QEMU_ARGS:
    just build-direct-userspace
    @echo "Launching QEMU (direct-boot + userspace)..."
    qemu-system-x86_64 \
        -kernel {{build_dir}}/kernel \
        -m 512 \
        -serial mon:stdio \
        -display none \
        -no-reboot \
        {{QEMU_ARGS}}

# Userspace milestone smoke test: bootstrap spawns and init starts early daemons.
smoke-userspace: build-direct-userspace
    USERSPACE_SMOKE=1 "{{justfile_directory()}}/qemu/smoke-test.sh" --no-build

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
