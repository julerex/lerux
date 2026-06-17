# Extractable kernel logic for `kernel-unit-tests`

Candidates for the next wave of host unit tests. Each needs a small extraction
before this standalone crate can cover it (no private `kernel` module access).

## `cpu_set` — `parts()` bit decomposition

**Source:** [`src/cpu_set.rs`](../../src/cpu_set.rs) (`fn parts`, lines 59–61)

**Pure logic:** map `LogicalCpuId` → `(word_index, bit_index)` for the CPU mask.

**Extraction:** publish `fn logical_cpu_parts(id: u32) -> (usize, u32)` in
`common/` or keep as a `#[path]`-included test helper. `LogicalCpuSet::contains`
/ `iter` can then be tested without `CPU_COUNT` or `cpu_count()`.

**Blocked today:** `LogicalCpuId::next()` and `Display for LogicalCpuSet` depend
on kernel globals.

## `startup/direct_boot` — `virt_to_phys`

**Source:** [`src/startup/direct_boot.rs`](../../src/startup/direct_boot.rs) (lines 29–33)

**Pure logic:** `virt.wrapping_sub(kernel_offset).wrapping_add(KERNEL_LOAD_PHYS)`.

**Extraction:** factor into `fn virt_to_phys_for_load(virt, kernel_offset, load_phys) -> u64`
with constants `KERNEL_LOAD_PHYS = 0x200_000` tested independently of
`kernel_executable_offsets::KERNEL_OFFSET()` (linker symbol).

**Blocked today:** private fn inside `direct_boot`; `KERNEL_OFFSET()` is a linker symbol.

## `memory/page` — page rounding

**Source:** [`src/memory/page.rs`](../../src/memory/page.rs) (`round_down_pages`, `round_up_pages`, `Page` helpers)

**Pure logic:** page alignment arithmetic (`div_floor`, `next_multiple_of`).

**Extraction:** thin pure functions parameterized by `PAGE_SIZE`, or test via
`lerux-rmm` std tests once page-size constants are shared.

**Blocked today:** `PAGE_SIZE` comes from `RmmA::PAGE_SIZE` (arch-specific rmm).

## Priority order

1. `cpu_set::parts` — smallest, no arch coupling
2. `memory/page` rounding — depends on documenting/fixing page size for host tests
3. `direct_boot::virt_to_phys` — needs offset injection for testability
