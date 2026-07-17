# Security posture (Phase 60)

lerux is a **static Microkit system**: protection domains, memory maps, and channels are fixed at image build time. Isolation is therefore a property of the **composed SDF + typed IPC contracts**, not of a dynamic process model.

This document is the Phase 60 threat model and trust map. It does not claim formal verification of lerux PDs (kernel seL4 is verified; userspace is not).

## Assets

| Asset | Why it matters |
|-------|----------------|
| Block device / on-disk FS | Integrity of configs, logs, app data |
| Network stack state | Availability and privacy of sockets |
| Config keys under `/config/` (incl. `secret.*`) | Policy and credentials (path isolation today; no encryption at rest) |
| Service readiness | Shell and apps need FS/net/supervisor alive |
| Image integrity | What boots is what CI built (signing is host-side stretch) |

## Trust domains

```
┌─────────────────────────────────────────────────────────────────┐
│  Platform (highest privilege for devices)                       │
│  serial-driver, virtio-*-driver, genet, emmc2, RTC/timer PDs    │
│  Maps: MMIO, IRQs, DMA rings                                    │
└────────────────────────────▲────────────────────────────────────┘
                             │ rings / device RPC (not app-facing)
┌────────────────────────────┴────────────────────────────────────┐
│  Trusted services                                               │
│  fs-server, net-server, serial-virt, config-server, log-server, │
│  supervisor, blk-server (when present)                          │
│  Maps: device client DMA only where required; no app DMA share  │
└────────────────────────────▲────────────────────────────────────┘
                             │ postcard RPC (Fs/Net/Config/Log/…)
┌────────────────────────────┴────────────────────────────────────┐
│  Untrusted / interactive apps                                   │
│  shell, edit, chat-client, http-file-browser, backup,           │
│  fetch-client, crash-demo (isolation smoke)                     │
│  Maps: **no** virtio/net/blk DMA; channels only                 │
└─────────────────────────────────────────────────────────────────┘
```

**Rule (ADR-003, workstation):** untrusted apps never map NIC or block DMA. They speak `NetRequest` / `FsRequest` (and similar) only.

## Trust map (workstation-shaped)

| PD | Trust class | MMIO / IRQ | DMA | Clients may call | Must not map |
|----|-------------|------------|-----|------------------|--------------|
| `virtio-*-driver` / genet / emmc2 | platform | yes | driver (+ bounce per ADR-003) | service only | — |
| `serial-driver` | platform | UART | no | `serial-virt` (device-only mode) | app channels |
| `serial-virt` | service | no | no | apps (serial RPC) | UART MMIO |
| `fs-server` | service | no | blk client DMA | shell, edit, backup, http-fs, config | NIC DMA |
| `net-server` | service | no | net bounce (unified-dma) | shell, fetch, chat, http-fs | blk DMA |
| `config-server` / `log-server` | service | no | no | shell, supervisor | device DMA |
| `supervisor` | control | no | no | shell (status/reboot/time) | device DMA |
| shell / apps | untrusted | no | **none** | each other only via typed RPC | any DMA / MMIO |
| `debug-handler` | debug-only | no | no | hierarchy parent | production workstation default |

Channel numbers come from profile `[[channel]]` manifests; PPC callees outrank callers ([`qos.md`](qos.md), ADR-006).

## Threats and mitigations

| Threat | Status | Mitigation |
|--------|--------|------------|
| Untrusted app corrupts FS DMA / disk | **Mitigated** | Apps use postcard FS RPC only; only `fs-server` maps blk rings |
| Untrusted app sniffs NIC DMA | **Mitigated** (aarch64 virtio-net) | Unified-dma + net-server sole stack; apps have no DMA map (ADR-003) |
| App crash takes down services | **Mitigated (smoke)** | Separate PDs; `just test-isolation` crashes a child then FS round-trips |
| Shell over-privileged surface | **Partial** | Shell is lowest priority but holds many RPC ends; admin vs untrusted profile split is open |
| Secrets on disk readable by any FS client | **Partial** | Path prefix `/config/secrets/`; no encryption; any FS client with open rights can read |
| Compromised net-server | **Accepted residual** | Stack is trusted; full sDDF copy-swarm deferred |
| Malicious `loader.img` | **Open (stretch)** | Host-side image signing / measured boot not implemented |
| Channel/QoS abuse (starve shell) | **Partial** | Fixed priorities + single-flight jobs; MCS budgets deferred |
| Supply-chain pin drift | **Partial** | Pins in `deps/versions.toml` + package pins; no formal security update runbook yet |

## Isolation smoke (automated)

Board: `qemu_virt_aarch64_isolation` (`just test-isolation`).

1. `crash-demo` (child of `debug-handler`) deliberately raises a VM fault.
2. `debug-handler` logs the fault, suspends the child, and **notifies** `fs-client`.
3. `fs-client` then performs a normal FS round-trip against `fs-server`.

Success strings prove the untrusted fault path ran **and** the FS service remained usable afterward:

- `lerux-debug: crash-demo stopped`
- `lerux-isolation: fs-server survived untrusted PD crash`

Production **workstation** images stay flat (no debug parent) per ADR-005. Isolation is a CI property of the PD layout, not something users enable on device.

## Capability direction (open)

Near-term hardening without redesigning Microkit:

1. **Admin vs untrusted profiles** — e.g. `net-appliance` without shell edit/chat; workstation with full surface documented as higher risk.
2. **Reduce shell channel set** — launch apps without giving shell every service end where possible (may need non-PPC notify patterns; see ADR-006).
3. **Config ACL** — optional config-server deny for non-supervisor writers on `secret.*`.
4. **Image signing** — host CLI verifies digest before `deploy`; hardware roots later.
5. **Pin update runbook** — document response when rust-sel4 / Microkit / seL4 ship security fixes.

## Dependency pins (hygiene)

| Component | Pin location | Notes |
|-----------|--------------|--------|
| seL4 / Microkit | `deps/versions.toml` | Upstream tags; rebuild SDK after bump |
| rust-sel4 | root `Cargo.toml` git tag | Workspace-wide |
| PD packages | `support/package-pins.toml` | Rolling ELF pins via `lerux package upgrade` |

Security updates today: bump pin → `just check` / `just check-pd` → smoke matrix → rebuild images. A dedicated incident runbook remains stretch.

## Related

- [ADR-003](decisions/003-net-virtualiser.md) — net trust map
- [ADR-005](decisions/005-debug-pd.md) — fault parent (not production default)
- [ADR-006](decisions/006-workstation-qos.md) — priorities / PPC
- [debug.md](debug.md) — `test-debug` and GDB
- [plan-arch.md](plan-arch.md) — Phase 60 checklist
- [net-topology.md](net-topology.md) — NIC channel map
