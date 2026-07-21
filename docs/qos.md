# Workstation QoS / priority policy (Phase 48)

Microkit uses **fixed priorities** (higher number wins). Isolation in v1 is:

1. Documented **service-class** priority bands in workstation templates
2. **PPC priority rule** (callee priority > caller priority) — hard Microkit constraint
3. **Single-flight** server jobs in `fs-server` / `net-server` (coarse bulk throttle)

No seL4 MCS budgets. See [ADR-006](decisions/006-workstation-qos.md).

## Critical rule: PPC direction vs priority

Microkit rejects systems where a PD with `pp="true"` has **priority ≥** the peer:

> PPCs must be to protection domains of strictly higher priorities

So every client that uses protected procedure calls must sit **below** its servers.

The shell PPCs `edit`, `chat`, `fs_server`, `net_server`, `supervisor`, `log_server`, `serial_virt`, and `config_server`. Therefore **shell must remain the lowest** of those PDs (priority **1**). “Interactive above bulk” is **impossible** while shell launches edit/chat via PPC.

Interactive isolation under load relies on:

- Bulk apps **blocking** in PPC/wait most of the time (not busy-spinning)
- Drivers and services outranking apps so IRQs and ring progress are not delayed by shell
- Single outstanding FS/net job limiting bulk fan-out

## Service classes (workstation)

| Class | Priority band | PDs | Role |
|-------|---------------|-----|------|
| **Platform** | 10–6 | `serial_driver` 10, `serial_virt` / `virtio_blk` 9, `virtio_net` 8, timers 7–6 | IRQ + device rings |
| **Services** | 5–4 | `log_server` 5, `fs_server` / `net_server` 4 | Shared infra (PPC servers) |
| **Control** | 3–2 | `config_server` 3, `supervisor` 2 | Orchestration |
| **Bulk apps** | 2 | `edit`, `chat_client`, `http_file_browser` | Best-effort user apps |
| **Interactive** | 1 | `shell` | Human REPL (must be ≤ all PPC targets − 1) |

RPi4 workstation uses the same class mapping (`emmc2_driver` / `genet_driver` in Platform).

## Inventory (QEMU workstation)

| PD | Priority | Class |
|----|----------|-------|
| `serial_driver` | 10 | Platform |
| `serial_virt` | 9 | Platform |
| `virtio_blk_driver` | 9 | Platform |
| `virtio_net_driver` | 8 | Platform |
| `pl031_driver` | 7 | Platform |
| `sp804_driver` | 6 | Platform |
| `log_server` | 5 | Services |
| `fs_server` | 4 | Services |
| `net_server` | 4 | Services |
| `config_server` | 3 | Control |
| `supervisor` | 2 | Control |
| `edit` / `chat_client` / `http_file_browser` | 2 | Bulk |
| `shell` | 1 | Interactive |

## Inherent bulk throttling

- **fs-server**: one `FsJob` at a time; extra clients get `Pending`
- **net-server**: one async client op (`begin_async`); other clients wait

These serialize bulk I/O without token buckets.

## Runtime view

```text
lerux> qos
--- qos (Phase 48) ---
class        band   examples
platform     10-6   serial, virtio/genet/emmc, timers
services     5-4    log, fs, net
control      3-2    config, supervisor
bulk         2      edit, chat, http-fs
interactive  1      shell (below all PPC servers)
note: Microkit PPC requires callee priority > caller
```

## Abuse tests (Phase 60 Track D)

### Host (CI / `just check`)

```bash
lerux profile check-qos              # all profiles with default_board
lerux profile check-qos workstation  # one profile
```

Parses the composed SDF and fails if:

1. Any PPC edge has caller priority ≥ callee priority (Microkit rule; also caught at image build).
2. On **admin** / **admin-core** profiles: service-class floors from this doc are violated (e.g. shell ≠ 1, platform drivers &lt; 6, fs/net &lt; 4).

### Guest concurrent-boot smoke

`just test-workstation` (and riscv/x86/rpi variants) expects:

- Higher-priority path: `lerux-supervisor: timer ok`, `lerux-fs: ready`, `lerux-net: ready`
- Bulk apps attach: `lerux-edit: ready` / `top count=` (arch-dependent)
- Interactive still live: `lerux-shell: ready` and **`lerux-shell: qos ok`**

That proves shell initialization completed while platform/service/bulk PDs also ran — the practical anti-starvation signal under fixed priorities + single-flight I/O. It is **not** a synthetic CPU-spin stress benchmark.

### Manual stress

1. Boot workstation (`just disk-img && just run` with `BOARD=qemu_virt_aarch64_workstation`).
2. At `lerux>`: `fetch` (net bulk), then `ls` / `top` / `qos` immediately.
3. Pass: prompt returns without multi-second freeze; `ls` succeeds.

### Residual / deferred

| Mechanism | Status |
|-----------|--------|
| Fixed priority bands | Enforced by template + `check-qos` |
| Single-flight fs/net jobs | Code path; serializes bulk clients (`Pending`) |
| MCS / time partitions | **Deferred** — needs ADR; not enabled |
| Shell above bulk apps | **Impossible** while shell→app launch is PPC |

If measured starvation appears (busy higher-prio PD), next steps: MCS budgets or convert shell→app launch off PPC — not priority inversion against the PPC rule.

## Changing priorities

1. Sketch caller→callee edges from `support/profiles/workstation.toml` (`pp = true` = caller)
2. Topologically assign priorities so every callee > caller
3. Update templates + this file + shell `qos` text
4. `just test-workstation` (microkit validates PPC priorities at image build)
