# CONTEXT.md — lerux domain language

## Project purpose

lerux builds **Rust-only userspace** on the formally verified [seL4](https://sel4.systems/) microkernel. The kernel is upstream seL4 (C/ASM), not lerux code. lerux owns protection domains, system descriptions, and utilities.

## Core concepts

**seL4 microkernel**
: Capability-based kernel providing threads, address spaces, and IPC primitives. Source from [seL4/seL4](https://github.com/seL4/seL4), built via Microkit SDK — not copied into this repo.

**seL4 Microkit**
: Static system framework. Protection domains, memory regions, and IPC channels are declared in `.system` XML files. The `microkit` tool assembles `loader.img` from PD ELFs + kernel.

**Protection domain (PD)**
: An isolated userspace component with its own address space and capabilities. lerux PDs are `#![no_std]` Rust crates using `sel4-microkit`.

**rust-sel4**
: Foundation-maintained Rust bindings and runtimes ([Git dependency](https://github.com/seL4/rust-sel4), tag `v4.0.0`). Not vendored; pinned in `Cargo.toml`.

**No vendoring**
: seL4 and microkit source live in gitignored `deps/workspace/`. Version pins are committed in `deps/versions.toml`.

## Resolved decisions (2026-06 pivot)

| Decision | Choice |
|----------|--------|
| Kernel | Upstream seL4 15.0.0, built from source |
| Userspace model | seL4 Microkit (static PD layout) |
| Userspace language | Rust only (no musllibc/relibc in lerux code) |
| First platform | aarch64 QEMU virt; x86_64 parameterized for follow-up |
| Dependency fetch | `lerux fetch` git clones (pinned tags) |

## Platform parity

Echo IPC and virtio smoke tests run on aarch64, RISC-V virt, and x86 (PCI virtio on q35). Block IPC (read + write) and net IPC (UDP TX) run on all three arches. RTC/timer init runs on all three: aarch64 PL031/SP804, RISC-V Goldfish RTC + `rdtime`, x86 CMOS RTC + TSC (Phase 56).

The composed board (`qemu_virt_aarch64_composed`) runs `supervisor` (historically `boot-init`) and `hello`+virtio in one system. Both PDs log via serial IPC (multi-client serial driver). `supervisor` notifies `hello` when init is complete before virtio starts (composed-sync). See [`boards.md`](boards.md) and [`plan.md`](plan.md).

## Non-POSIX direction

lerux does **not** target a Linux or POSIX syscall ABI. Apps are Rust protection domains that speak **typed postcard RPC** (`lerux-interface-types`) over Microkit channels — not file descriptors, `errno`, or `fork`/`exec`.

“Arch-like” means **workflow**, not binary compatibility: rolling PD artifact pins, named system profiles, init ordering, shell + core utilities — each implemented as PDs you port deliberately. Unmodified Arch packages (`bash`, `pacman`, `firefox`, etc.) are out of scope. Gap plan for Arch-level capability (phases 50–60): [`plan-arch.md`](plan-arch.md).

## System profiles and packages

**System profile**
: A named bundle in `support/profiles/*.toml`: which PD crates, `.system` template, and channel manifest compose one `loader.img`. Use `lerux profile list|build|diff <name>`. Board hardware (MMIO, IRQs, arch) remains in `support/boards.toml`; profiles are selected via default_board or `--board`. (Phase 35)

**System generation (Phase 41)**
: Composed Microkit SDF = board `system_vars` + layout template (MRs/PDs/maps) + **generated channels** from the profile’s structured `[[channel]]` list ([ADR-001](decisions/001-in-tree-system-generation.md)). Workstation templates are channel-free; `lerux profile check-channels` guards PD `Channel::new` drift. Details: [`system-generation.md`](system-generation.md).

**Serial virtualiser (Phase 42)**
: On workstation, UART is owned by `serial-driver` (`device-only`); multi-client postcard RPC is served by `serial-virt` (PPC to the driver). Apps still use `SerialClient` / `SERIAL_DRIVER` channel consts (peer is `serial_virt`). See [ADR-002](decisions/002-serial-virtualiser.md).

**Network topology (Phase 43 / 51)**
: Untrusted apps use only `NetRequest` / `NetResponse` against `net-server`. On aarch64 virtio-net, **unified-dma** removes the separate client_dma MR: Hal + bounce share `virtio_net_driver_dma`; the stack maps the bounce half only. Apps never map net DMA. See [ADR-003](decisions/003-net-virtualiser.md), [`net-topology.md`](net-topology.md). Phase 51: **DHCP** (with static fallback), **real DNS** (static `host`/`dns` aliases for smokes), dual TCP (client + listen), and `GetIface` / shell `ip`.

**Filesystem backends (Phase 44 / 50)**
: `fs-server` serves `FsRequest` / `FsResponse` over virtio-blk (or emmc2). Default on-disk format is **LERUXFS2** (`lerux-fs`): hierarchical directories, free-map allocation, multi-sector contiguous files (up to 16 KiB). IPC includes `Mkdir` / `Unlink` / `Rename` and path-scoped `ListDir`. Path grammar: `/`-separated components, optional leading `/`, max 48-byte paths (see `lerux-interface-types`). Alternate **FAT16** backend (`lerux-fat`, feature `backend-fat`) stays root-only, 8.3, single-cluster; hierarchy ops return `Error`. Select via board feature (`qemu_virt_aarch64_fs` vs `qemu_virt_aarch64_fs_fat`). Shell/edit/config stay on the same IPC.

**Service async (Phase 45)**
: Service PDs keep Microkit `Handler` as the root event loop. Long sequential device I/O may use **stackless cooperative async** (`lerux-service-async`: `SingleTask`, `poll_fn`, `WakeCell`) instead of only explicit step machines. Clients still use postcard `Poll` RPC. See [ADR-004](decisions/004-service-async.md).

**Debug / faults (Phase 46)**
: Child PD faults can be delivered to a **parent** PD (`Handler::fault`) via Microkit hierarchy. Demo: `debug-handler` + `crash-demo` (`just test-debug`). Interactive host debugging uses QEMU’s gdbstub + `gdb-multiarch` ([`debug.md`](debug.md), [ADR-005](decisions/005-debug-pd.md)).

**Security posture (Phase 60)**
: Trust map and threat model in [`security.md`](security.md). Automated isolation smoke (`just test-isolation`): untrusted `crash-demo` faults, then `fs-client` still completes an FS round-trip against `fs-server`. Production workstation stays without a debug parent (ADR-005).

**Hardware serial smoke (Phase 47 / 52)**
: On-device boot checks use `LERUX_HW_SERIAL` + `--mode hw-serial` / `just test-hw`, with expects from `support/smoke-expects.toml` and a local board lock. Phase 52 adds optional **scripted REPL** steps (`script = [{send, expect}, …]`) after boot match, and `lerux deploy` / `just deploy-rpi4` for SD boot media. First-boot seeds `/config` via supervisor. Cloud CI stays QEMU-only; optional self-hosted workflow is documented in [`ci.md`](ci.md). Install path: [`boards.md`](boards.md#rpi4-workstation-install-path-phase-52).

**Workstation QoS (Phase 48)**
: Fixed Microkit PD priorities form service classes (platform / services / control / bulk / interactive). PPC callees must outrank callers (shell stays lowest among clients). See [`qos.md`](qos.md), [ADR-006](decisions/006-workstation-qos.md).

**Microbenches (Phase 49)**
: `just bench` runs guest-timed echo RTT, blk read IOPS, and UDP TX PPS on QEMU aarch64 and writes markdown/JSON summaries. See [`bench.md`](bench.md).

**Shell (Phase 53 / 57 / 58)**
: Interactive REPL over serial with file/net/sys built-ins (`ls`…`df`, `ip`/`ping`, `uptime`/`history`/`clear`, apps). Long `cat`/`dmesg` use a space/`q` pager. `dmesg --pd` / `-l` filter the log ring; `ps`/`top`/`status` show service state; `calc`, `backup`, `fetch save`, `chat [#room]`. `help -l` and boot log `lerux-shell: cmds=` expose a machine-readable command list for smokes.

**App catalog (Phase 58)**
: Installable PD packages under `support/packages/`: edit, chat-client, http-file-browser, backup, fetch-client (≥5). See [`packages.md`](packages.md).

**Multi-arch workstation (Phase 59)**
: Workstation is a profile concept across arches: `workstation` (aarch64), `workstation-riscv`, `workstation-x86`, `workstation-rpi4`. Shared app channels; arch-specific serial/virtio/time drivers. Tiers: [`platforms.md`](platforms.md).

**Config policy (Phase 54)**
: FS-backed keys under `/config/` via `config-server` ([`docs/config.md`](config.md)). Supervisor seeds missing keys only (`boot.seeded`), logs active hostname/net.mode/log.level, and may rotate `/boot.log`. Shell: `config get|set|list|del`, `hostname`. Secrets use the `secret.*` prefix (`/config/secrets/`). Host: `lerux config schema|defaults|seed-disk`.

**Package (Phase 40 / 55)**
: One PD crate plus its interface-types version and an optional profile fragment (`support/packages/<name>.toml`). Host CLI: `lerux package search|install|remove|upgrade` merges fragments into profiles (channel auto-wiring by name), then `lerux profile build`. Pins in `support/package-pins.toml`. See [`packages.md`](packages.md). Microkit does not load arbitrary ELFs at runtime.

**Supervisor**
: Evolution of `boot-init` (Phase 33): `supervisor` PD provides RTC/timer, brings up FS/net services, performs ordered app notify (generalizes composed-sync), exposes reboot/status IPC. Phase 56: static **service graph** log lines (`unit=… after=… restart=no`) and a post-bring-up **watchdog** re-query of the timer PD. Phase 57: richer `ServiceStatus` (state + last error) from FS/net probes; applies `log.level` to log-server.

**Observability (Phase 57)**
: Tagged log ring (`log-server`), shell filters, host `lerux diagnose` on serial captures under `build/smoke-logs/`, optional `lerux bench --check` against `support/bench-thresholds.toml`. See [`ops.md`](ops.md).

**Ported app checklist** (new PD that users interact with):

1. Define request/response types in `lerux-interface-types`
2. Implement client and/or server PD (`#![no_std]`, `lerux-ipc`)
3. Wire channels in the profile `[[channel]]` manifest (workstation templates are channel-free; `lerux profile check-channels` validates ends)
4. Add `board-<profile>` features in PD `Cargo.toml` files
5. Register board in `support/boards.toml`, smoke expects in `lerux-cli`, CI job if needed

## Boundaries

- **In scope:** Rust PD crates, `.system` files, build/CI, docs, host profile tooling
- **Out of scope:** POSIX/glibc/musl, Linux ABI emulation, seL4 kernel modifications, C userspace, vendored upstream trees, unmodified third-party Linux binaries
- **Upstream SDK components:** Microkit monitor, loader, libmicrokit (C) — part of SDK, not lerux-owned