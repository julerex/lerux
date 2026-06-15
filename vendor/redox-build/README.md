# Redox prefix / toolchain build (lerux wrapper)

This directory documents how lerux builds the upstream-style **dynamic** Redox prefix
used for the rootfs toolchain (`/usr/bin/rustc`, `libc.so`, `libLLVM`, …).

## Quick start

```bash
# Fetch large upstream forks (rust, llvm, rustc_codegen_cranelift) — once, gitignored
just fetch-vendor-sources

# Download or build host prefix sysroot (LLVM rustc by default)
just build-prefix

# Populate a redoxfs disk image for QEMU
just mk-rootfs

# Boot with virtio rootfs + userspace
just qemu-rustc-smoke
```

## Environment

| Variable | Default | Purpose |
|----------|---------|---------|
| `LERUX_REDOX_REF` | `../tryredox` | Reference Redox tree (cookbook, redox build system) |
| `PREFIX_BINARY` | `1` | `1` = download official tarballs (fast); `0` = cook from source |
| `RUST_CODEGEN_BACKEND` | `llvm` | `cranelift` for `just build-rustc-redox-cranelift` |

After fetch, `scripts/patch-vendor-rust.sh` gives `src/bootstrap` a standalone `[workspace]` so it does not inherit lerux's root workspace when `x.py` runs cargo.

## Pinned upstream (document in VENDORED.md on sync)

| Component | Branch / source |
|-----------|-----------------|
| rust | `gitlab.redox-os.org/redox-os/rust` @ `redox-2025-10-03` |
| llvm-project | `redox-os/llvm-project` (llvm21 recipe) |
| rustc_codegen_cranelift | matching nightly for redox rust fork |

Recipe copies live under [`../redox-recipes/`](../redox-recipes/).
