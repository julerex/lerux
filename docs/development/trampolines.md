# SMP Trampoline Byte Validation

This directory validates that the embedded SMP AP trampoline bytes in
`kernel/src/arch/x86_shared/trampoline.rs` are **byte-for-byte identical** to
what the original NASM sources produce.

## Layout

| Path | Purpose |
|------|---------|
| `asm/trampoline_x86_64.asm` | NASM source (from upstream `src/asm/x86_64/trampoline.asm`) |
| `asm/trampoline_x86.asm` | NASM source (from upstream `src/asm/x86/trampoline.asm`) |
| `expected/*.bin` | Golden binaries assembled from `asm/` (embedded via `include_bytes!`) |
| `compare_trampoline_bytes.py` | Assemble + compare logic |
| `validate-trampolines.sh` | Shell wrapper |

## Why this exists

As part of the lerux "Only Rust Redox" goal, the last external assembler
dependency (nasm for AP bring-up trampolines) was removed from the **kernel
build**. Upstream `redox-os/kernel` still assembled these via nasm in
`build.rs`; see root [docs/vendored.md](../../docs/vendored.md).

The kernel embeds the golden `.bin` files directly:

```rust
pub static TRAMPOLINE: &[u8] =
    include_bytes!("../../../validation/trampolines/expected/trampoline_x86_64.bin");
```

Because the trampoline contains baked absolute addresses and a GDT at fixed
offsets under `ORG 0x8000`, any drift from the NASM output would cause APs to
triple-fault silently during SMP bring-up.

## Running validation

From the repo root:

```bash
just validate-trampolines
```

Or from this directory:

```bash
./validate-trampolines.sh          # check (default)
./validate-trampolines.sh refresh  # regenerate expected/*.bin after asm edits
./validate-trampolines.sh print-rust  # print inline Rust arrays (legacy)
```

Requirements: **nasm** (dev/CI only — not needed to build the kernel).

Unit tests (no nasm required — compares against committed golden files):

```bash
cargo test --bin kernel trampoline
```

## CI

The GitHub Actions `trampolines` job runs both `just validate-trampolines` and
`cargo test --bin kernel trampoline` on every push/PR.

## After changing trampoline logic

1. Edit the `.asm` file(s) in `asm/`.
2. `./validate-trampolines.sh refresh` — updates `expected/*.bin`.
3. `./validate-trampolines.sh` — confirm pass.
4. Commit `asm/`, `expected/`, and any `trampoline.rs` changes together.

## Notes on the bytes

- Loaded at physical address `0x8000`.
- Data fields at offsets 8, 16, 24, 32 (`.ready`, `.args_ptr`, `.page_table`,
  `.code`) are patched by the BSP before SIPI (`acpi/madt/arch/x86.rs`).
- GDT placement and `lgdt` operands must stay consistent with `ORG 0x8000`.
