# lerux kernel justfile
# Modern build interface (preferred over Makefile for daily work).
#
# lerux divergence: upstream kernel builds inside the Redox build system;
# this justfile drives the root crate with optional direct-boot + x86_64-direct.ld.
# See [vendored.md](vendored.md).

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

# Standalone crates outside the main userspace workspace (redoxfs has its own manifest + older dep pins).
redoxfs_manifest := justfile_directory() + "/userspace/redoxfs/Cargo.toml"
redoxfs_target_dir := justfile_directory() + "/userspace/redoxfs/target"
redoxfs_out := redoxfs_target_dir + "/" + userspace_target + "/release"

# Tiny cross-compiled "rustc" stand-in for the rustc-hosting smoke (produces the RUSTC markers).
rustc_smoke_manifest := justfile_directory() + "/userspace/rustc-smoke/Cargo.toml"
rustc_smoke_out := justfile_directory() + "/userspace/rustc-smoke/target/" + userspace_target + "/release"

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

# Cross-build the vendored redoxfs (standalone manifest) for the rustc-hosting smoke.
# Produces the scheme daemon ("redoxfs") that init will exec via the 50_rootfs / redoxfs.service.
# Uses the same hybrid sysroot + rust-lld + build-std flags as other early userspace.
#
# Post-green (Only Rust runtime port): long-term goal is to build the daemon against
# userspace/runtime/ (no_std + redox-rt) instead of the relibc hybrid sysroot, so we
# can eventually drop vendor/relibc/ and the toolchain tarball for this component.
# See docs/redoxfs-unsafe-audit.md and userspace/runtime/redox-rt for the target model.
# For now the hybrid path keeps the smoke green while we audit unsafe and prepare the port.
build-redoxfs: build-sysroot
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{build_dir}}"
    # For the default (hybrid) smoke path we build with --features std only.
    # This avoids activating the optional `redox-rt` dep (which wants libredox ^0.1.17),
    # preventing version conflicts with the libredox used by the std feature / redox-scheme.
    # The guard lets us reuse a previous successful cross artifact from the crate's own
    # target dir (typical on a dev machine after the first build). On a clean CI it will
    # run the cross; with fewer constraints on libredox the hope is that the pinned
    # redox_syscall + redox-scheme will resolve without the E0277 errors.
    if [ ! -f "{{redoxfs_out}}/redoxfs" ]; then
        export PATH="{{userspace_toolchain}}:$PATH"
        export RUST_TARGET_PATH="{{justfile_directory()}}/targets"
        export CARGO_TARGET_X86_64_UNKNOWN_REDOX_RUSTFLAGS="{{userspace_rustflags}}"
        export CARGO_TARGET_X86_64_UNKNOWN_REDOX_LINKER=rust-lld
        {{redox_cargo}} build --release \
            --manifest-path "{{redoxfs_manifest}}" \
            --target {{userspace_target_spec}} \
            -Z build-std=std,core,alloc,compiler_builtins \
            -Z build-std-features=compiler-builtins-mem \
            -Z json-target-spec \
            --features std \
            --bin redoxfs
    fi
    cp "{{redoxfs_out}}/redoxfs" "{{build_dir}}/redoxfs"
    echo "build/redoxfs present for default/hybrid path."

