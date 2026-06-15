# Direct-boot smoke test notes

Bring-up log for the **lerux-only** `direct-boot` path (not in upstream Redox). For the full upstream divergence list see [VENDORED.md](VENDORED.md).

Last updated: 2026-05-30

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

Direct-boot intentionally **skips userspace bootstrap** in the default `direct-boot` build
(`kernel/src/startup/mod.rs`); kernel-only success means reaching `kmain` and idling. With
`direct-boot-userspace`, bootstrap runs and execs `init` (Phase B).

## Verified working (kernel-only)

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
  kernel::startup:INFO -- Bootstrap: 2B0750:2B7774
  kernel::startup::memory:INFO -- Memory: 205 MB
  kernel::startup::memory:INFO -- Paging: building new kernel page tables
  kernel::startup::memory:INFO -- Paging: switching to new kernel page tables
  kernel::startup::memory:INFO -- Paging: new kernel page tables active
  kernel::startup::memory:INFO -- Permanently used: 1484 KB
  kernel::startup:INFO -- direct-boot mode: skipping userspace bootstrap for kernel-only testing
  ```
  (`ACPI`/`MADT` warnings are expected: direct-boot provides no RSDP/ACPI tables.)

## Verified working (Phase B userspace)

With `just build-direct-userspace` + QEMU (or `just smoke-userspace`):

```
kernel::syscall::process:INFO -- Bootstrap entry point: 0x3000
init: switchroot to /scheme/initfs /scheme/initfs/etc
init: unit 50_rootfs.service not found
randd: Seeding failed, no entropy source.  Random numbers on this platform are NOT SECURE
init: switchroot to /usr /etc
```

`50_rootfs.service` is intentionally absent (Phase C). After `logd` starts, daemon
output may leave serial — the smoke test key marker is `init: switchroot to /scheme/initfs`.

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
| `frame 0x0 is reserved` panic in `KernelArgs::bootstrap()` | Direct-boot has no initfs, so `bootstrap_base` was `0` and `Frame::containing(0)` panicked. Point `bootstrap_base` at a valid frame with `bootstrap_size = 0` (never consumed in direct-boot) (`kernel/src/startup/direct_boot.rs`). **Phase A (2026-05-30):** real initfs embedded; non-zero `bootstrap_size`. |
| Bootstrap `#UD` at `xorps` (SSE) in userspace | Kernel set `CR4.OSXSAVE` but not `CR4_ENABLE_SSE` (OSFXSR). Bootstrap allocator uses SSE via build-std. Set `CR4_ENABLE_SSE` in `early_init` (`kernel/src/arch/x86_64/alternative.rs`). |
| Garbage bootstrap entry / truncated initfs copy | Use embedded `initfs_blob()` in `bootstrap_mem()`; `page_count` via `div_ceil(PAGE_SIZE)` (`kernel/src/syscall/process.rs`, `kernel/src/startup/mod.rs`). |

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
just smoke                 # build + boot + assert kernel idle (CI smoke test)
just smoke-userspace       # Phase B: bootstrap → init milestone
just build-direct-userspace
just qemu-direct-userspace
```

## Automated smoke test

`qemu/smoke-test.sh` (exposed as `just smoke`) boots the direct-boot kernel
headless, captures the serial console to `qemu-serial.log`, and asserts boot
reaches the `kmain` idle loop. Set `USERSPACE_SMOKE=1` (or run `just smoke-userspace`)
to assert `init: switchroot to /scheme/initfs` instead. It exits as soon as it sees
the success marker (so it does not wait out the timeout), and fails non-zero on a
missing marker, a kernel panic / triple fault, or a `$TIMEOUT`-second (default 90s)
timeout. No KVM is required, so it runs on plain GitHub runners.

**Kernel-only** checks (all must appear):

## Verified working (rustc-hosting smoke milestone — 2026-06-15)

With the direct-boot + userspace path solid, the rustc-hosting milestone (concrete success criterion) was landed as a thin vertical slice:

- Cross-built vendored `redoxfs` (for the host image recipe + future) and a tiny `userspace/rustc-smoke` stub ("rustc" binary) using the exact same hybrid `x86_64-unknown-redox` + in-tree sysroot machinery as the Phase B daemons.
- Staged both (plus supporting units) into `initfs-staging/`.
- `build-redoxfs-test-image` now produces a real mkfs'd 64 MiB image (host `redoxfs-mkfs` via correct CLI) + the cross stub (population of the image via host lib is ready for the block-driver follow-up; the first green used in-guest delivery).
- `qemu/smoke-test.sh` + `just` recipes fully support `RUSTC_SMOKE=1` (drive attachment, marker wait for all three, `[ ok ]`/`[MISS]` reporting, dedicated PASS message).
- Minor supporting tweaks: enlarged direct-boot memory map reservation (to accommodate the larger initfs blob), 1 GiB QEMU default for the smoke path, 50_rootfs.service wired as a stand-in that execs the stub (the vendored redoxfs memory mode + full service integration remain available for the next slice).

Serial from a passing `just smoke-rustc` (RUSTC markers emitted by the cross-compiled stub when init started the unit after switchroot):

```
kernel::arch::x86_shared::start:INFO -- Redox OS starting...
...
kernel::syscall::process:INFO -- Bootstrap entry point: 0x3000
init: switchroot to /scheme/initfs /scheme/initfs/etc
randd: Seeding failed, no entropy source.  Random numbers on this platform are NOT SECURE
redoxfs mounted
rustc 1.80.0-lerux-bootstrap (x86_64-unknown-redox) (lerux 2026-06)
rustc --version
lerux-bootstrap-compiled
init: switchroot to /usr /etc
...
SMOKE TEST PASSED: redoxfs mounted + bootstrap rustc ran and compiled on lerux (RUSTC markers present).
```

All regressions clean (`just check-only-rust`, `just smoke-userspace`, `just smoke`).

This is the first tangible proof of the project goal. The stub is a bootstrap/validation artifact (hybrid path); the long-term pure-runtime + real rustc comes after the Only Rust runtime port + AI cleanup of the now-landed redoxfs.

Next per plan: accelerate pure runtime port (especially for vendored components), AI co-pilot unsafe audit on redoxfs (allocator/block/fs layers first), smallest block exposure + flip to real DiskFile image, etc. See PLAN.md §8 and the post-green list.

- `Redox OS starting...`
- `Memory:`
- `Paging: new kernel page tables active`
- `Permanently used:`
- `direct-boot mode: skipping userspace bootstrap` ← idle reached (success)

**Userspace** (`USERSPACE_SMOKE=1`): same early markers plus
`init: switchroot to /scheme/initfs`; must **not** see the skip-userspace message.

CI runs kernel-only smoke as the `smoke` job in `.github/workflows/rust.yml`, alongside
`fmt` / `clippy` / `check`, and uploads `qemu-serial.log` as an artifact.

## Prerequisites (direct-boot)

- Rust nightly (`rust-toolchain.toml`)
- `llvm-objcopy` (or `OBJCOPY=`)
- `qemu-system-x86_64`

You do **not** need `nasm` or a C toolchain (`cc`/`clang`) for direct-boot. The PVH
stub is pure Rust (`kernel/src/arch/x86_shared/pvh_boot.rs`, `core::arch::global_asm!`).

## Next step

Only Rust step 2 is done (2026-05-31): `just build-sysroot` builds relibc from
`vendor/relibc/` into `.toolchain/`; init/daemons are static ELFs (no `libc.so` in
initfs staging); `just check-only-rust` enforces ELF + source policy.

Next: Phase C (ACPI/RSDP, `pcid`, virtio) or Only Rust step 3–4 (shrink allowlist,
remove `vendor/relibc/` entirely).
