# Postmortem: incorrect inline trampoline byte arrays

This document explains why the original hand-embedded SMP trampoline bytes in
`kernel/src/arch/x86_shared/trampoline.rs` did not match the NASM sources they
were meant to replace, and why that went undetected until automatic validation
was added.

For the current validation workflow, see
[`kernel/validation/trampolines/README.md`](../kernel/validation/trampolines/README.md).

---

## Root cause: hand-transcribed bytes, not NASM output

Everything traces back to commit **`d57c9e0`** (“Initial major progress toward
Only Rust Redox”), which introduced `trampoline.rs` in the same commit that
removed NASM from `build.rs`.

The commit message says the arrays hold the **“exact bytes produced by nasm”**,
but git history shows they were **written by hand from reading the assembly
logic**, not copied from a NASM build:

- Each region has comment annotations like `// mov edi, [0x8018]` and
  `// lgdt [0x808a]`.
- The byte patterns match those *semantic* guesses, not what NASM actually
  emits.

Comparing `d57c9e0`’s arrays to NASM output from the sources in
`kernel/validation/trampolines/asm/`:

| Arch   | Match before first error      | Total size (rust vs nasm) |
|--------|-------------------------------|---------------------------|
| x86_64 | 52 bytes (through `mov sp, 0`) | 208 vs 202                |
| x86    | 1 byte (`0xeb` only)           | 186 vs 175                |

The first x86_64 divergence is at the **`mov edi, [page_table]`** instruction:

- **Rust (wrong):** `8b 3e 18 80 00 00` — 16-bit `mov di, [disp32]`
- **NASM (correct):** `66 8b 3e 18 80` — needs the **`0x66` operand-size prefix**
  in 16-bit mode to load into `edi`

Other mistakes follow the same pattern — correct *intent*, wrong *encoding* or
wrong *layout-derived addresses*:

| Region           | What rust assumed                         | What NASM actually does                                      |
|------------------|-------------------------------------------|--------------------------------------------------------------|
| Enable paging    | `mov ebx, [0x8028]` then OR/modify        | `mov ebx, cr0` (`0f 20 c3`) — comment even says “cr0 shadow” |
| GDT load         | `lgdt [0x808a]` hardcoded                 | `lgdt [gdtr]` at **`0x80a8`** (address depends on layout)    |
| Long-mode entry  | `mov rax, 0x10` (REX + imm64)             | Different encoding (`b8 10 00 00 00` + segment loads)        |
| x86 header       | `jmp` displacement **`0x1e`**             | **`0x26`** (different layout assumption)                       |

The author understood the trampoline *algorithm* but not the low-level encoding
details (16 vs 32-bit operand sizes, where NASM places `gdtr`, etc.).

---

## Why validation didn’t catch it

The same commit added `kernel/validation/trampolines/`, but it could not have
caught the mismatch in practice:

1. **No automatic comparison** — `validate-trampolines.sh` only printed NASM
   output and said *“NOTE: Manually compare”*. It never diffed against
   `trampoline.rs`.

2. **Broken path to `trampoline.rs` from day one** — the script used:

   ```bash
   KERNEL_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"   # repo root
   TRAMPOLINE_RS="$KERNEL_ROOT/src/arch/x86_shared/trampoline.rs"
   ```

   but the file lives at **`kernel/src/arch/x86_shared/trampoline.rs`**. Running
   the script always printed *“Could not find …”* and skipped comparison. That
   path bug survived the later move of `Cargo.toml` to the repo root (`0f1e081`
   only reformatted `trampoline.rs`; the bytes were unchanged).

3. **NASM removed before extraction** — `build.rs` dropped the NASM invocation
   in the same commit, so there was no build-time artifact to copy from.

4. **`asm/` gitignored** — sources lived only as heredocs inside the script,
   not as committed files you would assemble and diff against.

5. **No CI check** — nothing in `.github/workflows/rust.yml` ran trampoline
   validation until automatic checks were added.

6. **No runtime test coverage** — direct-boot smoke tests (`just smoke`) never
   start APs via SIPI, so a bad trampoline would not fail CI. It would only
   surface with **`multi_core` + ACPI AP bring-up**.

---

## Timeline summary

```
d57c9e0  Hand-write byte arrays; remove NASM from build.rs
         Add validation script that can't find trampoline.rs and doesn't auto-compare
         ↓
0f1e081  cargo fmt on trampoline.rs only (bytes still wrong)
         ↓
…        Several direct-boot PRs; trampoline bytes never revisited
         ↓
(now)    Automatic validation + include_bytes! from golden .bin files
```

---

## Bottom line

The arrays were constructed as annotated pseudocode bytes, not NASM output. The
validation tooling was described as complete in the commit message, but it was
manual-only, pointed at the wrong file, and was never run in CI — so the
mismatch persisted undetected until real byte-for-byte checks were added.

---

## Related files

| Path | Role |
|------|------|
| [`kernel/src/arch/x86_shared/trampoline.rs`](../kernel/src/arch/x86_shared/trampoline.rs) | Embeds golden bytes via `include_bytes!` |
| [`kernel/validation/trampolines/asm/`](../kernel/validation/trampolines/asm/) | Committed NASM sources (source of truth) |
| [`kernel/validation/trampolines/expected/`](../kernel/validation/trampolines/expected/) | Golden `.bin` files assembled from `asm/` |
| [`kernel/validation/trampolines/compare_trampoline_bytes.py`](../kernel/validation/trampolines/compare_trampoline_bytes.py) | Automatic byte-for-byte comparison |
| [`VENDORED.md`](../VENDORED.md) | lerux vs upstream Redox divergence overview |
