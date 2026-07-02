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

“Arch-like” means **workflow**, not binary compatibility: rolling PD artifact pins, named system profiles, init ordering, shell + core utilities — each implemented as PDs you port deliberately. Unmodified Arch packages (`bash`, `pacman`, `firefox`, etc.) are out of scope.

## System profiles and packages

**System profile**
: A named bundle in `support/profiles/` (planned Phase 35): which PD crates, `.system` template, board features, and channel layout compose one `loader.img`. Override with `lerux profile build <name>` (host tooling).

**Package**
: One PD crate plus its interface-types version and an optional profile fragment. “Installing” a package means adding it to a profile and rebuilding the static image — Microkit does not load arbitrary ELFs at runtime.

**Supervisor**
: Planned evolution of `boot-init` (Phase 33): RTC/timer, bring up drivers and services (FS, net), notify app PDs in dependency order, expose reboot/status IPC.

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