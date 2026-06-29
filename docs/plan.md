# PLAN.md — lerux roadmap

Last updated: 2026-06-29 (Phase 15)

## Phase 1 — Bring-up

- [x] Pivot from Redox kernel to seL4 + Microkit
- [x] `just fetch` / `just build-sdk` / `just run` for aarch64 virt
- [x] Single hello protection domain
- [x] CI smoke test in Docker

## Phase 2 — Multi-PD IPC

- [x] Serial driver PD (PL011 on virt) — `userspace/pds/serial-driver/`
- [x] Two-PD system with Microkit channels (hello client + serial_driver)
- [x] Board-templated `.system` files — `userspace/systems/templates/` + `scripts/generate-system.py`

## Phase 3 — x86_64

- [x] Microkit board: `x86_64_generic` (QEMU generic PC; not `qemu_x86_64`)
- [x] `x86_64-sel4-microkit.json` target spec in `support/targets/`
- [x] `BOARD=x86_64_generic just run` (NS16550 COM1 driver PD over I/O port 0x3f8)

## Phase 4 — Utilities

- [x] `lerux-logging` — debug-print and serial-IPC sinks on `sel4-logging`
- [x] `lerux-ipc` — typed postcard RPC re-exports (`sel4-microkit-simple-ipc`)
- [x] `lerux-sync` — notification mutex aliases on `sel4-sync`
- [x] Virtio block/net driver PDs (`virtio-blk-driver`, `virtio-net-driver`) on `qemu_virt_aarch64_virtio`

## Phase 5 — CI & Ops

- [x] x86_64 smoke job in GitHub Actions
- [x] Virtio smoke job in GitHub Actions
- [x] Single SDK build (`MICROKIT_BOARDS=qemu_virt_aarch64,x86_64_generic`) shared across smoke jobs
- [x] GHA caching for workspace, SDK, and per-board `build/` targets
- [x] `just test-all` local CI mirror

## Phase 6 — Virtio block I/O

- [x] Map client DMA + blk ring buffers into a client PD
- [x] Read block 0 from `support/disk.img` via shared ring buffers
- [x] Extend `just test-virtio` to verify block data

## Phase 7 — x86 serial IRQ/RX

- [x] COM1 IRQ in x86 system template
- [x] IRQ-driven RX in `ns16550.rs`

## Phase 8 — Custom IPC and minimal services

- [x] `lerux-interface-types` crate with postcard RPC messages
- [x] `echo-server` + `echo-client` PDs using `lerux-ipc`
- [x] Optional: timer/RTC/init PD vertical slice (`pl031-driver`, `boot-init`, `sp804-driver`)

## Phase 9 — RISC-V bring-up

- [x] Microkit board: `qemu_virt_riscv64` (QEMU RISC-V virt)
- [x] `riscv64-sel4-microkit.json` target spec in `support/targets/`
- [x] NS16550 MMIO serial driver PD at `0x1_000_0000` (PLIC IRQ 10)
- [x] `BOARD=qemu_virt_riscv64 just run` / `just test-riscv`
- [x] RISC-V toolchain + QEMU in Docker image
- [x] RISC-V smoke job in GitHub Actions

## Phase 10 — Virtio net I/O

- [x] Map virtio-net shared memory regions into hello PD
- [x] `sel4-shared-ring-buffer-smoltcp` client in hello
- [x] UDP TX smoke to QEMU user netdev (`10.0.2.2`)
- [x] Extend `just test-virtio` to expect `virtio-net: TX ok`

## Phase 11 — Cross-arch services and net RX

- [x] Echo IPC on `qemu_virt_riscv64_echo` (`just test-riscv-echo`)
- [x] Virtio block/net on `qemu_virt_riscv64_virtio` (`just test-riscv-virtio`; QEMU needs `bus=virtio-mmio-bus.N`)
- [x] TCP loopback RX smoke (`virtio-net: TCP RX ok`) on virtio boards
- [x] RTC smoke on `qemu_virt_aarch64_init` (`just test-init`)

## Phase 12 — SP804 timer in init smoke

- [x] Wire `sp804-driver` into `init.system.template` (MMIO `0x90d0000`, IRQ 43)
- [x] `boot-init` reads elapsed time via `TimerClient` after RTC
- [x] Patched QEMU for virt SP804 (`scripts/install-qemu-sp804.sh`; rust-sel4 `arm-virt-sp804` patch)
- [x] `just test-init` expects `lerux-init: timer ok` and `lerux-init: init ok`

## Phase 13 — Ops and docs

- [x] GHA cache for patched QEMU (`deps/toolchains/qemu-sp804`) on init smoke job
- [x] README: full smoke matrix, init board section, SP804 QEMU note
- [x] `install-qemu-sp804.sh`: reuse build tree when install binary missing but configure done
- [x] Plan/README/justfile comment hygiene

## Phase 14 — Cross-arch echo and init parity

- [x] x86 echo IPC smoke (`x86_64_generic_echo`, `just test-x86-echo`)
- [x] `echo-x86.system.template` (COM1 ioport + IOAPIC IRQ)
- [x] CI matrix job `x86-echo`
- [x] Cross-arch smoke parity documented (init remains aarch64-only)

### Cross-arch smoke parity

| Smoke | aarch64 | RISC-V | x86 |
|-------|---------|--------|-----|
| Serial hello | yes | yes | yes |
| Echo IPC | yes | yes | yes |
| Virtio blk/net | yes | yes | no (PCI virtio; deferred) |
| Init RTC+timer | yes | no | no |

Init (`just test-init`) uses PL031 + SP804 drivers from rust-sel4 v4.0.0, which target QEMU aarch64 virt MMIO only. RISC-V virt and x86 PC do not expose those devices in stock QEMU, and there are no equivalent rust-sel4 driver crates yet.

## Phase 15 — Composed aarch64 system

- [x] `composed.system.template`: boot-init + hello + init drivers + virtio drivers (7 PDs)
- [x] Board `qemu_virt_aarch64_composed` (`just test-composed`)
- [x] `boot-init` uses serial IPC for RTC/timer; `hello` uses virtio with debug-print (serial-driver is single-client)
- [x] Patched SP804 QEMU + virtio blk/net in one smoke test
- [x] CI matrix job `composed` with SP804 QEMU cache
- [x] `scripts/test.py --unordered` for concurrent PD log lines

## Version alignment

| Component | Version |
|-----------|---------|
| seL4 | 15.0.0 |
| Microkit | 2.2.0 |
| rust-sel4 | v4.0.0 |