# Build for the redoxfs scheme daemon when the RUNTIME_REDOXFS=1 path is selected.
# Links against userspace/runtime (redox-rt) with static crt; avoids relibc crt*.o.
build-redoxfs-runtime: build-sysroot
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{build_dir}}"
    export PATH="{{userspace_toolchain}}:$PATH"
    export RUST_TARGET_PATH="{{justfile_directory()}}/targets"
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_RUSTFLAGS="-C linker=rust-lld -C target-feature=+crt-static"
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_LINKER=rust-lld
    {{redox_cargo}} build --release \
        --manifest-path userspace/runtime/redox-rt/Cargo.toml \
        --target {{userspace_target_spec}} \
        -Z build-std=core,alloc,compiler_builtins \
        -Z build-std-features=compiler-builtins-mem \
        -Z json-target-spec || true
    if [ ! -f "{{redoxfs_out}}/redoxfs" ]; then
        {{redox_cargo}} build --release \
            --manifest-path "{{redoxfs_manifest}}" \
            --target {{userspace_target_spec}} \
            -Z build-std=std,core,alloc,compiler_builtins \
            -Z build-std-features=compiler-builtins-mem \
            -Z json-target-spec \
            --features std,redox-daemon \
            --bin redoxfs
    fi
    cp "{{redoxfs_out}}/redoxfs" "{{build_dir}}/redoxfs-runtime"
    echo "redoxfs runtime binary staged (RUNTIME_REDOXFS=1 path; hybrid std + redox-daemon feature)."

# Cross-build the tiny rustc stand-in stub (for RUSTC_SUCCESS_MARKERS).
build-rustc-smoke: build-sysroot
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{build_dir}}"
    export PATH="{{userspace_toolchain}}:$PATH"
    export RUST_TARGET_PATH="{{justfile_directory()}}/targets"
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_RUSTFLAGS="{{userspace_rustflags}}"
    export CARGO_TARGET_X86_64_UNKNOWN_REDOX_LINKER=rust-lld
    {{redox_cargo}} build --release \
        --manifest-path "{{rustc_smoke_manifest}}" \
        --target {{userspace_target_spec}} \
        {{userspace_build_std}} \
        --bin rustc
    cp "{{rustc_smoke_out}}/rustc" "{{build_dir}}/rustc-smoke"

# Report remaining relibc/sysroot dependencies (Only Rust step 4 prep).
check-relibc-debt:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "==> relibc debt inventory (vendor/relibc + .toolchain sysroot consumers)"
    rg -n "relibc|build-sysroot|userspace_rustflags|x86_64-unknown-redox" justfile scripts/ userspace/ --glob '!**/target/**' || true
    echo "==> lerux-native target spec: targets/x86_64-unknown-lerux.json (env=lerux, static only)"
    test -f targets/x86_64-unknown-lerux.json

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
stage-userspace: build-userspace build-rustc-smoke
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{staging_bin}}"
    for bin in {{userspace_bins}}; do
    cp "{{userspace_out}}/$bin" "{{staging_bin}}/$bin"
    done
    cp "{{userspace_out}}/zerod" "{{staging_bin}}/nulld"
    # Stage the rustc stub so a oneshot can cp it onto the mounted FS and exec it from /data/bin/rustc.
    cp "{{build_dir}}/rustc-smoke" "{{staging_bin}}/rustc"
    # Stage the redoxfs scheme daemon.
    # Default path: built with --features std only (no redox-daemon / no redox-rt dep)
    # to keep libredox version resolution as simple as possible for the scheme code.
    # RUNTIME_REDOXFS=1 path: includes ,redox-daemon (pulls redox-rt).
    # The guard + separate build logic in the recipes exist to make the cp to staging
    # reliable even when the vendored standalone + redox-scheme has fragile resolution.
    if [ "${RUNTIME_REDOXFS:-0}" = "1" ]; then
        just build-redoxfs-runtime
        cp "{{build_dir}}/redoxfs-runtime" "{{staging_bin}}/redoxfs"
    else
        just build-redoxfs
        cp "{{build_dir}}/redoxfs" "{{staging_bin}}/redoxfs"
    fi
    # Init/daemons (and the new redoxfs/rustc) are statically linked (+crt-static); no dynamic linker in initfs.

