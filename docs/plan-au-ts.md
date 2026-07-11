# PLAN — au-ts inspiration

Last updated: 2026-07-11

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
| Programmatic SDF | `microkit_sdf_gen` | `.system` templates, `support/profiles/`, `lerux profile` |
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

## Phase 41 — System generation (sdfgen-shaped)

**Goal:** Profiles and channel manifests drive composition with fewer hand-edited XML edges; validate against Microkit SDF rules before `microkit` runs.

### Inspiration

`microkit_sdf_gen`: programmatic PDs, memory regions, channels, and subsystem recipes (Python/C/Zig).

### Scope

- [ ] Inventory current templates: which regions/channels are mechanical vs board-specific
- [ ] ADR: extend `lerux profile` / `lerux system` in-tree **vs** call out to sdfgen Python
  - Default lean: in-tree Rust generator in `lerux-cli` (matches host tooling rules); optional later bridge to sdfgen for sDDF-compatible layouts
- [ ] Generate channel IDs and named `Channel` constants from a single manifest (profile TOML or sibling YAML) so PD `const` values cannot drift from XML
- [ ] Emit `.system` from profile + board `system_vars`; keep templates only for irreducible board MMIO/IRQ snippets if needed
- [ ] `lerux profile diff` shows generated SDF delta, not only TOML
- [ ] Smoke: `lerux profile build workstation` bit-identical (or documented-equivalent) to today’s hand template path for one golden board

### Out of scope

- Full sDDF subsystem recipes in C
- Replacing `support/boards.toml` hardware constants

### Exit

Workstation (QEMU aarch64) still boots via generated SDF; CI smoke green; AGENTS.md notes “channels come from manifest.”

---

## Phase 42 — Serial virtualiser (sDDF serial shape)

**Goal:** Least-privilege serial mux: UART driver owns MMIO/IRQ only; a virtualiser multiplexes clients over shared queues.

### Inspiration

sDDF serial: driver ↔ Tx/Rx virtualisers ↔ clients; SPSC queues; power-of-two capacity; `producer_signalled` protocol ([`sddf/docs/serial/serial.md`](https://github.com/au-ts/sddf/blob/main/docs/serial/serial.md)).

### Scope

- [ ] Document current `serial-driver` multi-client-2/3 model vs sDDF split (ADR or short design note under `docs/`)
- [ ] Introduce Rust queue crate or module (`lerux-sddf-serial` or under `lerux-ipc`) matching SPSC + separate queue/data regions — **or** confirm rust-sel4 already covers enough and wrap it
- [ ] Split PDs (names TBD): `serial-driver` (device only) + `serial-virt` (mux) + existing clients unchanged at RPC boundary where possible
- [ ] Map log-server / shell / supervisor onto virt queues; preserve multi-client workstation behaviour
- [ ] Smoke: workstation serial REPL + `dmesg` unchanged in expects

### Out of scope

- Porting C sDDF serial components
- Changing postcard `LogRequest` / shell line protocol (transport may change under the hood)

### Exit

Driver PD has no client data regions mapped; virt owns mux; smokes pass.

---

## Phase 43 — Net virtualiser (sDDF net shape)

**Goal:** Multi-client ethernet with clear trust boundaries: NIC driver without client DMA; Rx/Tx virtualisers; optional per-client copy for untrusted clients.

### Inspiration

sDDF network architecture ([`sddf/docs/network/network.md`](https://github.com/au-ts/sddf/blob/main/docs/network/network.md)): driver / Rx virt / Tx virt / copy PDs; shared queue metadata + DMA vs client data regions.

### Scope

- [ ] Design note: map today’s `virtio-net-driver` | `genet-driver` → `net-server` (smoltcp) → apps onto sDDF roles
- [ ] First vertical slice on QEMU virtio-net only (one trusted client = `net-server`)
  - Driver lacks client DMA maps; virt owns buffer handoff
- [ ] Second client path: e.g. `http-server` or `fetch-client` as separate net client **or** stay behind `net-server` RPC (prefer RPC for untrusted apps; virt for stack/driver boundary)
- [ ] Preserve `NetRequest` / `NetResponse` as the app-facing API
- [ ] Smoke: `just test-fetch`, `just test-http`, net-composed boards
- [ ] Stretch: genet path on RPi4 after QEMU virt is stable

### Out of scope

- Full sDDF copy-PD swarm unless a second untrusted L2 client appears
- Replacing smoltcp with lwIP

### Exit

Documented trust map; at least one board where NIC driver address space excludes client DMA; fetch/HTTP smokes green.

---

## Phase 44 — Filesystem backends (LionsOS menu)

**Goal:** Real on-disk / network FS options behind existing `FsRequest` / `FsResponse`, without POSIX.

### Inspiration

LionsOS `components/fs/fat`, `components/fs/nfs`, `examples/fileio`.

### Scope

- [ ] Keep `LERUXFS1` as the default smoke FS
- [ ] FAT backend behind `fs-server` (read + write subset matching current IPC ops) on virtio-blk / emmc2
- [ ] Optional NFS client PD or `fs-server` backend for QEMU user-net (libnfs inspiration only — reimplement or carefully wrap; stay `no_std` or isolate host-incompatible bits)
- [ ] Profile fragments: `fs-lerux`, `fs-fat` (and later `fs-nfs`)
- [ ] Shell / edit unchanged at IPC boundary
- [ ] Smoke: FAT round-trip board; document format choice in `docs/context.md`

### Out of scope

- Mounting Linux rootfs or glibc apps
- Full POSIX VFS

### Exit

One alternate FS backend selectable by profile; workstation can boot with FAT for demo if desired.

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
- At least one of: generated SDF (41), serial virt (42), or net virt (43) ships before optional 48–49
- HW bring-up (47) closes the Phase 39 manual gate for workstation-rpi4
