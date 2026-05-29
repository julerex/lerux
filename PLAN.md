# PLAN.md — lerux (Only Rust Redox) Development Roadmap

This document collects all potential next steps, ideas, and open questions that have been discussed during development. It serves as a living backlog.

Last updated: 2026-05-29

---

## 1. QEMU Bring-up & Early Boot (Highest Immediate Priority)

The current focus is getting the kernel to actually boot under QEMU and produce useful output.

- [ ] Make the loader reliably consume the kernel ELF placed at `0x200000` via `-device loader` (partially implemented in the fixed-address path).
- [ ] Provide a realistic, minimal memory map in the loader (currently very fake).
- [ ] Create a minimal but valid `bootstrap` / initfs region (small tarball or in-memory structure) so `kmain` can proceed past early initialization.
- [ ] Reach the first real kernel message: `"Redox OS starting..."` over serial.
- [ ] Handle the first userspace bootstrap attempt without immediate panic (or at least reach a controlled failure point).
- [ ] Improve GDB experience:
  - Dedicated `qemu/debug.sh` script
  - Better symbol loading
  - Common breakpoint / watch setups documented
- [ ] Add support for passing kernel command-line / environment from the loader.
- [ ] Explore using Limine as a more capable bootloader for development (instead of the custom minimal loader).
- [ ] Add EFI stub / UEFI boot path (longer term but valuable for real hardware).

---

## 2. "Only Rust" Purity & Architecture

Core philosophy of the project.

- [ ] Convert the QEMU loader itself to pure Rust (eliminate `loader.asm` + `loader.S` entirely).
- [ ] Investigate removing or dramatically simplifying the custom linker scripts (`linkers/*.ld`).
- [ ] Achieve fully `cargo`-only development builds (reduce or remove reliance on the `Makefile` for day-to-day work).
- [ ] Complete SMP bring-up on riscv64 and aarch64 (currently only x86 paths have real trampoline work).
- [ ] Audit the entire kernel for any remaining non-Rust codegen or build-time tools.
- [ ] Decide on long-term project layout:
  - Keep `kernel/` as a subdirectory forever?
  - Eventually flatten so the root crate *is* the kernel?
- [ ] Root-level Cargo workspace setup (so we can easily add loader, tests, userspace crates, etc. as members).
- [ ] Strategy for maintaining the vendored kernel vs. upstream Redox kernel over time (VENDORED.md, patch management, etc.).
- [ ] Proper attribution / licensing notes for the vendored Redox code.

---

## 3. Trampoline Validation & Maintenance

We have good infrastructure now, but it can be stronger.

- [ ] Enhance `kernel/validation/trampolines/validate-trampolines.sh`:
  - Automatic byte-for-byte comparison against `trampoline.rs`
  - Exit non-zero on mismatch (good for CI)
- [ ] Add an optional build-time check (in `build.rs` or a `cargo xtask`) that validates the trampoline bytes when nasm is available.
- [ ] Add the validation as a GitHub Action / CI step.
- [ ] Improve comments in `trampoline.rs` with per-instruction disassembly or regeneration instructions.
- [ ] Consider storing the assembled `.bin` files in the repo (under `validation/`) as the canonical artifacts, with the `.asm` as human-readable source.

---

## 4. Tooling & Development Experience

- [ ] Automated QEMU boot tests (boot the kernel, capture serial, assert on expected early messages).
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