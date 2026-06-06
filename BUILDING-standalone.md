# Building the lerux kernel (standalone / direct-boot)

This workflow is **lerux-specific**. Upstream Redox kernel development normally goes through the full Redox build system and bootloader handoff; lerux adds a `direct-boot` feature so you can iterate with `qemu -kernel` alone.

See **[VENDORED.md](VENDORED.md)** for how this repo diverges from `redox-os/kernel`.

This repo supports fast kernel-only development using the `direct-boot` feature.

## Prerequisites

- Recent Rust nightly (see `rust-toolchain.toml`)
- `llvm-objcopy` (or any working `objcopy`)
- `qemu-system-x86_64`

You do **not** need a C toolchain (`cc`/`clang`) or `nasm` for direct-boot. The PVH
boot stub is pure Rust (`kernel/src/arch/x86_shared/pvh_boot.rs`, assembled via
`core::arch::global_asm!`).

**Smoke test verified (2026-05-30):** `just build-direct` embeds a real initfs blob;
serial shows non-zero `Bootstrap: START:END`, then `direct-boot mode: skipping
userspace bootstrap…` (default kernel-only path).

## Initfs (Phase A)

```bash
just build-initfs      # build/initfs.bin from userspace/initfs-staging/
just test-initfs       # host archiver round-trip (CI job: initfs)
just build-direct      # builds initfs, embeds in kernel, then links kernel
```

The kernel embeds `build/initfs.bin` at build time (`build.rs` → `direct_boot.rs`).
Staging layout: `userspace/initfs-staging/` (minimal file + dummy bootstrap ELF).

## Userspace bootstrap (Phase B)

Cross-build bootstrap + minimal daemons, then boot with userspace spawn enabled:

```bash
rustup target add x86_64-unknown-redox --toolchain nightly-2026-05-24
rustup toolchain install nightly-2025-11-15 --component rust-src   # relibc build
just build-sysroot          # in-tree relibc → .toolchain/ (+ libgcc from Redox tarball)
just build-direct-userspace   # bootstrap + daemons + initfs + kernel
just qemu-direct-userspace    # serial: bootstrap → init → early daemons
just smoke-userspace          # CI-friendly headless assert (USERSPACE_SMOKE=1)
just check-only-rust          # ELF audit + source allowlist
```

**Linking:** bootstrap uses `rust-lld` + `-Z build-std=…,compiler_builtins` (no host
`x86_64-unknown-redox-gcc` required). Init and daemons static-link via in-tree
`libc.a` + `crt*.o` from `.toolchain/x86_64-unknown-redox/lib`, Redox `libgcc_eh`
from `.toolchain/lib/gcc/…`, and lerux target spec
[`targets/x86_64-unknown-redox.json`](targets/x86_64-unknown-redox.json) (no `-lgcc`
late-link). Initfs staging ships **static ELFs only** (no `libc.so` / `ld64.so.1`).

Default `just build-direct` / `just smoke` keep userspace spawn disabled for fast CI.

## Toolchain / rootfs (Cranelift rustc on lerux)

Cross-build a native Redox `rustc`, populate a virtio rootfs, and compile `hello.rs` inside QEMU:

```bash
just fetch-vendor-sources    # large forks: rust, llvm, rustc_codegen_cranelift
just build-prefix            # download dynamic prefix sysroot (LLVM sanity)
just llvm-sanity             # verify prefix layout (do not run rustc on host)
just build-rustc-redox       # native Redox rustc (LLVM; long)
just build-rustc-redox-cranelift   # same with Cranelift backend (experimental)
just build-rootfs-userspace  # initfs + drivers + rootfs image (prefix only)
just build-rootfs-userspace-rustc  # includes native rustc on rootfs
just qemu-toolchain          # boot with virtio disk, 4G RAM
just qemu-rustc-smoke        # headless assert rustc hello (needs build-rootfs-userspace-rustc)
```

Initfs daemons stay **static**; the rootfs `/usr` toolchain uses **dynamic** linking (upstream model). See [VENDORED.md](VENDORED.md) § Toolchain / rootfs policy.


```bash
# Build with direct-boot support
just build-direct

# Build + run directly in QEMU (no full Redox image needed)
just qemu-direct

# With GDB stub (stopped at entry)
just qemu-direct -- -s -S

# In another terminal
just gdb
```

Extra QEMU flags are passed after `--`:

```bash
just qemu-direct -- -m 1G -smp 4 -s
```

## Manual build

```bash
# Build with the direct-boot feature
RUST_TARGET_PATH=targets \
BUILD=build \
KERNEL_CARGO_FEATURES=direct-boot \
cargo build --release \
    --target targets/x86_64-unknown-kernel.json \
    -Z build-std=core,alloc \
    -Zbuild-std-features=compiler-builtins-mem \
    -Z json-target-spec

# Run directly in QEMU
qemu-system-x86_64 \
    -kernel build/kernel \
    -m 512 \
    -serial mon:stdio \
    -display none \
    -no-reboot
```

## What direct-boot does

When the `direct-boot` feature is enabled, the kernel synthesizes a minimal `KernelArgs` + memory map at boot time (see `kernel/src/startup/direct_boot.rs`). A real initfs image (`build/initfs.bin`) is embedded via `include_bytes!`; `bootstrap_base` / `bootstrap_size` are non-zero. **Upstream Redox has no equivalent feature** — it always expects an external bootloader to supply `KernelArgs`.

By default the kernel enters the idle loop after early bring-up (`direct-boot` skips `userspace_init`). Enable `direct-boot-userspace` (and a cross-built bootstrap ELF in initfs) to spawn bootstrap.

This is intended purely for rapid kernel development and testing.

## GDB Debugging

The quickest path is `qemu/debug.sh`, which builds, launches QEMU paused with a GDB
stub plus exception/reset logging (`qemu-int.log`), and attaches GDB with the
boot-path breakpoints pre-set:

```bash
./qemu/debug.sh             # build + launch QEMU (paused) + attach GDB
./qemu/debug.sh --no-gdb    # only launch QEMU (paused); attach GDB yourself
```

Two-terminal equivalent:

```bash
# Terminal 1
just qemu-direct -- -s -S

# Terminal 2
just gdb
```

Or manually:

```bash
gdb -ex "symbol-file build/kernel.sym" \
    -ex "target remote localhost:1234"
```

Useful boot-path breakpoints (bare `start`/`kmain` do not resolve — use full paths):
`pvh_start32`, `kstart`, `kernel::arch::x86_shared::start::start`,
`kernel::startup::kmain`.

## Notes

- The old `Makefile` is still present for compatibility but the `justfile` is the recommended interface.
- You can still build without the `direct-boot` feature for normal Redox-style builds (requires the full Redox build system / prefix).
