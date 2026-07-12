# PLAN — au-ts inspiration

Last updated: 2026-07-12 (Phase 43 net topology + ADR-003)

Upstream mirror: [`/home/julian/repos/github_orgs/au-ts`](https://github.com/au-ts) (Trustworthy Systems).  
Related: [`plan.md`](plan.md) (main roadmap), [`context.md`](context.md) (domain language).

This plan turns the highest-leverage ideas from the au-ts ecosystem into lerux work **without** adopting C sDDF/LionsOS as userspace or abandoning the Rust-only, typed postcard RPC, non-POSIX direction.

## Principles

| Keep | Steal the idea, not the code |
|------|------------------------------|
| Rust-only PDs (`#![no_std]`) | sDDF virtualiser topology and queue protocols |
| Typed postcard RPC (`lerux-interface-types`) | sdfgen-style programmatic system composition |
| Static Microkit images + profiles | LionsOS component *menu* as ported PD candidates |
| No musl / POSIX ABI | libmicrokitco-style sync-over-async where poll loops hurt |
| Upstream via rust-sel4 / Microkit pins | libgdb, systems-ci patterns for debug and HW CI |

**Do not** vendor `deps/` copies of sDDF or LionsOS into the tree. Prefer reimplementing protocols in Rust (or wrapping rust-sel4 crates that already encode the same shapes). Link ADRs when a choice hardens (e.g. “adopt sdfgen vs extend `lerux profile`”).

## Source map

| Inspiration | au-ts repos | lerux touchpoints today |
|-------------|-------------|-------------------------|
| Driver / virtualiser / client | `sddf` (+ design PDF) | `serial-driver`, virtio/genet/emmc2, `net-server`, `blk-server` |
| Programmatic SDF | `microkit_sdf_gen` | `.system` templates (layout body), `support/profiles/`, `lerux profile` / `render_system` |
| Component catalog | `lionsos` examples + `components/` | workstation profile, Phase 40 apps |
| Sync over Microkit events | `libmicrokitco` | poll-based `FsRequest` / `NetRequest` |
| Remote GDB | `libgdb` | none |
| HW CI / board lock | `systems-ci`, `machine_queue` | `LERUX_HW_SERIAL=… just test` |
| Partitioned scheduling | `arinc-scheduling` | `supervisor` notify / priorities |
| Perf baselines | `sel4bench`, `ipbench` | smoke-only (functional) |

## Tracer-bullet order

Dependency-aware order after Phase 40 packaging:

```
41 system-gen  →  42 serial virt  →  43 net virt
        ↓                                    ↓
   44 FS backends ←—————— 45 sync runtime
        ↓
   46 debug PD  ·  47 HW CI  ·  48 QoS (optional)
        ↓
   49 perf baselines (optional)
```

Phases 41–43 strengthen the static system and I/O mux story.  
Phases 44–45 grow services without POSIX.  
Phases 46–47 harden bring-up on real boards.  
48–49 are optional once mux and CI exist.

---

## Phase 41 — System generation (sdfgen-shaped) ✅ complete

**Goal:** Profiles and channel manifests drive composition with fewer hand-edited XML edges; validate against Microkit SDF rules before `microkit` runs.

### Inspiration

`microkit_sdf_gen`: programmatic PDs, memory regions, channels, and subsystem recipes (Python/C/Zig).

### Scope

- [x] Inventory current templates: which regions/channels are mechanical vs board-specific → [`system-generation.md`](system-generation.md)
- [x] ADR: extend `lerux profile` / `lerux system` in-tree **vs** call out to sdfgen Python → [ADR-001](decisions/001-in-tree-system-generation.md)
  - **Accepted:** in-tree Rust generator in `lerux-cli`; optional later bridge to sdfgen for sDDF-compatible layouts only
- [x] Structured channel manifest (`[[channel]]`) + load-time validation (`lerux profile validate|show`)
- [x] Channel XML composition: `render_system` / `render_profile_system` = layout body (`system_vars` + template) + generated channels from profile
- [x] Workstation + workstation-rpi4 templates are **channel-free layout recipes**; channels live only in profile TOML
- [x] Named channel constants: `lerux profile emit-channels` + `check-channels` (PD `const` drift check); `channel_consts.rs` written next to `system.system`
- [x] Full SDF emit path: `lerux profile sdf <name>` / `lerux system --board …` compose complete Microkit XML (layout body from template + channels from manifest; hardware from `boards.toml`)
- [x] `lerux profile diff` shows PD/channel TOML delta **and** composed SDF delta
- [x] Golden unit tests (17) for workstation composition + channel checks; `just check` clean

### Out of scope (deferred)

- Full sDDF subsystem recipes in C
- Replacing all layout templates with pure device-recipe IR (MRs/maps still in `.system.template` bodies; Phase 42+ can shrink further)
- Replacing `support/boards.toml` hardware constants

### Exit

- [x] Workstation (QEMU aarch64) SDF composed from profile + board (channels generated)
- [x] Host checks green (`just check`, `cargo test -p lerux-cli`)
- [x] AGENTS.md notes channels come from the profile manifest

### CLI surface (Phase 41)

| Command | Purpose |
|---------|---------|
| `lerux profile list\|show\|validate` | Inspect structured channels |
| `lerux profile sdf <name> [-o file]` | Emit composed Microkit SDF |
| `lerux profile emit-channels <name>` | Generate `channel_consts.rs` text |
| `lerux profile check-channels [name]` | Drift-check PD `Channel::new` vs manifest |
| `lerux profile diff a b` | TOML topology + SDF delta |
| `lerux system --board … -o …` | Compose + write SDF (+ channel_consts side file) |

---

## Phase 42 — Serial virtualiser (sDDF serial shape) ✅ complete (workstation slice)

**Goal:** Least-privilege serial mux: UART driver owns MMIO/IRQ only; a virtualiser multiplexes clients over shared queues.

### Inspiration

sDDF serial: driver ↔ Tx/Rx virtualisers ↔ clients; SPSC queues; power-of-two capacity; `producer_signalled` protocol ([`sddf/docs/serial/serial.md`](https://github.com/au-ts/sddf/blob/main/docs/serial/serial.md)).

### Scope

- [x] Document current multi-client model vs sDDF split → [ADR-002](decisions/002-serial-virtualiser.md)
- [x] `lerux-serial-queue` SPSC + data region + `producer_signalled` (host unit tests)
- [x] Split on workstation: `serial-driver` `device-only` + `serial-virt` multi-client postcard RPC (clients unchanged wire format)
- [x] Shell / supervisor / log-server → `serial_virt`; driver ↔ virt notify + shared TX/RX queues
- [x] Non-workstation boards keep combined multi-client driver (migration later)
- [ ] Full QEMU workstation smoke regression (run in CI / local `just test-workstation` when convenient)

### Out of scope

- Porting C sDDF serial components
- Changing postcard `LogRequest` / shell line protocol
- Per-client shared queues / separate TX+RX virt PDs (future)

### Exit

- [x] Driver PD has no client PPCs (sole client = `serial-virt`); UART MMIO only
- [x] Virt owns multi-client mux; workstation profile/template/board updated
- [x] Workstation smoke green (`just test-workstation`, CI fix `cb3ef88`)

---

## Phase 43 — Net virtualiser (sDDF net shape) ✅

**Goal:** Multi-client ethernet with clear trust boundaries: NIC driver without client DMA; Rx/Tx virtualisers; optional per-client copy for untrusted clients.

### Inspiration

sDDF network architecture ([`sddf/docs/network/network.md`](https://github.com/au-ts/sddf/blob/main/docs/network/network.md)): driver / Rx virt / Tx virt / copy PDs; shared queue metadata + DMA vs client data regions.

### Scope

- [x] Design note → [ADR-003](decisions/003-net-virtualiser.md), [`net-topology.md`](net-topology.md)
- [x] Apps stay behind `NetRequest` / `NetResponse`; sole L2 Microkit client of the NIC driver is `net-server`
- [x] **Unified DMA (aarch64 virtio-net):** remove separate `virtio_net_client_dma` MR; driver maps only `virtio_net_driver_dma` (Hal low half + bounce high half); stack maps same MR for bounce; feature `unified-dma`
- [x] Prefer RPC for untrusted apps; no app L2 DMA
- [x] Preserve `NetRequest` / `NetResponse`
- [x] Smoke: `just test-net`, `just test-fetch`, `just test-http` (and workstation)
- [ ] Stretch: genet / x86 unified-dma; separate Rx/Tx virt PDs / copy PDs

### Out of scope

- Full sDDF copy-PD swarm
- Replacing smoltcp with lwIP

### Exit

- [x] Documented trust map
- [x] Aarch64 virtio-net boards: no distinct client_dma map in the NIC driver PD
- [x] Apps never map NIC DMA; net/fetch/HTTP smokes green

---

## Phase 44 — Filesystem backends (LionsOS menu) ✅ FAT slice

**Goal:** Real on-disk / network FS options behind existing `FsRequest` / `FsResponse`, without POSIX.

### Inspiration

LionsOS `components/fs/fat`, `components/fs/nfs`, `examples/fileio`.

### Scope

- [x] Keep `LERUXFS1` as the default smoke FS (`just test-fs`)
- [x] FAT16 backend behind `fs-server` (`lerux-fat` + `backend-fat`) matching `Open`/`Create`/`Read`/`Write`/`Stat`/`ListDir`/`Poll` on virtio-blk
- [ ] Optional NFS client PD or `fs-server` backend for QEMU user-net (deferred)
- [x] Board/feature selection: `qemu_virt_aarch64_fs` (LERUXFS1) vs `qemu_virt_aarch64_fs_fat` (FAT16); Cargo features `backend-lerux` / `backend-fat`
- [x] Shell / edit unchanged at IPC boundary
- [x] Smoke: `just test-fs-fat`; format choice in `docs/context.md`

### Out of scope

- Mounting Linux rootfs or glibc apps
- Full POSIX VFS
- Multi-cluster files / LFN / subdirectories (v1 FAT is root + single cluster)

### Exit

- [x] One alternate FS backend selectable by board/feature; `just test-fs` and `just test-fs-fat` green
- [ ] Workstation optional FAT demo (stretch)

---

## Phase 45 — Sync runtime for service PDs

**Goal:** Replace the worst poll loops in `fs-server` / `net-server` with structured concurrency that still fits Microkit’s `Handler` model.

### Inspiration

`libmicrokitco` — coroutines / sync API over async notifications.

### Scope

- [ ] Spike: evaluate rust-sel4 async / local executor vs a thin `lerux-microkitco`-style coop scheduler inside one PD
- [ ] ADR: coop tasks in-server **vs** keep poll RPC only
- [ ] If adopted: refactor one server (`fs-server` or `net-server`) to await ring completions without busy `Poll` from clients regressing
- [ ] Clients may keep poll-based RPC; server-internal only is enough for v1
- [ ] `just check-pd` + affected smokes

### Out of scope

- Preemptive threads in every PD
- Porting libmicrokitco C into the tree

### Exit

ADR accepted; at least one service uses the chosen model; no smoke regression.

---

## Phase 46 — Debug protection domain

**Goal:** Attach GDB to a crashing or hung PD on QEMU (then RPi4).

### Inspiration

`libgdb` (debugger PD as fault handler; serial or TCP transport).

### Scope

- [ ] Feasibility: upstream Microkit/seL4 patches libgdb needs vs what 2.2.0 + rust-sel4 v4.0.0 expose
- [ ] Minimal path: fault-handler PD + serial stub that speaks enough GDB RSP for breakpoints on `qemu_virt_aarch64`
- [ ] Profile `debug` or feature flag on workstation that wires fault caps
- [ ] Doc: how to attach `gdb-multiarch` from host
- [ ] Optional: TCP via net-server once Phase 43 exists

### Out of scope

- Shipping a permanent debugger in production workstation images
- x86/RISC-V debug until aarch64 works

### Exit

Documented QEMU workflow: deliberate fault in a demo PD → GDB backtrace.

---

## Phase 47 — Hardware CI harness

**Goal:** Reliable RPi4 (and future board) serial expect tests without ad-hoc cable folklore.

### Inspiration

`systems-ci` (`ts_ci` TestCase, board locking) and `machine_queue`.

### Scope

- [ ] Formalize `LERUX_HW_SERIAL` path into `lerux-cli test` modes: `qemu` | `hw-serial`
- [ ] Expect scripts as data (board + profile → ordered string matches), shared with QEMU where possible
- [ ] Optional lock file / env for single-writer board access (local first; remote queue later)
- [ ] CI: manual or self-hosted runner job gated on hardware label (document; do not require cloud runners to have Pi)
- [ ] Align docs with [`ci.md`](ci.md)

### Out of scope

- UNSW machine_queue integration
- Full matrix of every smoke on HW

### Exit

`BOARD=rpi4b_4gb_workstation LERUX_HW_SERIAL=… just test` is the documented golden path; Phase 39 manual gate checklist is mostly automated.

---

## Phase 48 — Supervisor QoS / partitioning (optional)

**Goal:** Explicit time or priority isolation between shell, net, and FS when contention matters.

### Inspiration

`arinc-scheduling` sampling/queuing ports; Microkit priorities already in templates.

### Scope

- [ ] Inventory current PD priorities and PPC behaviour in workstation templates
- [ ] Define service classes (interactive / bulk I/O / background) in supervisor or profile
- [ ] Optional: budget or rate-limit IPC for bulk clients
- [ ] Smoke under load (e.g. fetch + edit + dmesg) shows interactive serial remains responsive

### Exit

Documented priority policy; one stress smoke or bench note.

---

## Phase 49 — Performance baselines (optional)

**Goal:** Know whether Rust ring + postcard paths are in the same ballpark as sDDF/LionsOS anecdotes.

### Inspiration

`sel4bench`, `ipbench` / `autobench`.

### Scope

- [ ] Host-driven microbench profile: echo RTT, blk read IOPS, UDP PPS on QEMU
- [ ] Record numbers in `docs/` (table + qemu version / CPU)
- [ ] Optional: compare against a pinned sDDF example build *outside* this repo (script in `tools/`, not vendored tree)

### Exit

Reproducible `just bench` (or `lerux bench`) producing a markdown or JSON summary.

---

## Phase 40 interaction

Keep Phase 40 packaging/apps on the main [`plan.md`](plan.md) track. Prefer:

1. Finish Phase 40 package pin story enough that new virt/FS PDs can be profile fragments.
2. Start Phase 41 in parallel with “more apps” only if generator work unblocks channel/manifest pain.

Suggested app ports once 41–44 land:

| App idea | Depends on | Notes |
|----------|------------|-------|
| HTTP file browser | FS + net IPC | LionsOS webserver *idea*; stay Rust PD |
| MicroPython / WAMR PD | Phase 45 helpful | Optional runtime; IPC to fs/net only |
| `top` / chat-client | Phase 40 | Unchanged |

---

## Explicit non-goals

- Replacing lerux userspace with LionsOS or sDDF C components
- Formal verification of lerux PDs (gordian / Cogent / CakeML) unless product requirements change
- libvmm / guest Linux or Windows (revisit only with a dedicated ADR)
- Nix flakes as the primary developer UX (au-ts uses Nix heavily; lerux stays `just` + `lerux-cli`)

## Version / upstream pins

Track alongside [`plan.md`](plan.md) version table. Revisit when adopting debug or newer Microkit APIs:

| Component | Current | Watch |
|-----------|---------|-------|
| seL4 | 15.0.0 | libgdb fork branches |
| Microkit | 2.2.0 | sdfgen / GDB patches |
| rust-sel4 | v4.0.0 | shared-ring + driver-adapter evolution |

## Success criteria (program-level)

- Inspiration is visible as **Rust PDs + profiles**, not as C submodules
- Workstation QEMU smokes remain green after each phase
- At least one of: generated SDF (41), serial virt (42), or net virt (43) ships before optional 48–49 — **Phase 41 done**
- HW bring-up (47) closes the Phase 39 manual gate for workstation-rpi4
