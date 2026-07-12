# PLAN.md — lerux roadmap

Last updated: 2026-07-12 (Phase 48 workstation QoS; Phase 49 in plan-au-ts)

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
| Net IPC service | yes | yes | yes |

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

## Phase 26 — Block write over IPC

- [x] `BlockRequest::WriteSector` and `BlockResponse::Ok` in `lerux-interface-types`
- [x] `blk-server` issues virtio write operations; `blk-client` verifies sector-1 round-trip
- [x] QEMU blk profiles mount `disk.img` read-write (`read-only=off`) for write smoke coverage
- [x] Smoke expects `lerux-blk: write round-trip ok` on all blk boards (aarch64, RISC-V, x86, composed)

## Phase 27 — Net service over IPC

- [x] `NetRequest` / `NetResponse` in `lerux-interface-types` (UdpTx + Poll async RPC)
- [x] `net-server` PD — virtio-net ring-buffer client + postcard RPC server
- [x] `net-client` PD — UDP TX to QEMU user netdev (`10.0.2.2`) via IPC
- [x] `net.system.template` — serial + virtio-net + net-server/client
- [x] Board `qemu_virt_aarch64_net` (`just test-net`)
- [x] CI matrix job `net` (20 smoke jobs total)

## Phase 28 — Cross-arch net IPC

- [x] `net-riscv.system.template` / `net-x86.system.template`
- [x] Boards `qemu_virt_riscv64_net`, `x86_64_generic_net`
- [x] `virtio-pci-driver` net-only board feature for x86
- [x] `just test-riscv-net` / `just test-x86-net`
- [x] CI matrix jobs `riscv-net`, `x86-net` (22 smoke jobs total)

## Phase 29 — Composed net service

- [x] `net-composed.system.template`: boot-init + init drivers + net-server/client + virtio-net
- [x] Board `qemu_virt_aarch64_net_composed` (`just test-net-composed`)
- [x] `net-client` composed-sync: probe net after boot-init notify
- [x] CI matrix job `net-composed` (23 smoke jobs total)

## Phase 30 — Grand composed IPC

- [x] `ipc-composed.system.template`: boot-init + init drivers + blk-server/client + net-server/client + virtio-blk/net (10 PDs)
- [x] Board `qemu_virt_aarch64_ipc_composed` (`just test-ipc-composed`)
- [x] Serial driver `multi-client-3` (boot-init, blk-client, net-client)
- [x] Notify chain: boot-init → blk-client → net-client (sequential probes)
- [x] CI matrix job `ipc-composed` (24 smoke jobs total)

## Phases 31–38 — Non-POSIX workstation (QEMU MVP done)

Roadmap toward a minimal “Arch-like” workflow (profiles, init, shell, FS, network) **without** POSIX, glibc, or unmodified Linux binaries. A “package” is a PD crate pinned in a **system profile**; installing means reassembling `loader.img`, not `fork`/`exec`.

Tracer-bullet order: FS (32) → TCP/fetch (31) → shell (34) → supervisor (33) → profiles (35) → ops (36) → HW slice (37) → ported apps (38).

### Cross-stack smoke parity

| Capability | QEMU virt | Real HW (RPi4) |
|------------|-----------|----------------|
| Serial hello | yes | yes (`rpi4b_4gb`, `hardware-rpi4` profile) |
| Block IPC | yes | yes (`rpi4b_4gb_blk`, `emmc2-driver`; workstation uses same driver) |
| Net IPC (UDP TX) | yes | yes (`rpi4b_4gb_net`, `genet-driver`; workstation uses same driver) |
| Net TCP + DNS | yes | no (workstation `fetch` is UDP demo only) |
| Filesystem IPC | yes | yes (`rpi4b_4gb_workstation`; manual gate pending on-device REPL) |
| Interactive shell | yes | yes (`rpi4b_4gb_workstation`; manual gate pending) |
| Logging / config | yes | yes (workstation profile) |
| Edit TUI | yes | yes (workstation profile; manual gate pending) |
| Profile-based build | yes | yes (`workstation-rpi4`, `hardware-rpi4`, hello/net/blk slices) |

## Phase 31 — Net service v2 (TCP + DNS)

- [x] Extend `NetRequest` / `NetResponse`: `TcpConnect`, `TcpSend`/`TcpRecv`, `DnsResolve` (poll-based async RPC)
- [x] Extend `net-server` with smoltcp TCP client + static DNS (`host` → `10.0.2.2` for QEMU smoke)
- [x] `fetch-client` PD — HTTP GET over net IPC
- [x] Board `qemu_virt_aarch64_fetch` (`just test-fetch`); smoke expects `lerux-fetch: 200`
- [x] Host `lerux http-one 8081` helper for fetch smoke; CI matrix job `fetch` (25 smoke jobs total)

