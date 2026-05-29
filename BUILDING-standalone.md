# Building the lerux kernel (standalone / direct-boot)

This repo supports fast kernel-only development using the `direct-boot` feature.

## Prerequisites

- Recent Rust nightly (see `rust-toolchain.toml`)
- `nasm`
- `llvm-objcopy` (or any working `objcopy`)
- QEMU

## Recommended: Use the justfile

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

When the `direct-boot` feature is enabled, the kernel synthesizes a minimal `KernelArgs` + memory map at boot time. This lets you test the kernel with plain `qemu -kernel` without the Redox bootloader or a full initfs.

The kernel will perform early bring-up (serial, memory, paging, allocator, etc.) and then enter the idle loop (userspace bootstrap is intentionally skipped in this mode).

This is intended purely for rapid kernel development and testing.

## GDB Debugging

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

## Notes

- The old `Makefile` is still present for compatibility but the `justfile` is the recommended interface.
- You can still build without the `direct-boot` feature for normal Redox-style builds (requires the full Redox build system / prefix).
