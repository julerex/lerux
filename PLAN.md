# PLAN.md — lerux (Only Rust Redox) Development Roadmap

This document collects all potential next steps, ideas, and open questions that have been discussed during development. It serves as a living backlog.

Last updated: 2026-05-29 (direct-boot reaches the `kmain` idle loop; PVH stub now pure Rust — PR #3)

---

## 1. QEMU Bring-up & Early Boot (Highest Immediate Priority)

The current focus is getting the kernel to actually boot under QEMU and produce useful output.

**Direct-boot (`just qemu-direct`)** is the preferred fast path: QEMU `-kernel` + PVH note + `direct-boot` feature. Verified 2026-05-29 (PR #3): boots all the way through kernel init to the `kmain` idle loop, C-toolchain-free (pure-Rust PVH stub). See `NOTES.md` for the verified serial output and the root-cause fixes.

- [ ] Make the loader reliably consume the kernel ELF placed at `0x200000` via `-device loader` (parallel track; partially implemented in the fixed-address path).
- [x] Provide a realistic, minimal memory map for direct-boot (`kernel/src/startup/direct_boot.rs`).
- [ ] Create a minimal but valid `bootstrap` / initfs region (small tarball or in-memory structure) so `kmain` can proceed past early initialization.
- [x] Reach the first real kernel message: `"Redox OS starting..."` over serial (direct-boot).
- [x] Handle the first userspace bootstrap attempt without immediate panic — direct-boot skips userspace bootstrap by design.
- [x] Complete direct-boot through `kmain` idle loop (reached in PR #3; the `map_memory` stall was a missing `EFER.NXE` enable plus `env`/`bootstrap` mapping fixes — see `NOTES.md`).
- [x] Improve GDB experience:
  - [x] Dedicated `qemu/debug.sh` script
  - [x] Better symbol loading (`just gdb` / `debug.sh` load `build/kernel.sym` and `set language rust`)
  - [x] Common breakpoint / watch setups documented (`NOTES.md`; pre-set in `debug.sh`)
- [ ] Add support for passing kernel command-line / environment from the loader.
- [ ] Explore using Limine as a more capable bootloader for development (instead of the custom minimal loader).
- [ ] Add EFI stub / UEFI boot path (longer term but valuable for real hardware).

---

## 2. "Only Rust" Purity & Architecture

Core philosophy of the project.

- [x] Port the direct-boot PVH boot stub to pure Rust (`core::arch::global_asm!`) and drop the `cc`/`clang` build dependency (PR #3; see `kernel/src/arch/x86_shared/pvh_boot.rs`).
- [ ] Convert the QEMU loader itself to pure Rust (eliminate `loader.asm` + `loader.S` entirely).
- [ ] Investigate removing or dramatically simplifying the custom linker scripts (`linkers/*.ld`).
- [ ] Achieve fully `cargo`-only development builds (reduce or remove reliance on the `Makefile` for day-to-day work).
- [ ] Complete SMP bring-up on riscv64 and aarch64 (currently only x86 paths have real trampoline work).
- [ ] Audit the entire kernel for any remaining non-Rust codegen or build-time tools.
- [ ] Decide on long-term project layout:
  - Keep `kernel/` as a subdirectory forever?
  - Eventually flatten so the root crate *is* the kernel?
- [ ] Root-level Cargo workspace setup (so we can easily add loader, tests, userspace crates, etc. as members).
- [x] Document lerux vs upstream divergence baseline ([VENDORED.md](VENDORED.md)).
- [ ] Strategy for maintaining the vendored kernel vs. upstream Redox kernel over time (patch management, upstream sync policy — extend **VENDORED.md**).
- [ ] Proper attribution / licensing notes for the vendored Redox code.

---

## 3. Trampoline Validation & Maintenance

- [x] Automatic byte-for-byte comparison (`compare_trampoline_bytes.py`, `just validate-trampolines`).
- [x] Golden `.bin` files under `validation/trampolines/expected/` (embedded via `include_bytes!`).
- [x] CI job: `trampolines` in `.github/workflows/rust.yml`.
- [ ] Add an optional build-time check (in `build.rs` or a `cargo xtask`) that validates when nasm is available.
- [ ] Per-instruction disassembly comments in `asm/` or generated docs.

---

## 4. Tooling & Development Experience

- [x] Automated QEMU boot tests (boot the kernel, capture serial, assert on expected early messages). Implemented as `qemu/smoke-test.sh` / `just smoke`, run by the `smoke` CI job (`.github/workflows/rust.yml`); asserts the direct-boot idle marker and fails on panic/triple-fault/timeout.
- [ ] Better integration between the QEMU harness and the kernel's own testing story.
- [ ] `cargo xtask` (or similar) for common development tasks (build kernel + loader, run under QEMU, validate trampolines, etc.).
- [ ] Improve the root `README.md` with a proper "Getting Started" section once basic QEMU bring-up works.
- [ ] Add a `CONTRIBUTING.md` once the project stabilizes a bit.

---

## 5. Longer-Term / Ambitious Goals

- [ ] Minimal pure-Rust userspace (at least enough to get past `userspace_init` and spawn a simple `init` process).
- [ ] Full ACPI / device bring-up under QEMU (beyond the current serial path).
- [ ] Graphical debug / early framebuffer support.
- [ ] Real hardware bring-up (especially aarch64 and riscv64).
- [ ] Explore replacing more low-level pieces with pure Rust where feasible (e.g. parts of paging setup, GDT/IDT construction).
- [ ] Decide on a long-term bootloader strategy (custom minimal loader vs. Limine vs. custom EFI bootloader written in Rust).
- [ ] Multi-architecture CI (build + basic QEMU smoke tests for x86_64, i586, aarch64, riscv64).

---

## 6. Open Questions & Design Decisions

- How closely should we track upstream Redox kernel changes vs. diverge for "Only Rust" purity?
- What is the target "minimum viable OS" for the first real demo? (serial shell? graphical? networking?)
- Should the QEMU loader eventually become part of the main repository as a first-class Rust crate?
- How do we want to handle the bootstrap/initfs in the long term? (Embedded tar? Separate filesystem image? Built by a Rust tool?)

---

## How to Use This Document

- Add new items as they come up in discussion.
- Move completed items to a "Done" section or strike them through.
- Use checkboxes for tracking progress.
- Feel free to re-prioritize as the project evolves.

This document is intentionally broad — it exists to prevent good ideas from being lost.