## Phase 32 — Filesystem server

- [x] `FsRequest` / `FsResponse` in `lerux-interface-types` (`Open`, `Read`, `Write`, `ListDir`, `Stat`, `Create`, `Poll`)
- [x] `lerux-fs` crate + `fs-server` PD — virtio-blk client + `LERUXFS1` on-disk format (LBAs 1+)
- [x] `fs-client` PD — write/read round-trip smoke
- [x] Board `qemu_virt_aarch64_fs` (`just test-fs`); smoke expects `lerux-fs: round-trip ok`
- [x] CI matrix job `fs` (26 smoke jobs total)

## Phase 33 — Supervisor + service graph

- [x] Evolve `boot-init` → `supervisor` PD (rename + workspace): RTC/timer + generalized init
- [x] `SupervisorRequest` / `SupervisorResponse` (Reboot, ListServices, ServiceStatus, GetTime)
- [x] `workstation.system.template`: supervisor + fs-server + net-server + drivers (with IPC channels)
- [x] Board `qemu_virt_aarch64_workstation` (`just test-workstation`); smoke expects `lerux-supervisor: ready`
- [x] Supervisor exercises FS mount + net up in init; serves IPC to shell
- [x] Generalize notify / multi-client serial preserved

## Phase 34 — Shell and core utilities

- [x] `shell` (lerux-shell) PD: full serial REPL (read/echo/line buffer via serial IPC client)
- [x] Commands: ls, cat <path>, write <path> <data>, time, ps, reboot, fetch (demo via net), echo, help
- [x] Shell calls fs-server/net-server/supervisor via typed IPC for commands
- [x] Included in workstation profile (template + board pds list + serial/fs/net/sup wiring)
- [x] Smoke test covers shell ready + service calls + prompt (scripted REPL testable via serial)
- [x] “add a ported app” checklist documented in `docs/context.md`

## Phase 35 — System profiles and packages

- [x] `support/profiles/*.toml` — named PD sets + template + channel manifest (e.g. `minimal`, `server`, `workstation`)
- [x] `lerux profile list|build|diff` in `lerux-cli`
- [x] Profile `workstation` (and others) bootable via `lerux profile build <name>` (reproduces loader.img from the pinned source tree state)
- [x] A package = one PD crate + interface-types version + optional profile fragment; publish = CI ELF artifact + pin → Phase 40

## Phase 36 — Logging, config, and ops

- [x] `log-server` PD — multiplex serial + ring buffer; `LogRequest::Append` / `Subscribe` / `GetRecent`
- [x] `config-server` PD (FS-backed under /config/ for net and boot keys; baseline implementation using existing fs-server)
- [x] Shell `dmesg` via log IPC; supervisor persists boot log to FS (`/boot.log`)
- [x] Updated workstation profile + serial multi-client-3 + logging sinks (server feature) + priority wiring for PPCs
- [x] Smoke + `just check` + `just check-pd` pass

## Phase 37 — Real hardware slice

- [x] One target board (`rpi4b_4gb`): serial console via PL011; `BOARD=rpi4b_4gb just image`
- [x] `hardware-rpi4.toml` profile (basic hello slice); full storage+net use native drivers (virtio dropped when native exist)
- [x] Smoke subset on hardware / manual gate: `BOARD=rpi4b_4gb just test` (and `lerux test`) now does full image build + prints manual verification guidance. `run` also supported for image+instructions. (No auto QEMU; real device or future HW CI.)
- [x] Native driver PD skeletons: `genet-driver` (RPi4 bcm2711-genet-v5) and `emmc2-driver` (bcm2711-emmc2); board entries `rpi4b_4gb_net` / `rpi4b_4gb_blk`; templates `net-genet-rpi.system.template`, `blk-emmc-rpi.system.template`. Stubs allow init + basic TX/block read completion for manual smoke parity. Full register/DMA/MDIO/ADMA impl → Phase 39.

## Phase 38 — Optional TUI apps (ported only)

- [x] Cherry-pick `edit` TUI app with clear IPC boundary (`EditRequest`/`EditResponse` via `lerux-interface-types`)
- [x] `edit` PD: FS load/save + editing state machine; shell proxies keys (Ctrl-S save, Ctrl-Q quit, basic insert/bs/enter/arrows) and renders view
- [x] Wired into `workstation` profile + system template + smoke

### MVP done (Phases 31–38 on QEMU)

