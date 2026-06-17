# Redox Memory Management (RMM)

RMM is the lowest layer of the kernel's memory system: the part that understands
the exact page-table format of each CPU architecture and tracks which physical
frames are free. Everything above it builds policy on top of these primitives.

In lerux it is inlined into the kernel tree as
[`lerux-kernel/src/lerux-rmm/`](../../lerux-kernel/src/lerux-rmm/) (wired in via
`#[path]` in `main.rs`) so the kernel has zero external runtime dependencies. The
crate also contains a software-emulation mechanism for testing memory management.

## How it fits with the rest of the kernel

The memory system has three layers, from lowest to highest:

1. **`lerux-rmm/`** — hardware-aware bookkeeping: physical addresses, frames,
   page-table reading/writing, and the free-frame allocator. Think of it as the
   "driver" for the CPU's memory-management unit.
2. **[`memory/`](../../lerux-kernel/src/memory/mod.rs)** — the kernel's wrapper
   over RMM: `Frame`, allocation helpers, and per-frame reference counting so a
   frame shared between processes is freed only when the last user goes away.
3. **[`context/memory.rs`](../../lerux-kernel/src/context/memory.rs)** — per-process
   virtual address spaces: `mmap`, grants, copy-on-write, and page-fault handling.

For the beginner-oriented explanation of physical vs. virtual memory, pages,
frames, and page faults, see section 4 of
[architecture.md](architecture.md#4-memory-model-physical-frames-virtual-addresses-paging).
