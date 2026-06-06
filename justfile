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
link_script := if features == "direct-boot" { "linkers/x86_64-direct.ld" } else if features == "direct-boot,direct-boot-userspace" { "linkers/x86_64-direct.ld" } else if features == "direct-boot,direct-boot-userspace,direct-boot-rootfs" { "linkers/x86_64-direct.ld" } else { "linkers/x86_64.ld" }

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
userspace_toolchain := home + "/.rustup/toolchains/nightly-2026-05-24-x86_64-unknown-linux-gnu/bin"
relibc_toolchain := home + "/.rustup/toolchains/nightly-2025-11-15-x86_64-unknown-linux-gnu/bin"
redox_cargo := userspace_toolchain + "/cargo"
userspace_target := "x86_64-unknown-redox"
userspace_target_spec := justfile_directory() + "/targets/x86_64-unknown-redox.json"
userspace_bins := "init logd zerod randd ramfs rtcd ptyd pcid-spawner"
userspace_drivers := "virtio-blkd"
rootfs_img := build_dir + "/rootfs.img"
prefix_sysroot := build_dir + "/prefix/x86_64-unknown-redox/sysroot"
kernel_features_rootfs := "direct-boot,direct-boot-userspace,direct-boot-rootfs"
# Static link: in-tree relibc sysroot + Redox libgcc_eh (rustc liblibc) + build-std panic_abort.
userspace_rustflags := "-C target-feature=+crt-static -C link-arg=" + redox_lib + "/crt1.o -C link-arg=" + redox_lib + "/crti.o -C link-arg=-L" + redox_lib + " -C link-arg=-L" + redox_gcc_lib + " -C link-arg=-lgcc_eh -C link-arg=-lc -C link-arg=" + redox_lib + "/crtn.o -C link-arg=--allow-multiple-definition"
userspace_build_std := "-Z build-std=std,panic_abort,core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem -Z json-target-spec"
bootstrap_rustflags := "-C linker=rust-lld"
userspace_out := justfile_directory() + "/userspace/target/" + userspace_target + "/release"
staging_bin := justfile_directory() + "/userspace/initfs-staging/bin"
toolchain_url := "https://static.redox-os.org/toolchain/x86_64-unknown-redox/relibc-install.tar.gz"

# Build relibc sysroot from vendor/relibc (libc.a, crt*.o, ld64) + libgcc from Redox toolchain.
build-sysroot:
    "{{justfile_directory()}}/scripts/build-sysroot.sh"

# Deprecated: full tarball install. Prefer `just build-sysroot`.
install-toolchain:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "install-toolchain is deprecated; use: just build-sysroot" >&2
    just build-sysroot

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
    export PATH="{{userspace_toolchain}}:$PATH"
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_LINKER=rust-lld
    export RUSTFLAGS="{{bootstrap_rustflags}}"
    {{redox_cargo}} build --release \
        --manifest-path userspace/bootstrap/Cargo.toml \
        --target {{userspace_target}} \
        -Z build-std=core,alloc,compiler_builtins \
        -Z build-std-features=compiler-builtins-mem
    cp userspace/bootstrap/target/{{userspace_target}}/release/bootstrap "{{build_dir}}/bootstrap.elf"

# Cross-build init + minimal early daemons (Phase B).
build-userspace: build-sysroot
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="{{userspace_toolchain}}:$PATH"
    export RUST_TARGET_PATH="{{justfile_directory()}}/targets"
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_RUSTFLAGS="{{userspace_rustflags}}"
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_LINKER=rust-lld
    {{redox_cargo}} build --release \
        --manifest-path userspace/Cargo.toml \
        --target {{userspace_target_spec}} \
        {{userspace_build_std}} \
        -p init -p logd -p zerod -p randd -p ramfs -p rtcd -p ptyd -p pcid-spawner

# Copy cross-built userspace binaries into initfs staging.
stage-userspace: build-userspace
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{staging_bin}}"
    for bin in {{userspace_bins}}; do
    cp "{{userspace_out}}/$bin" "{{staging_bin}}/$bin"
    done
    cp "{{userspace_out}}/zerod" "{{staging_bin}}/nulld"
    # Init/daemons are statically linked (+crt-static); no dynamic linker in initfs.

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

# Verify Only Rust policy: ELF audit, source allowlist, optional smoke.
check-only-rust *ARGS:
    "{{justfile_directory()}}/scripts/check-only-rust.sh" {{ARGS}}

# --- Toolchain / rootfs (Cranelift rustc on lerux) ---

fetch-vendor-sources:
    "{{justfile_directory()}}/scripts/fetch-vendor-sources.sh"

build-prefix:
    "{{justfile_directory()}}/scripts/build-prefix.sh"

build-rustc-redox:
    RUST_CODEGEN_BACKEND=llvm "{{justfile_directory()}}/scripts/build-rustc-redox.sh"

