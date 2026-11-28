# Packages and profiles (Phase 40 / 55)

lerux packages are **host-side composition units**, not runtime packages. A package is a PD crate + `interface_types` pin + optional profile fragment. Installing merges the fragment into a system profile and rebuilds `loader.img`. Microkit does **not** load arbitrary ELFs at runtime.

## CLI

```bash
lerux package install --board swift_sfx14_41g
lerux package upgrade --board pc_z97_d3h_i217
lerux package upgrade --all

lerux verify-image --board rpi4b_4gb
```

`lerux package install` and `lerux package upgrade` rebuild the selected board profile, write `support/config/policy/package-pins.toml`, write `loader.img.sha256`, and print the pin digest. `--all` rebuilds the Swift (`swift_sfx14_41g`), the Z97 (`pc_z97_d3h_i217`), and the Pi (`rpi4b_4gb`). The guest cannot install a package. The operator rebuilds on the host.

`lerux image` also writes `loader.img.sha256`. `lerux verify-image` checks that sidecar and, when the digest matches, prints the flash line for the Swift, the Z97, or the Pi. That line packs the checked `loader.img` with `lerux iso --no-build` and then `dd` for the Swift. The Z97 line is that ISO path, or `just deploy-pc` on a mounted FAT volume. The Pi line is `just deploy-rpi4`. The `just make-lerux-iso-*` recipes rebuild `loader.img`, so they stay the build step and are not the flash line.

## Recipes (Phase 55 / 60)

| Profile | `trust_class` | Role |
|---------|---------------|------|
| `minimal` | minimal | Serial hello |
| `server` | appliance | Echo IPC demo |
| `net-appliance` | appliance | HTTP server over virtio-net |
| `browser` | appliance | Phase 76: browser-ui + one web-content |
| `agent-runtime` | appliance | Phase 77: agent PD vs grok-one stub |
| `agent` | appliance | Phase 79: agent tools (fs + WebFetch) vs grok-one |
| `workstation-interactive` | admin | Phase 80: workstation + browser + agent (not the default) |
| `dev-workstation` | admin-core | Workstation **core** without bulk apps — use `package install` |
| `workstation` | admin | Full apps pre-wired (edit/chat/http-fs/backup) |
| `workstation-riscv` / `-x86` / `-rpi4` | admin | Arch / hardware variants of full workstation |

```bash
lerux profile audit workstation   # Phase 60: PD trust domains + high-risk edges
lerux profile list                # shows [trust_class] column
```

## App catalog packages (Phase 58)

| Package | Shell / use | Notes |
|---------|-------------|-------|
| `edit` | `edit <path>` | TUI editor |
| `chat-client` | `chat [#room]` | Multi-room UDP chat |
| `http-file-browser` | host curl :8080 | MIME, HTML listing, PUT |
| `backup` | `backup [snapshot\|status]` | `/backup/manifest` |
| `fetch-client` | smoke board / install | HTTP GET one-shot PD |
| (shell) `calc` | `calc (1+2)*3` | Integer REPL math |
| (shell) `top` | `top` | Service table + uptime |

≥5 installable daily apps: edit, chat-client, http-file-browser, backup, fetch-client.

## Package manifest

`support/packages/<name>.toml`:

```toml
pd = "edit"
interface_types = "0.1.0"
description = "…"

[fragment]
pds = ["edit"]
requires = ["shell", "fs-server"]

[[fragment.channel]]
name = "edit_shell"
ends = [
  { pd = "shell", id = 6, pp = true },
  { pd = "edit", id = 0 },
]
```

- **pd** — Cargo crate / ELF basename  
- **interface_types** — postcard contract version (must match tree when building)  
- **fragment** — what `install` merges into a profile  

Pins: `support/config/policy/package-pins.toml` (sha256 per board artifact).

## “AUR for lerux” (out-of-tree packages)

1. Out-of-tree crate implements a PD (`#![no_std]`, `lerux-ipc`) against a published `lerux-interface-types` version.  
2. Ship a package TOML + optional profile fragment (same shape as `support/packages/`).  
3. Vendor or path-depend the crate in the consumer tree; copy the TOML into `support/packages/`.  
4. `lerux package upgrade --board <board>` rebuilds that board profile, writes `loader.img.sha256`, and prints the pin digest. `lerux profile build <recipe>` rebuilds a named profile and links `loader.img`.  
5. Follow the **ported app checklist** in [`context.md`](../overview/context.md).  

Breaking `interface_types` majors requires rebuilding all PDs that speak that IPC.

## Not in scope

An on-device package manager. The guest cannot install a package.

Runtime loading of native ELFs into a live Microkit image. A signed module inside `program-runtime` is [Phase 81](../../plans/core/plan.md#phase-81--signed-wasm-runtime) ([ADR-010](../../decisions/007-012/010-program-runtime.md)). Milestone 14 runs ordinary WebAssembly on `qemu_virt_aarch64_program_sddf` ([ADR-012](../../decisions/007-012/012-ordinary-webassembly.md)).
