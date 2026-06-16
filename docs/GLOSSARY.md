# Glossary

Terms and concepts used across the lerux repository. Entries are grouped by topic;
within each section they are alphabetical.

For lerux-only names, files, and commands, see **[lerux-specific names](#lerux-specific-names)** below.

---

## General software & validation terms

**Bring-up**
: The process of getting code to run on real or emulated hardware for the first time — boot, early init, serial output, and basic correctness before full features exist.

**CI (continuous integration)**
: Automated checks on every push/PR. In this repo: formatting, clippy, `make check`, trampoline validation, and the QEMU smoke test (see `.github/workflows/rust.yml`).

**Golden file**
: A committed reference artifact whose contents are treated as the source of truth for regression checks. Here, `kernel/validation/trampolines/expected/*.bin` are golden binaries: NASM assembles `asm/*.asm`, the output is stored in `expected/`, and tests/scripts assert that embedded kernel bytes match exactly. If you intentionally change the assembly, regenerate golden files with `./validate-trampolines.sh refresh` and commit the update.

**Postmortem**
: A written retrospective explaining how something went wrong and why it was not caught earlier. See [trampoline-bytes-postmortem.md](trampoline-bytes-postmortem.md) (historical; now also referenced from [docs/development/trampolines.md](development/trampolines.md)).

**Smoke test**
: A minimal end-to-end test that proves the system basically works (boots, prints expected output, does not crash). lerux’s smoke test runs QEMU direct-boot headless and asserts on serial markers such as `"Redox OS starting..."` and the direct-boot idle message (`just smoke`).

**Upstream**
: The original project lerux derives from — here, [redox-os/kernel](https://gitlab.redox-os.org/redox-os/kernel). “Tracking upstream” means merging or comparing against that project; “divergence” is documented in [vendored.md](vendored.md).

**Vendoring**
: Copying another project’s source into this repository (`kernel/`) and building from that copy instead of fetching it as a dependency. lerux vendors the Redox microkernel (~2026-05 snapshot).

---

## Build & toolchain

**`-Z build-std`**
: Cargo flag to compile `core` and `alloc` for a custom target (required for bare-metal kernel builds).

**`build.rs`**
: Rust build script run before compilation. In lerux it parses `config.toml` CPU features; upstream also used it to invoke NASM and a C compiler for boot stubs.

**`config.toml` / kconfig**
: Kernel configuration file (copied from `config.toml.example` if missing). Lists per-arch CPU features as `always` / `never` / `auto`; `build.rs` turns these into `cfg(cpu_feature_…)` flags.

**`global_asm!`**
: Rust macro (`core::arch::global_asm!`) that embeds assembly text assembled by LLVM/rustc. lerux uses it for the PVH boot stub in `pvh_boot.rs` instead of a separate `.S` file and C compiler.

**`include_bytes!`**
: Rust macro that embeds a file’s raw bytes into the binary at compile time. Trampoline blobs are loaded from golden `.bin` files this way.

**JSON target spec**
: A `*.json` file under `targets/` describing a custom Rust target (e.g. `x86_64-unknown-kernel.json`). Used with `RUST_TARGET_PATH` and `-Z json-target-spec`.

**`justfile`**
: Command runner ( [just](https://github.com/casey/just) ) with recipes for build, QEMU, GDB, smoke test, and trampoline validation. Preferred over the root `Makefile` for daily work.

**`llvm-objcopy` / `objcopy`**
: Tool that strips debug sections from the linked kernel ELF to produce the raw `build/kernel` image QEMU loads. Default in the justfile; override with `OBJCOPY=`.

**NASM**
: Netwide Assembler. **Not required** to build the lerux kernel. Still used in **dev/CI validation** to re-assemble trampoline sources and verify golden files.

**Only Rust**
: lerux project goal: eliminate non-Rust build-time tooling (external assemblers, C compilers for kernel build glue) where practical, while keeping Redox kernel design.

---

## Boot & x86 architecture

**ACPI (Advanced Configuration and Power Interface)**
: Firmware tables describing hardware. On x86 the kernel often locates them via **RSDP**. AP startup and device enumeration depend on ACPI when enabled (`acpi` feature).

**AP (Application Processor)**
: A CPU core other than the bootstrap processor. Brought up via the SMP trampoline and IPIs.

**Bootstrap / initfs**
: Contiguous physical memory region passed in `KernelArgs` containing the initial userspace payload (Redox initfs tarball). In **direct-boot** this is stubbed out and userspace bootstrap is skipped.

**BSP (Bootstrap Processor)**
: The primary CPU that runs kernel entry first; it copies the trampoline and sends SIPIs to APs.

**CR3**
: x86 control register holding the physical address of the page table root (PML4). The trampoline loads CR3 from a field patched by the BSP before SIPI.

**Direct-boot**
: lerux Cargo feature and boot path: kernel synthesizes its own `KernelArgs` for `qemu -kernel` without the Redox bootloader or initfs. See [building/standalone.md](../building/standalone.md).

**EFER**
: Extended Feature Enable Register (MSR `0xC0000080`). Long mode and NX enablement; the PVH stub must set **LME** and **NXE** before paging with NX bits.

**GDT (Global Descriptor Table)**
: x86 structure defining segment descriptors. The SMP trampoline installs a minimal GDT for the transition to protected/long mode.

**Higher-half kernel**
: Kernel linked to run at a high virtual address (here `0xFFFFFFFF80000000` / `KERNEL_OFFSET`) while loaded at a lower physical address. Linker scripts under `linkers/` define the mapping.

**IDT (Interrupt Descriptor Table)**
: x86 table of interrupt and exception handlers; set up during early kernel init.

**INIT IPI / SIPI**
: Inter-processor interrupts used for SMP bring-up: **INIT** resets an AP; **SIPI** (Startup IPI) makes it jump to the trampoline at physical `0x8000`.

**`KernelArgs`**
: Redox’s custom bootloader → kernel handoff structure (physical addresses and sizes for kernel image, stack, env block, hardware descriptor, memory map, bootstrap/initfs). Defined in `kernel/src/startup/mod.rs`. Not Multiboot2 or Limine.

**`kstart` / `kstart_ap`**
: Low-level assembly entry symbols. **`kstart`** is the BSP entry (naked stub → Rust `start()`). **`kstart_ap`** is where APs land after the trampoline.

**`kmain`**
: Primary Rust kernel entry after arch setup; initializes contexts, schemes, and (unless direct-boot) spawns userspace bootstrap.

**Long mode**
: x86-64 64-bit execution mode. Entered after enabling paging with EFER.LME set and a far jump.

**Memory map (“areas”)**
: Array of `{ base, size, kind }` entries in `KernelArgs` describing RAM, reserved regions, kernel image, etc. Direct-boot uses a static map tuned for typical QEMU machines.

**Multiboot2 / Limine**
: Common open boot protocols. **Redox/lerux do not use them** for the main kernel handoff; they use `KernelArgs` instead. Limine is listed as a possible future dev bootloader in [plan.md](plan.md).

**ORG 0x8000**
: NASM directive placing the SMP trampoline at physical address `0x8000`, where SIPI startup code expects it.

**PVH (Para-Virtualized Hardware)**
: QEMU/x86 boot convention using a Xen-style ELF note so `qemu -kernel` can find a 32-bit entry stub. lerux places the note at `0x100000` and stub at `0x100020` via `linkers/x86_64-direct.ld`.

**`pvh_start32`**
: 32-bit entry point in the PVH stub; builds page tables, enables long mode, jumps to **`kstart`**.

**RSDP (Root System Description Pointer)**
: ACPI root pointer. Passed in `KernelArgs.hwdesc` when available; direct-boot leaves it zero and skips ACPI init.

**RMM (Redox Memory Management)**
: In-kernel crate (`kernel/rmm/`) abstracting page tables, frames, and mapping. “Initialize RMM” is an early boot milestone.

**SMP (Symmetric Multiprocessing)**
: Multi-CPU operation. Requires the trampoline, ACPI MADT parsing, and IPI support (`multi_core` feature).

**Trampoline (SMP)**
: Small real-mode binary copied to physical `0x8000` so APs can switch to long/protected mode and jump to `kstart_ap`. Embedded as bytes in `trampoline.rs` (from golden files).

**Triple fault**
: x86 unrecoverable CPU reset from nested faults. A bad trampoline often causes APs to triple-fault **silently** (no useful serial output).

**Userspace bootstrap**
: First userspace process spawned from the initfs region (`userspace_init`). Skipped in direct-boot mode.

**Xen ELF note (`.note.Xen`)**
: ELF section QEMU reads to find the PVH entry point for `-kernel` loads.

---

## Redox kernel concepts

**Context**
: A schedulable execution unit (thread/process kernel state) in the Redox kernel.

**Initfs**
: Initial filesystem archive the Redox bootloader places in memory; first userspace runs from it.

**Microkernel**
: Architecture where most OS services run in userspace; the kernel provides syscalls, memory, scheduling, and drivers for core hardware. Redox’s kernel is a microkernel.

**Scheme**
: Redox’s VFS-like abstraction: paths like `file:`, `sys:`, `acpi:` are handled by scheme providers in the kernel. Syscalls route through schemes.

**`sys:uname`**
: Scheme resource returning OS name and version. Still reports **“Redox”** in lerux builds for compatibility.

**Syscall**
: Userspace → kernel interface. Implemented via `redox_syscall` and arch-specific entry (`syscall` instruction on x86_64).

---

## ARM / RISC-V (multi-arch context)

**DTB / FDT (Device Tree Blob / Flattened Device Tree)**
: Binary hardware description used on aarch64 and riscv64 instead of ACPI RSDP. Passed via `KernelArgs.hwdesc` on those architectures.

---

## QEMU & debugging

**GDB stub**
: QEMU `-s` exposes a remote GDB server on port 1234; `-S` stops at entry. Use `just gdb` or `qemu/debug.sh`.

**`-kernel`**
: QEMU flag loading a bare kernel ELF/image directly (used by direct-boot), as opposed to booting a full disk/BIOS image.

**QEMU harness (`qemu/`)**
: lerux-only scripts and loaders for booting under QEMU without the full Redox image build.

**Serial (mon:stdio)**
: Redirects emulated UART output to the terminal; primary observability channel during bring-up.

---

## lerux-specific names

Named artifacts, features, paths, and commands that exist **only in this repository** (or with lerux-specific meaning).

| Name | What it is |
|------|------------|
| **lerux** | This project: “Only Rust Redox” — vendored Redox kernel + standalone build/QEMU tooling. |
| **`direct-boot`** | Cargo feature enabling synthetic `KernelArgs`, PVH stub module, and `x86_64-direct.ld`. Env marker: `direct-boot=1`. |
| **`direct_boot.rs`** | Module (`kernel/src/startup/direct_boot.rs`) that builds fake boot info for QEMU `-kernel`. |
| **`get_direct_boot_args()`** | Function returning the static synthetic `KernelArgs` used when `direct-boot` is enabled. |
| **`building/standalone.md`** | Doc for kernel-only builds without the Redox build system. |
| **[vendored.md](vendored.md)** | Canonical list of intentional differences from upstream `redox-os/kernel`. |
| **`PLAN.md`** | Living development roadmap and open questions. |
| **`NOTES.md`** | Direct-boot bring-up log and verified serial output. |
| **`linkers/x86_64-direct.ld`** | Linker script for PVH note, stub, and kernel load @ 2 MiB (direct-boot only). |
| **`pvh_boot.rs`** | Pure-Rust PVH 32→64 stub (`global_asm!`); replaces upstream `pvh_boot.S`. |
| **`kernel/validation/trampolines/`** | Trampoline NASM sources, golden bins, and validation scripts. |
| **`compare_trampoline_bytes.py`** | Assembles `asm/*.asm` and byte-compares against `expected/` and `trampoline.rs`. |
| **`validate-trampolines.sh`** | Shell wrapper: `check`, `refresh`, `print-rust`. |
| **`just build-direct`** | Release build with `KERNEL_CARGO_FEATURES=direct-boot`. |
| **`just qemu-direct`** | Build direct-boot kernel and run in QEMU. |
| **`just smoke`** | CI smoke test: headless QEMU + serial assertions (`qemu/smoke-test.sh`). |
| **`just validate-trampolines`** | Run full NASM vs golden vs embedded byte check. |
| **`just gdb`** | Attach GDB to QEMU on localhost:1234 with kernel symbols. |
| **`qemu/smoke-test.sh`** | Asserts direct-boot serial markers; used by CI. |
| **`qemu/debug.sh`** | Build, launch QEMU paused with GDB stub, optional auto-attach. |
| **`qemu/run.sh`** | General QEMU launcher (loader-based path, parallel to direct-boot). |
| **`docs/trampoline-bytes-postmortem.md`** | Retrospective on incorrect hand-written trampoline bytes. |
| **`docs/GLOSSARY.md`** | This file. |
| **Phase 1** | Current project stage (README): Only Rust build milestones + direct-boot to idle loop; full OS not yet. |
| **Repo-root crate** | lerux puts `Cargo.toml` at the repository root while sources live under `kernel/` — unlike upstream where the kernel directory *is* the crate root. |

---

## See also

| Document | Contents |
|----------|----------|
| [README.md](../README.md) | Project overview and status |
| [vendored.md](vendored.md) | Upstream divergence |
| [building/standalone.md](building/standalone.md) | Direct-boot build/run |
| [development/qemu.md](development/qemu.md) | QEMU harness and handoff protocol |
| [development/trampolines.md](development/trampolines.md) | Trampoline golden-file workflow + validation |
| [trampoline-bytes-postmortem.md](trampoline-bytes-postmortem.md) | Why bad bytes slipped through (historical) |
