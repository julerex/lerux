# PLAN.md — lerux roadmap

Last updated: 2026-06-30 (Phase 24)

## Phase 1 — Bring-up

- [x] Pivot from Redox kernel to seL4 + Microkit
- [x] `just fetch` / `just build-sdk` / `just run` for aarch64 virt
- [x] Single hello protection domain
- [x] CI smoke test in Docker

## Phase 2 — Multi-PD IPC

- [x] Serial driver PD (PL011 on virt) — `userspace/pds/serial-driver/`
- [x] Two-PD system with Microkit channels (hello client + serial_driver)
- [x] Board-templated `.system` files — `userspace/systems/templates/` + `lerux system`

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
- [x] Patched QEMU for virt SP804 (`lerux install sp804-qemu`; rust-sel4 `arm-virt-sp804` patch)
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
| Virtio blk/net | yes | yes | yes |
| Init RTC+timer | yes | no | no |
| Composed init+virtio | yes | no | no |
| HTTP over virtio-net | yes | yes | yes |
| Block IPC service | yes | yes | yes |

Init (`just test-init`) uses PL031 + SP804 drivers from rust-sel4 v4.0.0, which target QEMU aarch64 virt MMIO only. RISC-V virt and x86 PC do not expose those devices in stock QEMU, and there are no equivalent rust-sel4 driver crates yet.

## Phase 15 — Composed aarch64 system

- [x] `composed.system.template`: boot-init + hello + init drivers + virtio drivers (7 PDs)
- [x] Board `qemu_virt_aarch64_composed` (`just test-composed`)
- [x] `boot-init` uses serial IPC for RTC/timer; `hello` uses virtio with debug-print (serial-driver is single-client)
- [x] Patched SP804 QEMU + virtio blk/net in one smoke test
- [x] CI matrix job `composed` with SP804 QEMU cache
- [x] boot-init notifies hello before virtio probe (avoids serial/debug interleaving)

## Phase 16 — Docs and CI hardening

- [x] [`docs/boards.md`](boards.md) — board/QEMU reference table
- [x] [`docs/ci.md`](ci.md) — pipeline, smoke matrix, caches, troubleshooting
- [x] README trimmed; links to detailed docs
- [x] SP804 QEMU built once in **sdk** job (init/composed smoke only restore)
- [x] QEMU cache includes install prefix + source tree + tarball
- [x] Cache save on smoke failure (`if: always()` for `build/`)
- [x] Workflow concurrency (cancel stale runs); `permissions: contents: read`
- [x] Smoke jobs verify SP804 QEMU binary is executable after cache restore

## Phase 17 — HTTP over virtio-net

- [x] `http-server` PD: smoltcp TCP listen on `:8080`, `GET /` → `200 OK` body `lerux: HTTP ok`
- [x] `http-virtio.system.template` — serial + virtio-net + http-server (no blk)
- [x] Board `qemu_virt_aarch64_http` (`just test-http`); host `curl` via QEMU `hostfwd` `18080→8080`
- [x] `http-composed.system.template` — boot-init + init drivers + virtio-net + http-server (6 PDs)
- [x] Board `qemu_virt_aarch64_http_composed` (`just test-http-composed`); boot-init notify gate before net
- [x] `lerux test` HTTP smoke checks (`--curl URL EXPECT` via `lerux smoke`)
- [x] CI matrix jobs `http` and `http-composed` (12 smoke jobs total)

## Phase 18 — x86 PCI virtio

- [x] `virtio-pci-driver` PD — combined virtio-blk + virtio-net over PCI ECAM on q35
- [x] `lerux-virtio-hal` + `lerux-virtio-pci` crates (ECAM, I/O ports, shared HAL)
- [x] `virtio-hello-x86.system.template` — hello + serial + virtio-pci-driver
- [x] Board `x86_64_generic_virtio` (`just test-x86-virtio`); blk MBR read + net TX + TCP RX (host echo on 18080)

