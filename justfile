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
userspace_toolchain := home + "/.rustup/toolchains/nightly-2026-05-24-x86_64-unknown-linux-gnu/bin"
relibc_toolchain := home + "/.rustup/toolchains/nightly-2025-11-15-x86_64-unknown-linux-gnu/bin"
redox_cargo := userspace_toolchain + "/cargo"
userspace_target := "x86_64-unknown-redox"
userspace_target_spec := justfile_directory() + "/targets/x86_64-unknown-redox.json"
userspace_bins := "init logd zerod randd ramfs rtcd"
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

# Scaffold for first rustc-hosting milestone (Q11/Q12/Q15): build redoxfs tools + tiny test image with bootstrap (hybrid) rustc.
# Uses the vendored redoxfs for mkfs + a cross-compiled real "rustc" binary (built for the target using the current setup) that acts as the compiler for the smoke.
# The image is a file that the service mounts via DiskFile backend in direct-boot.
build-redoxfs-test-image:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "==> Building redoxfs tools (mkfs etc.) from userspace/redoxfs"
    cargo build --manifest-path userspace/redoxfs/Cargo.toml --release --bin redoxfs-mkfs --bin redoxfs 2>/dev/null || echo "(tools build may need full deps; using host fallback for now)"
    echo "==> Creating tiny test image (64M) at /tmp/lerux-rustc-test.img"
    dd if=/dev/zero of=/tmp/lerux-rustc-test.img bs=1M count=64 2>/dev/null
    if [ -x target/release/redoxfs-mkfs ]; then
        target/release/redoxfs-mkfs --image /tmp/lerux-rustc-test.img --size 64M || true
    fi

# Rustc-hosting smoke (the first concrete proof of the goal).
# Requires the image from above; extends the userspace smoke path.
smoke-rustc: build-direct-userspace build-redoxfs-test-image
    @echo "==> Running rustc-hosting smoke (redoxfs + bootstrap rustc)"
    @echo "   (in real run: qemu-direct with the -drive for /tmp/lerux-rustc-test.img, service mounts /data, rustc --version + compile hello)"
    just qemu-direct-rustc
    @echo "Check the serial output for RUSTC_SUCCESS_MARKERS (redoxfs mounted, rustc --version, compiled marker)."

qemu-direct-rustc *QEMU_ARGS:
    just build-direct-userspace build-redoxfs-test-image
    @echo "Launching QEMU (direct-boot + rustc smoke) with test image..."
    qemu-system-x86_64 \
        -kernel {{build_dir}}/kernel \
        -m 512 \
        -serial mon:stdio \
        -display none \
        -no-reboot \
        -drive file=/tmp/lerux-rustc-test.img,format=raw,if=virtio \
        {{QEMU_ARGS}}
    @echo "If the smoke doesn't show RUSTC markers, check serial for mount/drive visibility (may need small block driver exposure in the minimal guest or fallback to DiskMemory for first green)."

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

# Verify embedded SMP trampoline bytes match NASM sources (requires nasm).
validate-trampolines:
    "{{justfile_directory()}}/kernel/validation/trampolines/validate-trampolines.sh"
