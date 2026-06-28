# PLAN.md — lerux roadmap

Last updated: 2026-06-28 (Phase 2)

## Phase 1 — Bring-up

- [x] Pivot from Redox kernel to seL4 + Microkit
- [x] `just fetch` / `just build-sdk` / `just run` for aarch64 virt
- [x] Single hello protection domain
- [x] CI smoke test in Docker

## Phase 2 — Multi-PD IPC

- [x] Serial driver PD (PL011 on virt) — `userspace/pds/serial-driver/`
- [x] Two-PD system with Microkit channels (hello client + serial_driver)
- [x] Board-templated `.system` files — `userspace/systems/templates/` + `scripts/generate-system.py`

## Phase 3 — x86_64 (current)

- [x] Microkit board: `x86_64_generic` (QEMU generic PC; not `qemu_x86_64`)
- [x] `x86_64-sel4-microkit.json` target spec in `support/targets/`
- [x] `BOARD=x86_64_generic just run` (hello PD via kernel debug serial; userspace serial driver is aarch64-only for now)

## Phase 4 — Utilities

- Shared Rust crates for IPC helpers, logging, sync (`sel4-sync`, `sel4-logging`)
- Optional virtio block/net drivers from rust-sel4 examples

## Version alignment

| Component | Version |
|-----------|---------|
| seL4 | 15.0.0 |
| Microkit | 2.2.0 |
| rust-sel4 | v4.0.0 |