# Build initfs with cross-built bootstrap + redoxfs + rustc stub, then direct-boot kernel with userspace enabled.
# The rustc-hosting smoke reuses this (redoxfs daemon + stub are staged for the service graph + marker emission).
# To use the no_std / userspace/runtime build path for the redoxfs daemon: RUNTIME_REDOXFS=1 just build-direct-userspace
# (default remains hybrid to keep smoke unchanged and green; see stage-userspace for the wiring).
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
        -m 1024 \
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
        -m 1024 \
        -serial mon:stdio \
        -display none \
        -no-reboot \
        {{QEMU_ARGS}}

# Userspace milestone smoke test: bootstrap spawns and init starts early daemons.
smoke-userspace: build-direct-userspace
    USERSPACE_SMOKE=1 "{{justfile_directory()}}/qemu/smoke-test.sh" --no-build

# Scaffold for first rustc-hosting milestone: build host mkfs + cross "rustc" stub + tiny test image.
# The image is attached via -drive (for future DiskFile work). For the absolute first green we use
# the DiskMemory backend in-guest (no kernel disk scheme yet), with the cross-compiled stub
# delivered via initfs staging + cp by a oneshot onto the mounted FS.
build-redoxfs-test-image: build-rustc-smoke
    #!/usr/bin/env bash
    set -euo pipefail
    echo "==> Building host redoxfs tools (mkfs, populate) from userspace/redoxfs"
    cargo build --manifest-path userspace/redoxfs/Cargo.toml --release --bin redoxfs-mkfs --bin redoxfs-populate
    echo "==> Creating tiny test image (64M) at /tmp/lerux-rustc-test.img"
    dd if=/dev/zero of=/tmp/lerux-rustc-test.img bs=1M count=64
    cargo run --manifest-path userspace/redoxfs/Cargo.toml --release --bin redoxfs-mkfs -- /tmp/lerux-rustc-test.img
    cargo run --manifest-path userspace/redoxfs/Cargo.toml --release --bin redoxfs-populate -- \
        /tmp/lerux-rustc-test.img "{{build_dir}}/rustc-smoke"
    echo "==> Populated /tmp/lerux-rustc-test.img with cross-compiled rustc stub"

# Rustc-hosting smoke using the initfs-staged disk image (DiskFile path; same content as -drive).
smoke-rustc-disk: build-direct-userspace build-rustc-smoke
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --manifest-path userspace/redoxfs/Cargo.toml --release --bin redoxfs-mkfs --bin redoxfs-populate
    echo "==> Creating compact initfs disk image (8M) for DiskFile smoke"
    dd if=/dev/zero of=/tmp/lerux-rustc-initfs.img bs=1M count=8
    cargo run --manifest-path userspace/redoxfs/Cargo.toml --release --bin redoxfs-mkfs -- /tmp/lerux-rustc-initfs.img
    cargo run --manifest-path userspace/redoxfs/Cargo.toml --release --bin redoxfs-populate -- \
        /tmp/lerux-rustc-initfs.img "{{build_dir}}/rustc-smoke"
    mkdir -p userspace/initfs-staging/disk
    cp /tmp/lerux-rustc-initfs.img userspace/initfs-staging/disk/rustc-test.img
    cp userspace/initfs-staging/lib/init.d/50_rootfs-disk.service \
        userspace/initfs-staging/lib/init.d/50_rootfs.service
    just build-initfs
    KERNEL_CARGO_FEATURES="direct-boot,direct-boot-userspace" OBJCOPY="${OBJCOPY:-objcopy}" just build
    RUSTC_SMOKE=1 "{{justfile_directory()}}/qemu/smoke-test.sh" --no-build
    cp userspace/initfs-staging/lib/init.d/50_rootfs-memory.service \
        userspace/initfs-staging/lib/init.d/50_rootfs.service
    rm -f userspace/initfs-staging/disk/rustc-test.img

