# lerux-kernel tests

Host-runnable tests for kernel-native logic live under [`unit/`](unit/) as the
standalone workspace crate **`kernel-unit-tests`**.

## Layout

| Path | Purpose |
|------|---------|
| [`unit/`](unit/) | Host unit tests (`cargo test -p kernel-unit-tests`) |
| [`../validation/trampolines/`](../validation/trampolines/) | SMP trampoline golden assets + `trampoline-validation` crate (not moved here) |

SMP trampoline byte validation is **not** under `tests/` — it is production
infrastructure (embedded `.bin` files, NASM sources, CI scripts). Run:

```bash
just validate-trampolines
cargo test -p trampoline-validation
```

## What belongs in `kernel-unit-tests`

- Pure logic that can be tested without the custom kernel target or private module access.
- Macros and helpers included from `lerux-kernel/src/` via `#[path]` when they have no kernel dependencies.
- Logic extracted into shared modules that both kernel and this crate can depend on (future).

## What stays elsewhere

| Kind | Location |
|------|----------|
| Trampoline golden bytes | `validation/trampolines/` (`trampoline-validation`) |
| Hardware / boot / IRQ paths | QEMU smoke (`just smoke`) |
| Private kernel modules needing full linkage | `#[cfg(test)]` in `src/` (rare) or future lib split |

## Next extraction candidates

See [`unit/EXTRACT_CANDIDATES.md`](unit/EXTRACT_CANDIDATES.md) for modules worth pulling into testable form.
