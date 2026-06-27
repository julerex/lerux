# SMP Trampoline Validation

SMP AP trampoline binaries are assembled at kernel build time from NASM sources in
`lerux-kernel/src/asm/{x86,x86_64}/trampoline.asm` via `build.rs`. The output is
included in [`lerux-kernel/src/arch/x86_shared/trampoline.rs`](../../lerux-kernel/src/arch/x86_shared/trampoline.rs)
from `OUT_DIR/trampoline`.

## Layout

| Path | Purpose |
|------|---------|
| `lerux-kernel/src/asm/x86_64/trampoline.asm` | NASM source (x86_64 SMP bring-up) |
| `lerux-kernel/src/asm/x86/trampoline.asm` | NASM source (32-bit x86 variant) |
| `compare_trampoline_bytes.py` | Assemble + verify size/invariants |
| `validate-trampolines.sh` | Shell wrapper |

## Running validation

From the repo root:

```bash
just validate-trampolines
```

Requirements: **nasm** (also required for kernel builds on x86).

Unit tests (no nasm required — checks asm sources are present):

```bash
cargo test -p trampoline-validation
```

## CI

The GitHub Actions `trampolines` job runs `just validate-trampolines` and
`cargo test -p trampoline-validation` on every push/PR. The `check` job also
requires nasm because `build.rs` assembles trampolines during `make check`.

## After changing trampoline logic

1. Edit the `.asm` file(s) in `lerux-kernel/src/asm/`.
2. `./validate-trampolines.sh` — confirm pass.
3. Rebuild the kernel and run SMP smoke if applicable.

## Notes on the bytes

- Loaded at physical address `0x8000`.
- Data fields at offsets 8, 16, 24, 32 (`.ready`, `.args_ptr`, `.page_table`,
  `.code`) are patched by the BSP before SIPI (`acpi/madt/arch/x86.rs`).
- GDT placement and `lgdt` operands must stay consistent with `ORG 0x8000`.