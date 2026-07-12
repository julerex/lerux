# PLAN — Arch-level functionality (phases 50–60)

Last updated: 2026-07-12 (Phase 57 core done)

Related: [`plan.md`](plan.md) (completed phases 1–49), [`plan-au-ts.md`](plan-au-ts.md) (sDDF/LionsOS inspiration track), [`context.md`](context.md) (domain language).

## Context

lerux is a **Rust-only, non-POSIX** userspace on seL4 Microkit. Phases 1–49 delivered a QEMU workstation MVP and much of an RPi4 path: supervisor, FS/net IPC, shell, profiles/packages, serial/net virtualisers, QoS, debug, benches.

**Project definition of “Arch-like”** ([`context.md`](context.md)):

> Rolling PD artifact pins, named system profiles, init ordering, shell + core utilities — each implemented as PDs you port deliberately. Unmodified Arch packages (`bash`, `pacman`, `firefox`, etc.) are **out of scope**.

This plan maps **Arch Linux workflow and capability surface** onto that constraint. Target is not “run Arch packages”; it is “a daily-driver-feel console system for embedded/workstation seL4”: install/update compose images, manage storage/net/config, edit files, fetch over the network, observe and reboot — with hardware parity and a growing app catalog.

### Already at Arch-analogue (roughly)

| Arch concept | lerux today |
|--------------|-------------|
| Kernel | Upstream seL4 15.0.0 (not lerux-owned) |
| Init | `supervisor` (RTC/timer, service bring-up, reboot/status IPC) |
| Package set / ISO profiles | `support/profiles/*.toml` + `lerux profile build` |
| pacman package metadata | `support/packages/`, `package-pins.toml`, `lerux package` |
| Shell | `shell` REPL over serial (`ls cat write time ps top qos reboot fetch dmesg edit chat help`) |
| Storage | `fs-server` + LERUXFS2 / FAT16 slice on virtio-blk or eMMC2 |
| Network | `net-server` (UDP/TCP/DNS static map) + drivers |
| Logging | `log-server` + shell `dmesg` |
| Config | `config-server` FS-backed under `/config/` |
| Apps | `edit`, `chat-client`, `http-file-browser` |
| Multiarch bring-up | aarch64 / riscv64 / x86 serial+echo+virtio smokes |
| Hardware | RPi4 serial/net/blk/workstation profiles (manual HW gate still open) |

### Hard ceiling (do not plan as “become Arch”)

- No Linux/POSIX ABI, musl, `fork`/`exec`, unmodified third-party binaries
- Microkit **static** PD set at image build time — “install package” = pin + rebuild `loader.img`, not runtime ELF load
- No full desktop (Wayland/X) or browser-class stack unless a future ADR opens guest VMM / large runtimes

---

## Approach

Work in **vertical capability tracks**, each ending in a smoke-gated profile board. Prefer deepening existing IPC contracts (`lerux-interface-types`) and the workstation profile over new one-off boards. Align naming with Arch mental models in docs/CLI (`package`, `profile`, “rolling pins”) while keeping postcard RPC under the hood.

```
Foundation gaps          Daily-driver UX           Ecosystem
─────────────────        ─────────────────         ─────────────
50 FS v2 ──────────────► 53 Shell + coreutils-PD
51 Net stack v2 ───────► 54 Config & secrets
52 HW closeout ────────► 55 Package/repo UX ──────► 58 App catalog
        │                         │
        └──── 56 Time/RTC parity ─┘
              57 Observability
              59 Multi-board / multi-arch workstation
              60 Security posture (optional stretch)
```

Graphics, POSIX layers, and guest Linux (libvmm) stay **explicit non-goals** unless product requirements change (would need ADRs).

### Reuse map

