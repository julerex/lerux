# PLAN.md — lerux roadmap

Last updated: 2026-06-28 (seL4 pivot)

## Phase 1 — Bring-up (current)

- [x] Pivot from Redox kernel to seL4 + Microkit
- [x] `just fetch` / `just build-sdk` / `just run` for aarch64 virt
- [x] Single hello protection domain
- [x] CI smoke test in Docker

## Phase 2 — Multi-PD IPC

- Add serial driver PD (PL011 on virt)
- Two-PD system with Microkit channels (pattern: [rust-microkit-demo](https://github.com/seL4/rust-microkit-demo))
- Template `.system` files for board-specific MMIO addresses

## Phase 3 — x86_64

- Confirm Microkit board name (`qemu_x86_64` or pc99) after SDK build
- Add `x86_64-sel4-microkit.json` target spec (generate via `sel4-generate-target-specs` if needed)
- `BOARD=qemu_x86_64 just run`

## Phase 4 — Utilities

- Shared Rust crates for IPC helpers, logging, sync (`sel4-sync`, `sel4-logging`)
- Optional virtio block/net drivers from rust-sel4 examples

## Version alignment

| Component | Version |
|-----------|---------|
| seL4 | 15.0.0 |
| Microkit | 2.2.0 |
| rust-sel4 | v4.0.0 |