- [x] Profile `workstation` boots supervisor + FS + net + shell + log + config + edit
- [x] Shell lists/reads/writes files, `fetch`es a URL, `dmesg`, and launches `edit`
- [x] Host `lerux profile build workstation` (and list/diff) reproduces `loader.img` from the current pinned tree
- [x] CI smoke covers workstation (incl. edit PD init) + `edit` command path (basic)

## Phase 39 — RPi4 workstation (complete)

Bring the QEMU workstation stack to real hardware on `rpi4b_4gb`.

- [x] `workstation-rpi4` profile + `workstation-rpi4.system.template` + `rpi4b_4gb_workstation` board
- [x] `emmc2-driver`: SDHCI PIO block read/write + virtio_blk ring IPC + `GetBlockDeviceLayout`; clock/CMD41/4-bit bus hardening
- [x] `genet-driver`: Linux GENET v5 register map, ring-16 DMA descriptors, MDIO/INTRL2/IRQ enable
- [ ] Manual HW gate: serial REPL, `ls`/`cat`/`fetch`/`edit` on device — procedure in [docs/boards.md](boards.md#rpi4-workstation-manual-hw-gate-phase-39); image build verified (`BOARD=rpi4b_4gb_workstation just image`)
- [x] Optional serial-capture HW CI harness: `LERUX_HW_SERIAL=/dev/ttyUSB0 BOARD=rpi4b_4gb_workstation just test`

## Phase 40 — Packages and more apps (complete)

- [x] Phase 35 follow-up: package = PD crate + interface-types version + profile fragment; `lerux package` CLI + `support/packages/` + `support/package-pins.toml`; CI ELF artifact job
- [x] Additional ported apps: `top` (supervisor `ServiceList` IPC), `chat-client` (UDP chat + shell proxy), `http-file-browser` (FS + net TCP listen IPC)
- [x] Workstation profile wires chat + http-fs; smoke expects + hostfwd curl for directory listing

## Phase 41 — System generation ✅

In-tree SDF composition in `lerux-cli` ([ADR-001](decisions/001-in-tree-system-generation.md)):

- Structured `[[channel]]` manifests in profiles; workstation templates are channel-free layout bodies
- `render_system` = board `system_vars` + template layout + generated channels
- `lerux profile sdf|emit-channels|check-channels|diff` (SDF delta included)
- Docs: [`system-generation.md`](system-generation.md), [`plan-au-ts.md`](plan-au-ts.md)

## Phase 42 — Serial virtualiser ✅ (workstation)

`serial-driver` (device-only) + `serial-virt` on workstation profiles ([ADR-002](decisions/002-serial-virtualiser.md)). Other boards keep the combined multi-client driver. Smoke green.

## Phase 43 — Net sDDF topology ✅

Apps on `NetRequest` via `net-server`; aarch64 virtio-net **unified-dma** (no separate client_dma MR in the driver SDF). [ADR-003](decisions/003-net-virtualiser.md), [`net-topology.md`](net-topology.md).

## Phase 44 — FS backends ✅ (FAT slice)

`LERUXFS1` remains default (`just test-fs`). Alternate **FAT16** backend (`lerux-fat`, `backend-fat`, `just test-fs-fat`). NFS and multi-cluster stretch deferred — [`plan-au-ts.md`](plan-au-ts.md).

## Phase 45 — Service async ✅

Stackless coop async for service PDs ([ADR-004](decisions/004-service-async.md), `lerux-service-async`). `fs-server` LERUXFS1 format runs as a `SingleTask` future; clients keep `Poll` RPC.

## Phase 46 — Debug PD ✅

Microkit hierarchy fault parent + crash child (`just test-debug`); QEMU gdbstub docs ([`debug.md`](debug.md), [ADR-005](decisions/005-debug-pd.md)). libgdb deferred (needs forked seL4/Microkit).

## Phase 47 — Hardware CI harness ✅

`support/smoke-expects.toml`, `lerux test --mode hw-serial`, board locks, `just test-hw`, optional self-hosted workflow ([`ci.md`](ci.md)).

## Phase 48 — Workstation QoS ✅

Service-class PD priorities (shell interactive above bulk); [`qos.md`](qos.md), [ADR-006](decisions/006-workstation-qos.md).

## Phases 49+ — au-ts inspiration

See [`plan-au-ts.md`](plan-au-ts.md) for Phase 49 (optional perf baselines).

## Version alignment

| Component | Version |
|-----------|---------|
| seL4 | 15.0.0 |
| Microkit | 2.2.0 |
| rust-sel4 | v4.0.0 |