build-rustc-redox-cranelift:
    RUST_CODEGEN_BACKEND=cranelift "{{justfile_directory()}}/scripts/build-rustc-redox.sh"

mk-rootfs: build-prefix
    "{{justfile_directory()}}/scripts/mk-rootfs.sh"

# Full rootfs with native Redox rustc (long build; required for rustc-smoke).
mk-rootfs-with-rustc: build-prefix build-rustc-redox
    "{{justfile_directory()}}/scripts/mk-rootfs.sh"

# Cross-build redoxfs mount tool for initfs (required before 50_rootfs.service).
build-redoxfs: build-sysroot
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="{{userspace_toolchain}}:$PATH"
    export RUST_TARGET_PATH="{{justfile_directory()}}/targets"
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_RUSTFLAGS="{{userspace_rustflags}}"
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_LINKER=rust-lld
    {{redox_cargo}} build --release \
        --manifest-path vendor/redoxfs/Cargo.toml \
        --bin redoxfs \
        --target {{userspace_target_spec}} \
        {{userspace_build_std}} \
        --features std
    {{redox_cargo}} build --release \
        --manifest-path vendor/redoxfs/Cargo.toml \
        --bin redoxfs-mkfs \
        --features std
    {{redox_cargo}} build --release \
        --manifest-path vendor/redoxfs/Cargo.toml \
        --bin redoxfs-ar \
        --features std

# Build initfs driver ELFs (virtio-blkd, etc.) into staging/lib/drivers/.
build-drivers: build-sysroot
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="{{userspace_toolchain}}:$PATH"
    export RUST_TARGET_PATH="{{justfile_directory()}}/targets"
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_RUSTFLAGS="{{userspace_rustflags}}"
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_LINKER=rust-lld
    {{redox_cargo}} build --release \
        --manifest-path userspace/Cargo.toml \
        --target {{userspace_target_spec}} \
        {{userspace_build_std}} \
        -p virtio-blkd
    mkdir -p "{{justfile_directory()}}/userspace/initfs-staging/lib/drivers"
    for bin in {{userspace_drivers}}; do
        cp "{{userspace_out}}/$bin" "{{justfile_directory()}}/userspace/initfs-staging/lib/drivers/$bin"
    done

stage-redoxfs: build-redoxfs
    cp vendor/redoxfs/target/{{userspace_target}}/release/redoxfs "{{staging_bin}}/redoxfs"

# Build userspace with rootfs drivers + redoxfs + rootfs image (prefix toolchain; no native rustc cook).
build-rootfs-userspace: build-bootstrap build-userspace build-drivers stage-redoxfs stage-userspace build-initfs mk-rootfs
    KERNEL_CARGO_FEATURES="{{kernel_features_rootfs}}" just build

# Same as build-rootfs-userspace but also cross-builds native Redox rustc (very long).
build-rootfs-userspace-rustc: build-bootstrap build-userspace build-drivers stage-redoxfs stage-userspace build-initfs mk-rootfs-with-rustc
    KERNEL_CARGO_FEATURES="{{kernel_features_rootfs}}" just build

build-direct-rootfs: build-initfs
    KERNEL_CARGO_FEATURES="{{kernel_features_rootfs}}" just build

# Boot with virtio rootfs disk (4G RAM). Requires build/rootfs.img.
qemu-toolchain *QEMU_ARGS:
    just build-rootfs-userspace
    @echo "Launching QEMU (rootfs + toolchain)..."
    qemu-system-x86_64 \
        -kernel {{build_dir}}/kernel \
        -m 4096 \
        -smp 2 \
        -serial mon:stdio \
        -display none \
        -no-reboot \
        -drive file={{rootfs_img}},if=virtio,format=raw \
        {{QEMU_ARGS}}

qemu-rustc-smoke:
    RUSTC_SMOKE=1 "{{justfile_directory()}}/qemu/rustc-smoke-test.sh"

# LLVM prefix-only sanity (download tarballs, verify layout — do not execute Redox rustc on host).
llvm-sanity: build-prefix
    @echo "Prefix sysroot at {{prefix_sysroot}}"
    @test -f "{{prefix_sysroot}}/bin/rustc" || (echo "missing {{prefix_sysroot}}/bin/rustc" >&2; exit 1)
    @test -d "{{prefix_sysroot}}/lib/rustlib/x86_64-unknown-redox" || (echo "missing rustlib for x86_64-unknown-redox" >&2; exit 1)
    @file "{{prefix_sysroot}}/bin/rustc"
    @echo "llvm-sanity: prefix layout OK (run rustc inside lerux QEMU, not on host)"

# Verify embedded SMP trampoline bytes match NASM sources (requires nasm).
validate-trampolines:
    "{{justfile_directory()}}/kernel/validation/trampolines/validate-trampolines.sh"
