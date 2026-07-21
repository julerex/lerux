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
| Shell over-privileged surface | **Partial** | Profile tiers (`admin` / `admin-core` / `appliance`); `lerux profile audit`; shell still holds many ends on admin images |
| Secrets on disk writable by shell | **Mitigated (ACL)** | `secret.*` Set/Delete is supervisor-only (`ConfigResponse::Denied` for shell) |
| Secrets on disk readable by any FS client | **Partial** | Path prefix `/config/secrets/`; no encryption; FS RPC clients can still open paths |
| Compromised net-server | **Accepted residual** | Stack is trusted; full sDDF copy-swarm deferred |
| Malicious `loader.img` | **Partial (host digest)** | SHA-256 sidecar at image build; `lerux deploy` verifies by default; asymmetric / measured boot still open |
| Channel/QoS abuse (starve shell) | **Mitigated (checks)** | Fixed priorities + single-flight; `lerux profile check-qos` + workstation concurrent-boot smoke; MCS deferred |
| Supply-chain pin drift | **Mitigated (process)** | Pins in `deps/versions.toml` / `Cargo.toml` / package pins; [runbook](#dependency-pins-and-security-update-runbook-track-b) for bumps and CVE response |

## Isolation smoke (automated)

Board: `qemu_virt_aarch64_isolation` (`just test-isolation`).

1. `crash-demo` (child of `debug-handler`) deliberately raises a VM fault.
2. `debug-handler` logs the fault, suspends the child, and **notifies** `fs-client`.
3. `fs-client` then performs a normal FS round-trip against `fs-server`.

Success strings prove the untrusted fault path ran **and** the FS service remained usable afterward:

- `lerux-debug: crash-demo stopped`
- `lerux-isolation: fs-server survived untrusted PD crash`

Production **workstation** images stay flat (no debug parent) per ADR-005. Isolation is a CI property of the PD layout, not something users enable on device.

## Capability audit (Track A)

### Profile risk tiers

| `trust_class` | Profiles | Surface |
|---------------|----------|---------|
| **admin** | `workstation`, `workstation-riscv`, `workstation-x86`, `workstation-rpi4` | Shell + FS/net/config/supervisor + bulk apps (edit/chat/http-fs/backup) |
| **admin-core** | `dev-workstation` | Shell + services only; install apps via `lerux package` |
| **appliance** | `net-appliance`, `server` | Fixed PD set; no interactive admin shell |
| **minimal** | `minimal`, `hardware-rpi4` | Serial hello / bring-up |
| **debug** | isolation/debug boards (inferred) | Fault parent + crash child — not production default |

Declare with `trust_class = "admin"` (etc.) in `support/profiles/*.toml`. Host tooling:

```bash
lerux profile list              # shows [trust_class] column
lerux profile audit             # all profiles
lerux profile audit workstation # one profile: PD domains + high-risk edges
```

### Shell surface (admin)

The shell holds many RPC ends by design (REPL admin console). High-risk edges flagged by `profile audit`:

- shell ↔ supervisor — reboot / status
- shell ↔ config-server — policy R/W (see secrets ACL)
- shell ↔ fs-server / net-server — full storage and network RPC
- shell ↔ edit / chat / backup — app launch

**Shrink the surface by choosing a lower tier profile**, not by runtime capability dropping (Microkit is static). Prefer `dev-workstation` or `net-appliance` when bulk apps or a full REPL are unnecessary.

### Config ACL (`secret.*`)

`config-server` allows `Get`/`List` of secrets from shell (operator inspection) but **`Set`/`Delete` on `secret.*` is supervisor-only**. Non-supervisor writers receive `ConfigResponse::Denied`. Ordinary keys (`hostname`, `net.*`, `log.*`) remain writable from shell.

Still open: encryption at rest; path-level FS ACL for apps that speak `FsRequest` directly (any FS client can open `/config/secrets/` if it has FS rights — prefer config IPC).

### Image integrity (Track C)

Host-side **SHA-256 digests** for `loader.img` (not asymmetric signing, not measured boot):

| Step | Command / behavior |
|------|--------------------|
| Write | Automatic after `lerux image` → `build/<board>/loader.img.sha256` (`sha256sum` format) |
| Write (manual) | `lerux digest --board <board>` or `lerux digest -p path/to/loader.img` |
| Verify | `lerux verify-image --board <board>` |
| Deploy | `lerux deploy …` verifies the sidecar **by default**; copies `.sha256` to media; skip with `--no-verify` |

```bash
BOARD=rpi4b_4gb_workstation just image   # writes loader.img + loader.img.sha256
lerux verify-image --board rpi4b_4gb_workstation
lerux deploy --board rpi4b_4gb_workstation --dest /media/$USER/boot
# sha256sum -c build/rpi4b_4gb_workstation/loader.img.sha256   # host cross-check
```

**Trust model:** the sidecar is only as strong as the host that wrote it (CI or developer machine). It catches accidental corruption and casual SD-card swap of `loader.img` without the matching digest. It does **not** replace secure boot, signed releases, or a hardware root of trust.

**Out of scope for Track C:** ed25519/cosign signatures, TPM/fuse measured boot, in-guest verification.

### QoS / channel abuse (Track D)

| Check | How |
|-------|-----|
| PPC priority rule | `lerux profile check-qos` (host; also in `just check`) |
| Service-class bands | Same command on admin/admin-core profiles ([`qos.md`](qos.md)) |
| Concurrent boot under bulk apps | `just test-workstation` expects `lerux-shell: qos ok` after services/apps init |

MCS budgets and raising shell above bulk apps remain **deferred** (ADR-006). See [`qos.md`](qos.md#abuse-tests-phase-60-track-d).

### Remaining stretch

1. **Reduce shell channel set further** — launch apps without giving shell every service end (may need non-PPC notify; see ADR-006)
2. **Asymmetric image signing / measured boot** — follow-on after host digests

## Dependency pins and security update runbook (Track B)

lerux does **not** vendor seL4 / Microkit / rust-sel4 trees into git. Security and feature bumps are **pin updates** followed by rebuild and smoke. This section is the operational runbook.

### Pin inventory

| Component | Pin location | Current (see also plan version table) | When to bump |
|-----------|--------------|----------------------------------------|--------------|
| seL4 | `deps/versions.toml` → `[repos].sel4.tag` | 15.0.0 | Kernel CVE / release notes from seL4 Foundation |
| Microkit | `deps/versions.toml` → `[repos].microkit.tag` | 2.2.0 | Microkit release; often paired with seL4 |
| rust-sel4 (docs mirror) | `deps/versions.toml` → `[rust].rust_sel4.tag` | v4.0.0 | Keep in sync with Cargo.toml |
| rust-sel4 (build) | root `Cargo.toml` workspace git `tag = "…"` for all `sel4*` deps | v4.0.0 | rust-sel4 release / API fixes; **must match** versions.toml |
| Rust toolchain | `rust-toolchain.toml` | nightly-2026-03-18 | rustc regressions, or when rust-sel4 requires a newer nightly |
| PD package ELFs | `support/package-pins.toml` | per-board sha256 | After any PD/interface rebuild that ships CI artifacts |
| Host crates (clap, etc.) | `Cargo.lock` | resolved | Dependabot / `cargo update` for **host** crates only; PD path is git-pinned |

**Invariant:** `deps/versions.toml` `[rust].rust_sel4.tag` and every `sel4*` git dep tag in root `Cargo.toml` must be the **same** string. `lerux fetch` clones seL4/Microkit from versions.toml; Cargo resolves rust-sel4 from `Cargo.toml`.

### Severity triage

| Signal | Action |
|--------|--------|
| seL4 Foundation security advisory / kernel CVE in pinned tag | **P0** — bump seL4 (+ Microkit if required), rebuild SDK, full smoke, redeploy images |
| Microkit bug that breaks isolation or loader integrity | **P0** — bump Microkit, rebuild SDK, full smoke |
| rust-sel4 fix for driver/IPC safety used by lerux PDs | **P1** — bump tag in Cargo.toml + versions.toml, `cargo update -p sel4…`, check-pd + smoke subset |
| Host-only crate CVE (clap, serde on host tools) | **P2** — `cargo update` host path; `just check`; no SDK rebuild |
| PD package pin drift (sha256 mismatch in CI) | **P2** — `lerux package upgrade --all` after green build; commit `package-pins.toml` |
| Nightly rustc break on pinned channel | **P1** — move `rust-toolchain.toml` only after `just check` + `just check-pd` |

Document the CVE / advisory ID in the commit message and, if user-facing, a short note in `docs/plan.md` version table when seL4/Microkit major pins move.

### Standard bump procedure

Work on a branch. Prefer one logical bump per PR (e.g. “seL4 15.0.0 → 15.0.1” or “rust-sel4 v4.0.0 → v4.0.1”).

#### 1. Edit pins

```bash
# seL4 and/or Microkit
$EDITOR deps/versions.toml

# rust-sel4: edit root Cargo.toml (all sel4* git tags) AND deps/versions.toml [rust].rust_sel4
# Then refresh the lockfile for git deps:
cargo update -p sel4
# (or cargo update for a broader refresh; review Cargo.lock diff)
```

#### 2. Refresh sources and SDK

```bash
# Drop stale checkouts when tags change (fetch re-clones/updates deps/workspace/)
rm -rf deps/workspace/seL4 deps/workspace/microkit   # only if tags moved
just fetch

# Rebuild SDK for boards you care about (CI builds a matrix; local minimum:)
MICROKIT_BOARDS=qemu_virt_aarch64,x86_64_generic,qemu_virt_riscv64,rpi4b_4gb just build-sdk
# Or prebuilt fallback when the pin matches a published SDK:
# just fetch-sdk
```

If only **rust-sel4** moved (kernel/Microkit tags unchanged), SDK rebuild is usually **not** required; still run `just check-pd` so PD crates recompile against new bindings.

#### 3. Quality gates (local)

```bash
just check          # host fmt + clippy (lerux-cli, interface-types)
just check-pd       # cross-arch PD clippy (needs SDK)
just test-isolation # Phase 60 isolation smoke (fast security-relevant)
just test-workstation
# Prefer full matrix before merge when seL4/Microkit moved:
# just test-all
```

CI also runs `check` → `sdk` → `check-pd` + smoke matrix ([`ci.md`](ci.md)).

#### 4. Package pins (if PD ELFs are published)

```bash
lerux package build edit --board qemu_virt_aarch64_workstation
lerux package upgrade --all --board qemu_virt_aarch64_workstation
# review support/package-pins.toml; commit with the bump
```

#### 5. Ship

- Commit pin files + `Cargo.lock` + any necessary PD/API fixups.
- Push and wait for green CI (smoke matrix is the acceptance bar for kernel/Microkit bumps).
- Rebuild and redeploy field images (`just image` / `lerux deploy`) for boards in use; **old `loader.img` files are not hot-patched**.
- Optional lab: RPi4 `just test-hw` after deploy ([`boards.md`](boards.md)).

### Incident response template

Copy into the PR or issue body:

```markdown
## Security pin update

- **Advisory / CVE / issue:** …
- **Component:** seL4 | Microkit | rust-sel4 | host crate | package pin
- **Old pin → new pin:** …
- **Affects deployed images?** yes/no (if yes: boards …)
- **SDK rebuild required?** yes/no
- **Verification:**
  - [ ] `just check`
  - [ ] `just check-pd`
  - [ ] smoke: isolation / workstation / test-all (link CI)
  - [ ] package-pins updated (if applicable)
  - [ ] field images rebuilt / operators notified
- **Residual risk:** …
```

### What not to do

- Do not commit trees under `deps/workspace/` or `deps/microkit-sdk/` (gitignored).
- Do not bump seL4 without checking Microkit compatibility notes for that release.
- Do not leave `versions.toml` rust-sel4 tag different from `Cargo.toml`.
- Do not treat `lerux package` pins as a substitute for rebuilding from source in CI — they record artifact digests, they do not load arbitrary remote ELFs at runtime.

### Related ops

- Smoke matrix and caches: [`ci.md`](ci.md)
- RPi4 deploy after image rebuild: [`boards.md`](boards.md#rpi4-workstation-install-path-phase-52)
- Package pin CLI: [`packages.md`](packages.md)

## Related

- [ADR-003](decisions/003-net-virtualiser.md) — net trust map
- [ADR-005](decisions/005-debug-pd.md) — fault parent (not production default)
- [ADR-006](decisions/006-workstation-qos.md) — priorities / PPC
- [debug.md](debug.md) — `test-debug` and GDB
- [plan-arch.md](plan-arch.md) — Phase 60 checklist
- [net-topology.md](net-topology.md) — NIC channel map
- [ci.md](ci.md) — CI gates after pin bumps
