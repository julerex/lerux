# Direct-boot smoke test notes

Bring-up log for the **lerux-only** `direct-boot` path (not in upstream Redox). For the full upstream divergence list see [VENDORED.md](VENDORED.md).

Last updated: 2026-05-29

## Summary

Direct-boot (`just qemu-direct`) is the preferred fast path: QEMU `-kernel` + PVH note + `direct-boot` feature. It now boots all the way through kernel init and reaches the idle loop.

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

- **Toolchain:** nightly, QEMU 6.2, `llvm-objcopy`
- **Build:** `just build-direct` → `build/kernel` with `linkers/x86_64-direct.ld` and Xen PVH note at `0x100020`
- **Boots to idle:** reaches `kmain`, prints the direct-boot skip message, and idles with no QEMU reset (QEMU only exits when killed / timed out).
- **Serial output:**
  ```
  kernel::arch::x86_shared::start:INFO -- Redox OS starting...
  kernel::startup:INFO -- Kernel: 0:0
  kernel::startup:INFO -- Env: 2BC080:2BC08E
  kernel::startup:INFO -- HWDESC: 0:0
  kernel::startup:INFO -- Areas: 2BB5F8:2BB688
  kernel::startup:INFO -- Bootstrap: 2BC0A0:2BC0A0
  kernel::startup::memory:INFO -- Memory: 205 MB
  kernel::startup::memory:INFO -- Paging: building new kernel page tables
  kernel::startup::memory:INFO -- Paging: switching to new kernel page tables
  kernel::startup::memory:INFO -- Paging: new kernel page tables active
  kernel::startup::memory:INFO -- Permanently used: 1484 KB
  kernel::startup:INFO -- direct-boot mode: skipping userspace bootstrap for kernel-only testing
  ```
  (`ACPI`/`MADT` warnings are expected: direct-boot provides no RSDP/ACPI tables.)

## Fixes applied (root causes found via GDB)

| Issue | Fix |
|--------|-----|
| QEMU ignored PVH stub | Xen note entry `0x100000` → **`0x100020`** (`kernel/src/arch/x86_shared/pvh_boot.rs`) |
| Kernel text not loaded at low phys | Load at **`0x200000`**, align virt to `0xFFFFFFFF80000000` (`linkers/x86_64-direct.ld`) |
| Higher-half fetch failed | **2 MiB huge pages** (`0x183`), separate **`PVH_PD_HIGH`**, **`PML4[256]`** for `PHYS_OFFSET` linear map |
| `phys_to_virt` overflow | **`virt_to_phys()`** for `KernelArgs` pointers (`kernel/src/startup/direct_boot.rs`) |
| Crash in graphical debug | Skip **`graphical_debug::init`** when `direct-boot` |
| **Reserved-bit `#PF` right after CR3 switch** | PVH stub only enabled `EFER.LME`; the kernel's page tables set the **NX** bit on data pages. Enable **`EFER.NXE` (1<<11)** alongside LME in `pvh_boot.rs`, otherwise NX is a reserved bit and the first NX-page access triple-faults. |
| `env()` unreachable after CR3 switch | Direct-boot folds `env` into the kernel image (mapped only at `KERNEL_OFFSET`). Register it as **`IdentityMap`** so `map_memory` also linear-maps it at `PHYS_OFFSET` (`kernel/src/startup/memory.rs`). |
| `frame 0x0 is reserved` panic in `KernelArgs::bootstrap()` | Direct-boot has no initfs, so `bootstrap_base` was `0` and `Frame::containing(0)` panicked. Point `bootstrap_base` at a valid frame with `bootstrap_size = 0` (never consumed in direct-boot) (`kernel/src/startup/direct_boot.rs`). |

## Debugging

`qemu/debug.sh` wraps the GDB-stub workflow (`-s -S -d int,cpu_reset`):

```bash
./qemu/debug.sh             # build + launch QEMU (paused) and attach GDB here
./qemu/debug.sh --no-gdb    # build + launch QEMU (paused); attach GDB yourself
```

Interrupt/reset logging goes to `qemu-int.log` — grep for `v=0e` (page fault) and
`Triple fault`/`check_exception` to find the first fault and its error code (the
`RSVD` bit and `CR2` are the key clues for paging bugs).

### GDB breakpoints (boot path)

`pvh_start32` and `kstart` are plain symbols; the Rust entry points need their full
path (bare `start`/`kmain` do not resolve).

| Breakpoint | Purpose |
|------------|---------|
| `pvh_start32` | PVH stub entered (32-bit) |
| `kstart` | Rust entry after stub |
| `kernel::arch::x86_shared::start::start` | Args + serial + paging |
| `kernel::startup::kmain` | BSP init complete |

## Commands

```bash
just build-direct
just qemu-direct           # boots to the idle loop
just qemu-direct -- -s -S  # + `just gdb` in another terminal, or use ./qemu/debug.sh
just smoke                 # build + boot + assert serial markers (CI smoke test)
```

## Automated smoke test

`qemu/smoke-test.sh` (exposed as `just smoke`) boots the direct-boot kernel
headless, captures the serial console to `qemu-serial.log`, and asserts boot
reaches the `kmain` idle loop. It exits as soon as it sees the idle marker (so it
does not wait out the timeout), and fails non-zero on a missing marker, a kernel
panic / triple fault, or a `$TIMEOUT`-second (default 90s) timeout. No KVM is
required, so it runs on plain GitHub runners.

It checks for these serial substrings (all must appear):

- `Redox OS starting...`
- `Memory:`
- `Paging: new kernel page tables active`
- `Permanently used:`
- `direct-boot mode: skipping userspace bootstrap` ← idle reached (success)

CI runs it as the `smoke` job in `.github/workflows/rust.yml`, alongside
`fmt` / `clippy` / `check`, and uploads `qemu-serial.log` as an artifact.

## Prerequisites (direct-boot)

- Rust nightly (`rust-toolchain.toml`)
- `llvm-objcopy` (or `OBJCOPY=`)
- `qemu-system-x86_64`

You do **not** need `nasm` or a C toolchain (`cc`/`clang`) for direct-boot. The PVH
stub is pure Rust (`kernel/src/arch/x86_shared/pvh_boot.rs`, `core::arch::global_asm!`).

## Next step

Direct-boot is green and C-toolchain-free, and an automated serial smoke test
(`just smoke`, CI `smoke` job) now guards it against regressions. Next candidates:

- Minimal `bootstrap`/initfs region so the **non**-direct-boot path can spawn
  `userspace_init` (the first true "redox-like OS" milestone).
- Convert the remaining `qemu/` loaders (`loader.asm`, `loader.S`, `mbr_stub.S`,
  `aarch64-loader.S`) to Rust.
