# Direct-boot smoke test notes

Last updated: 2026-05-29

## Summary

Direct-boot (`just qemu-direct`) is the preferred fast path: QEMU `-kernel` + PVH note + `direct-boot` feature.

```mermaid
flowchart LR
  QEMU["qemu -kernel"] --> PVH["pvh_start32"]
  PVH --> kstart["kstart"]
  kstart --> start["start()"]
  start --> kmain["kmain()"]
  kmain --> idle["run_userspace idle loop"]
```

Direct-boot intentionally **skips userspace bootstrap** (`kernel/src/startup/mod.rs`); full success means reaching kernel init and idling, not spawning init.

## Verified working

- **Toolchain:** nightly, QEMU 6.2, `llvm-objcopy`, `clang`
- **Build:** `just build-direct` → `build/kernel` with `linkers/x86_64-direct.ld` and Xen PVH note at `0x100020`
- **Serial output:**
  ```
  kernel::arch::x86_shared::start:INFO -- Redox OS starting...
  kernel::startup::memory:INFO -- Memory: 205 MB
  ```

## Fixes applied (root causes found via GDB)

| Issue | Fix |
|--------|-----|
| QEMU ignored PVH stub | Xen note entry `0x100000` → **`0x100020`** (`kernel/src/arch/x86_shared/pvh_boot.S`) |
| Kernel text not loaded at low phys | Load at **`0x200000`**, align virt to `0xFFFFFFFF80000000` (`linkers/x86_64-direct.ld`) |
| Higher-half fetch failed | **2 MiB huge pages** (`0x183`), separate **`PVH_PD_HIGH`**, **`PML4[256]`** for `PHYS_OFFSET` linear map |
| `phys_to_virt` overflow | **`virt_to_phys()`** for `KernelArgs` pointers (`kernel/src/startup/direct_boot.rs`) |
| Crash in graphical debug | Skip **`graphical_debug::init`** when `direct-boot` |
| Identity-map conflicts | Skip env/bootstrap **IdentityMap** for direct-boot |

## Not yet reached (follow-up)

- **`direct-boot mode: skipping userspace bootstrap`** — needs `kmain`; blocked in **`map_memory`** after the memory-size log
- **Stable idle loop** — QEMU exits ~1s after the memory log
- Plan success criteria not fully met: `args.print()` debug dump (no-op on x86 `debug!`), `KernelArgs` dump over serial

## Commands

```bash
just build-direct
just qemu-direct          # serial: Redox OS starting + Memory: … MB
just qemu-direct -- -s -S # + just gdb for debugging
```

## GDB breakpoints (boot path)

| Symbol | Purpose |
|--------|---------|
| `pvh_start32` | PVH stub entered |
| `kstart` | Rust entry after stub |
| `start` | Args + serial + paging |
| `kmain` | BSP init complete |

## Prerequisites (direct-boot)

- Rust nightly (`rust-toolchain.toml`)
- `clang` (recommended; builds PVH stub via `build.rs`)
- `llvm-objcopy` (or `OBJCOPY=`)
- `qemu-system-x86_64`

You do **not** need `nasm` for direct-boot (PVH stub is `pvh_boot.S` via `cc`/`clang`).

## Next step

Debug why **`map_memory`** does not return (likely early page-table / linear-map setup with the PVH bootstrap tables) to reach `kmain` and the idle loop.
