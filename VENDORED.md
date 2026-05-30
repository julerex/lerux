# lerux vs upstream Redox kernel

This document records how **lerux** diverges from the upstream [Redox microkernel](https://gitlab.redox-os.org/redox-os/kernel) (`redox-os/kernel`). It is the canonical reference for vendoring, build layout, and intentional behavioral differences.

**Upstream snapshot:** vendored copy under `kernel/` from `redox-os/kernel`, approximately **2026-05**. There is no pinned commit hash yet; treat this document plus git history as the patch log until one is recorded.

**Project goal:** keep Redox kernel design, syscalls, schemes, and multi-arch support while pursuing an **"Only Rust"** build (no `nasm`, no `cc`/`clang` in the kernel build) and a **standalone QEMU direct-boot** path that does not require the full Redox build system.

---

## What stays the same as upstream

Most of `kernel/` is unmodified Redox code. In particular:

| Area | Status |
|------|--------|
| Syscall surface (`redox_syscall`, scheme handlers) | Unchanged |
| Memory manager (`kernel/rmm/`) | Unchanged |
| Context switching, signals, most device drivers | Unchanged |
| `config.toml` / kconfig CPU-feature pattern | Same format as upstream |
| Boot strings and `sys:uname` | Still report **"Redox"** (intentional compatibility; not renamed to "lerux") |
| Dependencies on Redox crates (`redox-path`, `redox_syscall`) | Kept for drop-in compatibility |

Building **without** the `direct-boot` feature is still intended to produce a normal Redox-style kernel image, but that path expects the full Redox build system / prefix (see [BUILDING-standalone.md](BUILDING-standalone.md)).

---

## Repository layout (lerux-only)

Upstream ships the kernel as the crate root. lerux wraps it:

```
lerux/                          # standalone dev repo (this project)
├── Cargo.toml, build.rs        # crate manifest at repo root, not inside kernel/
├── linkers/, targets/          # linker scripts + custom kernel JSON targets
├── justfile, Makefile          # build interfaces (justfile preferred)
├── qemu/                       # lerux QEMU harness (not in upstream kernel repo)
├── BUILDING-standalone.md      # direct-boot workflow
└── kernel/                     # vendored redox-os/kernel tree
    └── src/, rmm/, validation/
```

---

## Code changes from upstream

### 1. SMP AP trampolines — no `nasm`

| Upstream | lerux |
|----------|-------|
| `src/asm/{x86,x86_64}/trampoline.asm` assembled by `nasm` in `build.rs` | Bytes embedded as `&[u8]` in `kernel/src/arch/x86_shared/trampoline.rs` |

Validation/regeneration: `kernel/validation/trampolines/` — `just validate-trampolines` (nasm, CI) and `cargo test --bin kernel trampoline` (golden files).

### 2. PVH boot stub — no C toolchain

| Upstream | lerux |
|----------|-------|
| `pvh_boot.S` compiled via `cc`/`clang` in `build.rs` | `kernel/src/arch/x86_shared/pvh_boot.rs` via `core::arch::global_asm!` |
| Used in upstream direct-boot / PVH paths | Gated on `feature = "direct-boot"` only |

### 3. `direct-boot` feature and synthetic boot args

**New in lerux** (not an upstream Cargo feature):

| File | Role |
|------|------|
| `kernel/src/startup/direct_boot.rs` | Synthesizes `KernelArgs` + memory map for `qemu -kernel` without Redox bootloader or initfs |
| `kernel/src/arch/x86_shared/start.rs` | Uses `get_direct_boot_args()` instead of bootloader-supplied pointer; skips graphical debug and ACPI when `direct-boot` is enabled |
| `kernel/src/startup/mod.rs` | Skips `userspace_init` spawn in `kmain` when `direct-boot` is enabled |
| `kernel/src/startup/memory.rs` | Maps `env`/`bootstrap` storage so `KernelArgs::env()` stays reachable after CR3 switch |

Environment marker: `direct-boot=1` in the synthetic env block.

### 4. `build.rs` — assembler/compiler invocations removed

On x86/x86_64, upstream `build.rs` invoked `nasm` (trampolines) and `cc` (PVH stub). lerux `build.rs` only parses `config.toml` CPU features; see comments in `build.rs`.

---

## Config and build differences

| Item | Upstream | lerux |
|------|----------|-------|
| Crate root | `kernel/` directory | Repo root (`[[bin]] path = "kernel/src/main.rs"`) |
| `build-dependencies` | `toml`, `cc`, (nasm via script) | `toml` only |
| Cargo feature `direct-boot` | N/A | Enables PVH stub module + synthetic boot path |
| Linker script (x86_64 direct-boot) | N/A | `linkers/x86_64-direct.ld` — PVH note @ `0x100000`, stub @ `0x100020`, kernel load @ `0x200000` |
| Preferred build interface | Redox build system / `Makefile` in kernel repo | Root `justfile` (`just build-direct`, `just qemu-direct`, `just smoke`) |
| `objcopy` target | `*-unknown-redox` cross toolchain | Defaults to `llvm-objcopy` (override with `OBJCOPY=`) |
| Toolchain pin | Upstream CI choice | `rust-toolchain.toml` → `nightly-2026-05-24` |
| CI | Upstream GitLab | `.github/workflows/rust.yml` — fmt, clippy, `make check`, `just smoke` |

`config.toml.example` follows the upstream kconfig pattern and is not lerux-specific.

---

## QEMU tooling (lerux-only, partially non-Rust)

The `qemu/` directory is **not** part of upstream `redox-os/kernel`. It provides loaders and scripts for fast bring-up.

| Component | Language | Notes |
|-----------|----------|-------|
| `just qemu-direct` + PVH path | Rust kernel + `x86_64-direct.ld` | **Preferred** direct-boot smoke test; no external loader |
| `qemu/loader.asm`, `loader.S`, `mbr_stub.S`, `aarch64-loader.S` | NASM / GAS | Parallel loader track; **not** yet converted to Rust (see [PLAN.md](PLAN.md)) |

Memory map type `1` = usable RAM follows Redox bootloader convention (see comment in `qemu/loader.asm`).

---

## Intentionally unchanged Redox branding

lerux is a research fork, not a renamed OS (yet). These still identify as Redox at runtime:

- Serial: `"Redox OS starting..."` (`kernel/src/arch/*/start.rs`)
- `sys:uname`: `"Redox\n..."` (`kernel/src/scheme/sys/uname.rs`)
- Crate/docs title in `kernel/src/main.rs`

Smoke tests assert the upstream boot string on purpose.

---

## Upstream sync policy (open)

Not yet decided (see [PLAN.md](PLAN.md)):

- Track upstream closely vs. diverge for "Only Rust" purity
- Formal patch list / commit pin
- Attribution manifest beyond MIT license statement

When merging upstream changes, re-check the files listed in **Code changes** and **Config and build differences** above.

---

## Quick divergence checklist

| Change | Status |
|--------|--------|
| Remove `nasm` from kernel build | Done |
| Remove `cc`/`clang` from kernel build | Done |
| Embed SMP trampolines in Rust | Done |
| Pure-Rust PVH stub | Done |
| `direct-boot` feature + linker script | Done |
| Root `justfile` + smoke CI | Done |
| `VENDORED.md` (this file) | Done |
| Rust QEMU loaders | Planned |
| Upstream commit pin / patch log | Planned |
| Rename runtime branding to "lerux" | Not planned (compatibility) |