| Area | Paths |
|------|--------|
| Domain language / Arch definition | `docs/context.md` |
| Completed roadmap | `docs/plan.md`, `docs/plan-au-ts.md` |
| IPC contracts | `userspace/crates/lerux-interface-types/src/lib.rs` |
| FS formats | `userspace/crates/lerux-fs/`, `userspace/crates/lerux-fat/` |
| FS/net/services | `userspace/pds/fs-server/`, `net-server/`, `supervisor/`, `shell/`, `config-server/`, `log-server/` |
| Profiles / packages | `support/profiles/`, `support/packages/`, `support/package-pins.toml` |
| System gen | `tools/lerux-cli/` (`profile`, `package`, `render_system`), ADR-001 |
| Ported-app checklist | `docs/context.md` (“Ported app checklist”) |
| HW gate | `docs/boards.md` (RPi4 workstation section), Phase 47 `hw-serial` |

---

## Phase 50 — Filesystem v2 (Arch: real storage) — core done

**Why:** Arch assumes hierarchical dirs, multi-block files, delete/rename, and usable capacity. LERUXFS1 was flat (≤16 files, one 512-byte sector each); FAT remains root-only, 8.3, single-cluster.

### Steps

- [x] Extend `FsRequest` / `FsResponse` for paths with directories, `Unlink`/`Rename`/`Mkdir` (path grammar on `MAX_FS_PATH` / interface-types docs).
- [x] **LERUXFS2**: multi-sector contiguous files (≤32 sectors / 16 KiB), directory sectors, free-map bitmap; magic `LERUXFS2`; LERUXFS1 superblocks reformat on mount.
- [ ] Finish **FAT** stretch: multi-cluster files, subdirs (or LFN if needed for host interchange); optional workstation FAT demo (`plan-au-ts` deferred items).
- [ ] Optional **NFS** or host-backed FS for QEMU user-net (dev convenience; LionsOS-inspired).
- [x] Shell: `mkdir`, `rm`, `mv`, `cd`/`pwd` (shell-local cwd); larger `cat`/`write` via chunked IPC.
- [x] Smokes: `just test-fs` (hierarchy + multi-sector), `just test-fs-fat` (basic parity; new ops Error on FAT), workstation boots.

### Exit

Files large enough for configs, logs, and edit buffers without artificial 512 B caps; hierarchical layout usable from shell. **Met for LERUXFS2**; FAT/NFS remain stretch.

---

## Phase 51 — Network stack v2 (Arch: “networking just works”) — core done

**Why:** Arch has DHCP, real DNS, concurrent sockets, HTTPS-ish fetch. Pre-v2 lerux had static QEMU addresses, static DNS map only, single TCP socket.

### Steps

- [x] **DHCP client** in `net-server` (smoltcp `Dhcpv4Socket`); apply on bring-up; static fallback after timeout; shell `ip` / `GetIface` show address.
- [x] **Real DNS** over smoltcp DNS socket; static map for `host`/`dns` still wins (deterministic smokes).
- [x] **Dual TCP** sockets (client + listen) so outbound connect and inbound listen can coexist; exclusive async client lock remains for mid-op serialization.
- [ ] **TLS** for outbound fetch (e.g. `rustls` + `webpki-roots` in a dedicated `tls-proxy` PD or net-server feature) — keep apps on cleartext IPC to the proxy if cert store is large.
- [ ] RPi4 workstation: TCP+DNS+DHCP on GENET (today `fetch` is UDP-demo-only on HW).
- [ ] Unified-dma / trust map on genet + x86 PCI (ADR-003 residual).
- [ ] Full multi-client queue (shell fetch while http-fs TcpRecv pending without `Pending`).

### Exit

`fetch https://…` (or TLS-terminated `fetch`) works on QEMU; RPi4 can reach a real host; smokes stay deterministic (DHCP mock or fixed QEMU DHCP). **Partial:** DHCP+DNS+GetIface+dual TCP on QEMU; TLS/RPi4 remain open.

---

## Phase 52 — Hardware closeout (Arch: install and use the machine) — core done

**Why:** Arch on real metal is the bar; RPi4 workstation image built but lacked a single install path and automated REPL checks.

### Steps

