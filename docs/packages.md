# Packages and profiles (Phase 40 / 55)

lerux packages are **host-side composition units**, not runtime packages. A package is a PD crate + `interface_types` pin + optional profile fragment. Installing merges the fragment into a system profile and rebuilds `loader.img`. Microkit does **not** load arbitrary ELFs at runtime.

## CLI

```bash
lerux package list
lerux package search editor          # substring over name/pd/description
lerux package show edit

# Merge fragment into a profile (writes support/profiles/<name>.toml)
lerux package install edit --profile dev-workstation
lerux package install chat-client --profile dev-workstation
lerux package install http-file-browser --profile dev-workstation

lerux package remove edit --profile dev-workstation

# Rolling pins (CI artifacts / local ELF SHA256)
lerux package build edit --board qemu_virt_aarch64_workstation
lerux package pin edit --board qemu_virt_aarch64_workstation
lerux package diff edit --board qemu_virt_aarch64_workstation
lerux package upgrade edit --board qemu_virt_aarch64_workstation
lerux package upgrade --all --board qemu_virt_aarch64_workstation

lerux profile build dev-workstation
lerux profile validate
lerux profile check-channels
```

Install checks `fragment.requires` against the profile’s current PD set, merges `fragment.pds`, and appends named `[[fragment.channel]]` edges (skips channels whose `name` already exists).

## Recipes (Phase 55 / 60)

| Profile | `trust_class` | Role |
|---------|---------------|------|
| `minimal` | minimal | Serial hello |
| `server` | appliance | Echo IPC demo |
| `net-appliance` | appliance | HTTP server over virtio-net |
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

Pins: `support/package-pins.toml` (sha256 per board artifact).

## “AUR for lerux” (out-of-tree packages)

1. Out-of-tree crate implements a PD (`#![no_std]`, `lerux-ipc`) against a published `lerux-interface-types` version.  
2. Ship a package TOML + optional profile fragment (same shape as `support/packages/`).  
3. Vendor or path-depend the crate in the consumer tree; copy the TOML into `support/packages/`.  
4. `lerux package install <name> --profile <recipe>` then `lerux profile build <recipe>`.  
5. Follow the **ported app checklist** in [`context.md`](context.md).  

Breaking `interface_types` majors requires rebuilding all PDs that speak that IPC.

## Not in scope

Runtime dynamic loading of ELFs into a live Microkit image. That would need a different system model (ADR).