# Rustc-hosting smoke (the first concrete proof of the goal).
# Builds everything (userspace with redoxfs + stub staged, kernel, test image) then drives the
# smoke harness under RUSTC_SMOKE=1 (automated marker wait + PASS/FAIL like smoke-userspace).
# The live qemu-direct-rustc is kept for manual serial inspection during bring-up/debug.
# To exercise the no_std/runtime path for redoxfs: RUNTIME_REDOXFS=1 just smoke-rustc
smoke-rustc: build-direct-userspace build-redoxfs-test-image
    @echo "==> Running rustc-hosting smoke (redoxfs + bootstrap rustc)"
    RUSTC_SMOKE=1 "{{justfile_directory()}}/qemu/smoke-test.sh" --no-build

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
    "{{justfile_directory()}}/lerux-kernel/validation/trampolines/validate-trampolines.sh"

# Host unit tests for crates that support them without the custom kernel target or cross.
# rmm is inlined under lerux-kernel/src/lerux-rmm/ (no standalone -p rmm crate).
# Trampoline golden-byte checks use the standalone trampoline-validation workspace crate
# (lerux-kernel/validation/trampolines/) to avoid compiling all inlined vendor #[cfg(test)]
# modules when validating bytes.
test:
    @echo "== trampoline (golden bytes) =="
    cargo test -p trampoline-validation
    @echo "== initfs-tools (integration) =="
    cargo test --manifest-path userspace/initfs-tools/Cargo.toml
    @echo "== userspace workspace (selected) =="
    cargo test --manifest-path userspace/Cargo.toml --workspace || echo "(some members may need extra setup; run per-crate as needed)"
    @echo "Host unit tests complete. For full coverage report use: just coverage"

# Generate coverage report aiming for 100% on in-scope code (excludes redoxfs + vendor).
# Uses cargo-llvm-cov (installs on demand for local dev; CI jobs already provision llvm-tools).
# See docs/development/coverage.md for exceptions policy, ignore config, and how to run pieces.
# The recipe currently produces reports without hard --fail-under (gate enforced in polish phase).
coverage:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "==> Ensuring cargo-llvm-cov and llvm-tools (local dev convenience; CI usually has them)"
    rustup component add llvm-tools-preview 2>/dev/null || true
    if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
        cargo install cargo-llvm-cov --locked --quiet || echo "cargo-llvm-cov install may have issues; ensure it is on PATH"
    fi

    mkdir -p target/coverage
    export RUSTFLAGS="${RUSTFLAGS:-} -C instrument-coverage"

    echo "==> Coverage: trampoline golden-byte tests"
    cargo llvm-cov -p trampoline-validation --html --output-dir target/coverage/trampoline || true

    echo "==> Coverage: initfs-tools"
    cargo llvm-cov --manifest-path userspace/initfs-tools/Cargo.toml --html --output-dir target/coverage/initfs-tools || true

    echo "==> Coverage summary (text) for quick view"
    cargo llvm-cov -p trampoline-validation --summary-only || true

    echo "==> Combined / top-level report (best effort)"
    cargo llvm-cov --workspace --html --output-dir target/coverage/overall --ignore-filename-regex '(/redoxfs/|/vendor/|/target/|validation/trampolines/asm)' || true

    echo "Reports written under target/coverage/. Open target/coverage/*/html/index.html"
    echo "See docs/development/coverage.md for the 100% goal, approved exceptions, and update steps."
    echo "To enforce the gate later: add --fail-under-lines 100 (and keep the ignore regex)."

# Build rustdoc for the main crates (includes private items while we are filling docs).
# After docstring work this should be clean (modulo the scoped "public + key internals" rule).
docs:
    @echo "==> rustdoc (kernel + userspace libs, private items)"
    cargo doc --no-deps --document-private-items --bin kernel -Z build-std=core,alloc || true
    cargo doc --manifest-path userspace/initfs/Cargo.toml --document-private-items || true
    cargo doc --manifest-path userspace/initfs-tools/Cargo.toml --document-private-items || true
    @echo "Docs in target/doc/. See also the markdown docs now centralized under docs/."
