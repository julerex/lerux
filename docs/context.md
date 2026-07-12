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

Echo IPC and virtio smoke tests run on aarch64, RISC-V virt, and x86 (PCI virtio on q35). Block IPC (read + write) and net IPC (UDP TX) run on all three arches. RTC/timer init (`boot-init` + PL031/SP804) is aarch64 virt only until rust-sel4 adds drivers for other platforms.

The composed board (`qemu_virt_aarch64_composed`) runs `boot-init` and `hello`+virtio in one system. Both PDs log via serial IPC (multi-client serial driver). `boot-init` notifies `hello` when init is complete before virtio starts (composed-sync). See [`boards.md`](boards.md) and [`plan.md`](plan.md).

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

**Network topology (Phase 43)**
: Untrusted apps use only `NetRequest` / `NetResponse` against `net-server`. On aarch64 virtio-net, **unified-dma** removes the separate client_dma MR: Hal + bounce share `virtio_net_driver_dma`; the stack maps the bounce half only. Apps never map net DMA. See [ADR-003](decisions/003-net-virtualiser.md), [`net-topology.md`](net-topology.md).

**Filesystem backends (Phase 44 / 50)**
: `fs-server` serves `FsRequest` / `FsResponse` over virtio-blk (or emmc2). Default on-disk format is **LERUXFS2** (`lerux-fs`): hierarchical directories, free-map allocation, multi-sector contiguous files (up to 16 KiB). IPC includes `Mkdir` / `Unlink` / `Rename` and path-scoped `ListDir`. Path grammar: `/`-separated components, optional leading `/`, max 48-byte paths (see `lerux-interface-types`). Alternate **FAT16** backend (`lerux-fat`, feature `backend-fat`) stays root-only, 8.3, single-cluster; hierarchy ops return `Error`. Select via board feature (`qemu_virt_aarch64_fs` vs `qemu_virt_aarch64_fs_fat`). Shell/edit/config stay on the same IPC.

**Service async (Phase 45)**
: Service PDs keep Microkit `Handler` as the root event loop. Long sequential device I/O may use **stackless cooperative async** (`lerux-service-async`: `SingleTask`, `poll_fn`, `WakeCell`) instead of only explicit step machines. Clients still use postcard `Poll` RPC. See [ADR-004](decisions/004-service-async.md).

**Debug / faults (Phase 46)**
: Child PD faults can be delivered to a **parent** PD (`Handler::fault`) via Microkit hierarchy. Demo: `debug-handler` + `crash-demo` (`just test-debug`). Interactive host debugging uses QEMU’s gdbstub + `gdb-multiarch` ([`debug.md`](debug.md), [ADR-005](decisions/005-debug-pd.md)).

**Hardware serial smoke (Phase 47)**
: On-device boot checks use `LERUX_HW_SERIAL` + `--mode hw-serial` / `just test-hw`, with expects from `support/smoke-expects.toml` and a local board lock. Cloud CI stays QEMU-only; optional self-hosted workflow is documented in [`ci.md`](ci.md).

**Workstation QoS (Phase 48)**
: Fixed Microkit PD priorities form service classes (platform / services / control / bulk / interactive). PPC callees must outrank callers (shell stays lowest among clients). See [`qos.md`](qos.md), [ADR-006](decisions/006-workstation-qos.md).

**Microbenches (Phase 49)**
: `just bench` runs guest-timed echo RTT, blk read IOPS, and UDP TX PPS on QEMU aarch64 and writes markdown/JSON summaries. See [`bench.md`](bench.md).

**Package**
: One PD crate plus its interface-types version and an optional profile fragment (`support/packages/<name>.toml`). “Installing” a package means adding the PD to a `support/profiles/*.toml` and rebuilding the static image via `lerux profile build` — Microkit does not load arbitrary ELFs at runtime. CI can publish per-PD ELF artifacts; pins live in `support/package-pins.toml` (`lerux package list|show|build|pin|diff`). (Phase 40)

**Supervisor**
: Evolution of `boot-init` (Phase 33): `supervisor` PD provides RTC/timer, brings up FS/net services, performs ordered app notify (generalizes composed-sync), exposes reboot/status IPC.

**Ported app checklist** (new PD that users interact with):

1. Define request/response types in `lerux-interface-types`
2. Implement client and/or server PD (`#![no_std]`, `lerux-ipc`)
3. Wire channels in a profile `.system` template; match `Channel` constants to XML
4. Add `board-<profile>` features in PD `Cargo.toml` files
5. Register board in `support/boards.toml`, smoke expects in `lerux-cli`, CI job if needed

## Boundaries

- **In scope:** Rust PD crates, `.system` files, build/CI, docs, host profile tooling
- **Out of scope:** POSIX/glibc/musl, Linux ABI emulation, seL4 kernel modifications, C userspace, vendored upstream trees, unmodified third-party Linux binaries
- **Upstream SDK components:** Microkit monitor, loader, libmicrokit (C) — part of SDK, not lerux-owned