- [ ] Complete Phase 39 **lab** gate: serial REPL `ls`/`cat`/`fetch`/`edit` on device; record results in the boards.md checklist; fix drivers if failures recur.
- [x] Automate Phase 47 harness further: expand rpi4 workstation expects (fs/net/seed); **scripted** `ls`/`pwd`/`ip` over hw-serial after boot match.
- [x] First-boot disk format story: empty block → LERUXFS2 format → `mkdir /config` → seed net/hostname keys (`first-boot seed ok`).
- [x] Deploy ergonomics: `lerux deploy` / `just deploy-rpi4 DEST=…`, U-Boot helper file, install path in [`boards.md`](boards.md#rpi4-workstation-install-path-phase-52).
- [ ] Optional second board (e.g. another aarch64 SBC) only after RPi4 is reliable.

### Exit

Documented “install media → boot → shell works” path on RPi4 without folklore. **Met for tooling/docs/harness**; physical lab sign-off remains open.

---

## Phase 53 — Shell and core utilities (Arch: base packages) — core done

**Why:** Arch base is dozens of CLI tools; lerux shell was a thin REPL over a few IPC services.

### Steps

- [x] Expand built-ins: `mkdir`/`rm`/`mv`/`stat`/`df`, `ping`/`ifconfig`/`ip`, `date`/`time`/`uptime`, `clear`, `history` (ring in shell PD).
- [x] **Pager / less-like** for long `cat`/`dmesg` over serial (`-- more --`, space/q).
- [x] Structured **help** (`help`, `help -l`) and machine-readable `lerux-shell: cmds=` for smokes.
- [x] Prefer shell built-ins (no new coreutils PDs); `df` via `FsRequest::DiskInfo`, `uptime` via `SupervisorRequest::GetUptime`.
- [x] Deepen hw-serial scripts: `help -l`, `df` after boot match.

### Exit

A new user can administer files, net identity, services, and logs without knowing IPC channel IDs. **Met** for built-in surface.

---

## Phase 54 — Config, secrets, and boot policy (Arch: `/etc`, netctl) — core done

**Why:** Arch is configuration-driven; `config-server` was a thin FS key store without a published schema or boot policy.

### Steps

- [x] Schema for keys: net (IP/DHCP/DNS/mode/prefix), hostname, log level, rotate — [`docs/config.md`](config.md), `CFG_*`.
- [x] Supervisor seeds **missing** keys only, sets `boot.seeded`, logs `config hostname=… net.mode=… log.level=…` before net probe.
- [x] Shell `config get|set|list|del` + `hostname`; host `lerux config schema|defaults|seed-disk`.
- [x] Secrets: `secret.*` → `/config/secrets/` (path isolation; no encryption yet).
- [x] Boot log rotation: `log.rotate` renames `/boot.log` → `/boot.log.1`.
- [ ] Hot-apply static net from config into `net-server` (stretch).

### Exit

Changing hostname / net.* / log.* is a config write + reboot (values re-read and logged), not a rebuild. **Met** for policy surface; live net reconfigure remains stretch.

---

## Phase 55 — Package and profile UX (Arch: pacman + rolling) — core done

**Why:** Arch’s soul is package management. lerux had pins and profiles but “install” was a manual TOML edit.

### Steps

- [x] **Host-side package UX:** `lerux package search|install|remove` merges fragment `pds` + named channels into a profile; optional `--build`.
- [x] **Rolling pin workflow:** `package upgrade` / `upgrade --all` rebuilds, re-pins, prints SHA256 + interface_types delta.
- [x] **Profile recipes:** `net-appliance`, `dev-workstation` (+ existing minimal/server/workstation).
- [x] **Channel auto-wiring:** install merges `[[fragment.channel]]` by `name` (skip duplicates).
- [x] Docs: [`packages.md`](packages.md) (“AUR for lerux” + CLI).
- [x] **Not in scope:** runtime dynamic ELF load (still out of scope).

### Exit

Adding `edit` or a new app to a profile is one CLI command + rebuild; pins are auditable and rollable. **Met.**

---

## Phase 56 — Time and init parity (Arch: timedatectl / systemd units) — core done

**Why:** RTC/timer and composed init were **aarch64 virt-only**; RISC-V/x86 lacked PL031/SP804 stack.

### Steps

- [x] Platform timers: RISC-V Goldfish RTC + `rdtime` CSR (CLINT kernel-owned); x86 CMOS RTC + TSC (PIT owned by kernel for calibration) — thin lerux drivers.
- [x] Supervisor `GetTime` / `GetUptime` via stock `RtcClient`/`TimerClient` on aarch64, RISC-V, and x86 init boards (`just test-init{,-riscv,-x86}`).
- [x] Static **service graph** log lines (`unit=… after=… restart=no`) — still static PDs, ordered readiness like systemd units.
- [x] Watchdog: post-bring-up timer re-query (`lerux-supervisor: watchdog ok`).

### Exit

Cross-arch smoke parity table gains “init/time: yes” for RISC-V and x86; workstation concepts portable. **Met.**

---

## Phase 57 — Observability and ops (Arch: journalctl, systemd-analyze) — core done

**Why:** Arch admins debug with logs, process state, and metrics; lerux has log-server + `top`/`qos` + microbenches.

### Steps

- [x] Structured log levels, per-PD tags, ring=48; shell filters (`dmesg --pd shell`, `dmesg -l warn`).
- [x] Supervisor: richer `ServiceStatus` (ready/degraded/error, last error string) + `status <id>`.
- [x] `lerux bench --check` / `just bench-check` against `support/bench-thresholds.toml`.
- [x] Fault path: `crash dump` line for `lerux diagnose`; production workstation stays lean (ADR-005); optional nesting documented in [`debug.md`](debug.md).
- [x] Host tools: serial always saved under `build/smoke-logs/`; `lerux diagnose`; CI artifact `smoke-serial-*`.

### Exit

A failed boot or hung service is diagnosable from serial + one host command (`lerux diagnose`). **Met.**

---

## Phase 58 — App catalog (Arch: official repos + AUR-shaped ports)

**Why:** Arch is useful because of software; lerux needs deliberate ports, not ports of Linux binaries.

### Priority catalog

Each row = interface types + PD + package fragment + smoke.

| App | Depends on | Notes |
|-----|------------|-------|
| `top` polish / `htop`-like | supervisor | Already partial Phase 40 |
| `fetch` CLI improvements | net v2 + TLS | Progress, content-type, save to FS |
| `http-file-browser` v2 | FS v2 | Upload, MIME, larger listings |
| Calculator / REPL math | shell only | Trivial confidence builder |
| `irc`/`chat` multi-room | net multi-conn | Evolve `chat-client` |
| Scripting runtime PD | FS + net | MicroPython or WAMR **as PD**, IPC only (`plan-au-ts` idea) |
| Backup/sync PD | FS + net | Push tree over TCP |
| Cert/key tool | secrets + FS | For TLS trust anchors |

Defer heavy GUI browsers and language ecosystems until/unless a runtime PD proves viable.

### Exit

≥5 “daily” apps beyond shell builtins, all installable via Phase 55 packaging.

---

## Phase 59 — Multi-arch / multi-profile workstation (Arch: multi-architecture)

**Why:** Arch supports many arches; lerux workstation is essentially aarch64 QEMU (+ RPi4 path).

### Steps

- [ ] `workstation-x86` / `workstation-riscv` profiles using PCI/MMIO virtio and arch-appropriate serial.
- [ ] Shared channel manifests; board-specific drivers only in board vars / layout templates.
- [ ] CI matrix: at least one non-aarch64 workstation smoke (may be serial+fs+shell without full HTTP if cost is high).
- [ ] Document supported platform tiers (Tier 1: aarch64 virt + RPi4; Tier 2: x86/riscv virt; etc.).

### Exit

“Workstation” is a product concept, not a single board name.

---

## Phase 60 — Security posture (Arch: hardening baseline) — stretch

**Why:** seL4 sells isolation; Arch users care about least privilege and updates.

### Steps

- [ ] Threat model doc: which PDs trust which channels; untrusted apps never map DMA (already net policy).
- [ ] Capability audit: reduce shell’s surface; separate admin vs untrusted app profiles.
- [ ] Image signing / measured boot story (host-side first; hardware roots later).
- [ ] Channel/QoS abuse tests; optional MCS budgets if Microkit/seL4 config allows (beyond ADR-006 fixed priorities).
- [ ] Dependency pin hygiene (rust-sel4, Microkit) and security update runbook.

### Exit

Documented trust map + one automated isolation test (e.g. crash in app PD does not take down fs-server).

---

## Deferred stretch (from existing plans)

Fold in as capacity allows; see also [`plan-au-ts.md`](plan-au-ts.md) and ADRs:

- Per-client serial queues / separate TX+RX virt PDs
- Full sDDF net copy-PD swarm
- In-guest GDB RSP (needs fork or upstream APIs)
- libvmm / guest Linux — **only with dedicated ADR** (explicit non-goal today)
- Formal verification of lerux PDs

---

## Completion bar (“about Arch level”)

Treat the system as **done enough** when a developer can:

1. Flash or boot a **profile image** on QEMU and RPi4 without hand-editing XML.
2. Use a **shell** to manage hierarchical storage, config, logs, time, and services.
3. **Fetch** content over the network (DHCP/DNS/TLS path) and edit/save files on disk.
4. **Add/remove/upgrade** PD packages via host CLI with rolling pins and rebuild.
5. Run a small **catalog of apps** (edit, chat, http-fs, …) selected by profile.
6. Diagnose failures via **logs + service status + optional GDB/fault path**.
7. Rely on **CI** (QEMU matrix + optional HW) so regressions match Arch’s “breakage is visible” culture.

That is Arch’s **workflow and completeness**, reimplemented as static Microkit + Rust PDs — not Arch’s ABI.

---

## Near-term priority

If capacity is limited, do **not** start with graphics or scripting runtimes:

1. **Phases 50–55 cores** — FS through package UX done
2. **Phase 52 lab** — fill RPi4 REPL checklist on real hardware when available
3. **Phase 58** — deeper app catalog

---

## Verification (program-level)

| Gate | Command / artifact |
|------|-------------------|
| Host lint | `just check` |
| PD lint | `just check-pd` (needs SDK) |
| Workstation QEMU | `just test-workstation` |
| FS | `just test-fs` / `just test-fs-fat` (+ new multi-sector tests) |
| Net/fetch | `just test-net`, `just test-fetch` (+ TLS/DHCP smokes when added) |
| Packages | `lerux package list|diff`; profile build after install simulation |
| HW | `LERUX_HW_SERIAL=… BOARD=rpi4b_4gb_workstation just test-hw` + REPL checklist |
| Bench (optional) | `just bench` vs `docs/bench-results.latest.md` |
| Docs | Update `docs/plan.md` when a phase completes; keep this file as the living checklist |

Each phase should add or extend **one** profile board smoke rather than only unit tests.

---

## Explicit non-goals

- Unmodified Arch/Linux binaries, pacman on-device, glibc/musl userspace
- Full POSIX VFS / Linux rootfs mount as primary UX
- Desktop environment / GPU stack (unless future product ADR)
- Replacing seL4 or forking Microkit by default
- Vendoring sDDF/LionsOS C trees (`plan-au-ts` principles)

---

## Summary

Phases **1–49** built the **kernel of an Arch-like workflow** (profiles, init, shell, FS/net, packages). Reaching “about Arch level” of **functionality** still needs real storage (50), production networking (51), hardware truth (52), admin UX (53–55), parity and ops (56–57), a deeper app catalog (58), multi-arch workstation (59), and optional hardening (60) — all as **ported Rust PDs and host tooling**, never as a Linux compatibility layer.