## Phase 19 — x86 HTTP inbound (hostfwd)

- [x] `x86_64_generic_http` uses `virtio-pci-driver` (net-only) instead of standalone `virtio-net-driver`
- [x] `http-virtio-x86.system.template` — virtio PCI driver IRQ channel 3, shared ring layout with virtio hello
- [x] `http-server` net poll: drain device ring in a loop, UDP TX priming, listen on all guest addresses
- [x] `just test-x86-http` — serial `lerux-http: listening` then host `curl` via `hostfwd` `18080→8080`
- [x] CI matrix job `x86-http` (13 smoke jobs total)

## Phase 20 — CI and docs hygiene

- [x] CI matrix job `x86-virtio` (`just disk-img && just test-x86-virtio`; 14 smoke jobs total)
- [x] Document Phase 18 in plan; fix cross-arch virtio parity table (x86 → yes)
- [x] Sync smoke job counts across README, ci.md, and context.md

## Phase 21 — x86 HTTP notification fix

- [x] Remove `wait_for_inbound` init-time polling workaround in `http-server` (virtio-pci IRQ channel 3 + ring notify path is reliable post-init)
- [x] Simplify `drive_net` to one notification-driven poll round (+ post-serve flush)
- [x] Update x86 HTTP operational docs (guest returns from `init()` after `listening`; inbound via driver notifications)

## Phase 22 — RISC-V HTTP over virtio-net

- [x] `http-virtio-riscv.system.template` — serial + virtio-net + http-server (MMIO bus.1 at `0x10_002_000`)
- [x] Board `qemu_virt_riscv64_http` (`just test-riscv-http`); host `curl` via QEMU `hostfwd` `18080→8080`
- [x] `riscv64_http` QEMU profile in `lerux-cli` (net-only, no `disk.img`)
- [x] HTTP port cleanup covers aarch64, RISC-V, and x86 QEMU hostfwd on 18080
- [x] CI matrix job `riscv-http` (15 smoke jobs total)
- [x] Cross-arch HTTP parity table updated (RISC-V → yes)

## Phase 23 — Block service over IPC

- [x] `BlockRequest` / `BlockResponse` in `lerux-interface-types` (Poll-based async RPC)
- [x] `blk-server` PD — virtio ring-buffer client + postcard RPC server
- [x] `blk-client` PD — reads LBA 0, logs MBR signature
- [x] `blk.system.template` / `blk-riscv.system.template` / `blk-x86.system.template`
- [x] Boards `qemu_virt_aarch64_blk`, `qemu_virt_riscv64_blk`, `x86_64_generic_blk`
- [x] `virtio-pci-driver` blk-only board feature for x86
- [x] `just test-blk` / `just test-riscv-blk` / `just test-x86-blk`
- [x] CI matrix jobs `blk`, `riscv-blk`, `x86-blk` (18 smoke jobs total)

## Phase 24 — Multi-client serial driver

- [x] Lerux-owned serial `HandlerImpl` with `multi-client-2` feature (two IPC clients on composed boards)
- [x] Second serial channel in `composed.system.template` and `http-composed.system.template`
- [x] `hello` / `http-server` use `serial-ipc` on composed boards (no debug-print workaround)
- [x] Channel renumbering when `composed-sync` + `serial-ipc` are both enabled
- [x] Echo boards: `echo-server` on serial IPC (multi-client-2; serial driver priority 4 on echo layouts)

## Phase 25 — Composed block service

- [x] `blk-composed.system.template`: boot-init + init drivers + blk-server/client + virtio-blk
- [x] Board `qemu_virt_aarch64_blk_composed` (`just test-blk-composed`)
- [x] `blk-client` composed-sync: probe block after boot-init notify
- [x] CI matrix job `blk-composed` (19 smoke jobs total)

## Version alignment

| Component | Version |
|-----------|---------|
| seL4 | 15.0.0 |
| Microkit | 2.2.0 |
| rust-sel4 | v4.0.0 |