# SMP Trampoline Byte Validation

This directory exists to allow **deep, reproducible validation** of the pure-Rust SMP trampoline byte arrays used in `src/arch/x86_shared/trampoline.rs`.

## Why this exists

As part of the "Only Rust Redox" goal, the last external assembler dependency (nasm for the AP bring-up trampolines) was removed.

The trampolines are now embedded as raw `&[u8]` constants. Because they contain hand-written 16-bit real mode code + very specific absolute addresses and GDT layouts, it is important to be able to regenerate and verify the exact bytes at any time.

## How to run deep validation

```bash
cd kernel/validation/trampolines
chmod +x validate-trampolines.sh
./validate-trampolines.sh
```

This will:
- Assemble both `trampoline_x86.asm` and `trampoline_x86_64.asm` with nasm
- Print the exact bytes in Rust array syntax
- Show you what the current arrays in `trampoline.rs` should be compared against

Use `./validate-trampolines.sh update` to get clean output you can copy-paste.

## Maintaining the bytes over time

1. If you ever need to change the trampoline logic, edit the `.asm` files in this directory.
2. Run the validation script.
3. Copy the new arrays into `src/arch/x86_shared/trampoline.rs`.
4. Commit both the `.asm` sources (for auditability) and the updated `.rs` file.

The `.asm` files in this directory are the **source of truth** for the binary blobs.

## Notes on the bytes

- The trampolines are loaded at physical address `0x8000`.
- They contain baked absolute addresses (e.g. `0x8018` for the page table field).
- The GDT must be at a specific offset so the `lgdt` and far jumps have the correct values.
- Small differences in GDT placement or instruction encoding will cause APs to triple-fault silently.

This is why we keep the original NASM sources alongside the Rust blobs.
