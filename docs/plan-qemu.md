# PLAN — QEMU-only workstation deepening (phases 61–70)

Last updated: 2026-08-16

Related: [`plan.md`](plan.md) (roadmap 1–70), [`plan-arch.md`](plan-arch.md) (phases 50–60 + Physical RPi4 lab), [`plan-au-ts.md`](plan-au-ts.md) (sDDF inspiration), [`context.md`](context.md).

## Context

Phases 1–60 delivered an Arch-like **workflow** on QEMU: supervisor, hierarchical FS, DHCP/DNS/TLS fetch, shell, profiles/packages, serial/net virtualisers, QoS, debug, benches, and a three-arch workstation. Remaining hardware work lives only in [Physical RPi4 lab](plan-arch.md#physical-rpi4-lab-hardware-gated) and **does not block this program**.

This plan is the next ten phases. Every deliverable is **completable on QEMU virt** (aarch64, and riscv/x86 where the phase says so). No board on the desk, no GENET, no eMMC, no JTAG.

**What still feels toy on QEMU today**

| Gap | Today | Why it blocks daily QEMU use |
|-----|-------|------------------------------|
| DMA trust map | unified-dma on aarch64 virtio-net only | x86 (and possibly RISC-V) still map a distinct client DMA region (ADR-003 residual) |
| File size / paths | 16 KiB files, 48-byte paths | Edit buffers, logs, and scripts hit an artificial cap |
| Host ↔ guest files | Format a `disk.img`, boot, hope | No virtfs/NFS path to edit on the host and read in the guest |
| Net concurrency | Single-flight `NetRequest` | `fetch` while http-fs `TcpRecv` is pending returns `Pending` |
| Serial virt | Workstation mux only; combined driver elsewhere | Per-client queues and non-workstation QEMU boards still coupled to the UART PD |
| Arch parity | Debug + isolation are aarch64-only | RISC-V/x86 workstation exists, but fault/isolation smokes do not |
| Image integrity | SHA-256 sidecar | No asymmetric signature; `verify-image` cannot catch a swapped keyless image |
| TLS | Smoke CA only | `webpki-roots` and a cert store are stretch |
| In-guest automation | Interactive REPL + host-scripted serial | No on-disk batch of shell commands |
| Inner loop | Cold boot every smoke | No snapshot path; benches are aarch64-only; FAT workstation demo still open |

### Hard ceiling (unchanged)

- No Linux/POSIX ABI, musl, `fork`/`exec`, unmodified third-party binaries
- Microkit **static** PD set — install still means pin + rebuild `loader.img`
- No desktop / GPU, no libvmm / guest Linux (needs a dedicated ADR)
- No MCS budgets (ADR-006 still defers them)
- No measured boot / TPM / fuse (needs hardware; signing stays host-side)
- GENET / eMMC / on-device REPL stay in the RPi4 lab, not here

---

## Approach

Work in **vertical QEMU slices**, each ending in a `just test-*` (or host CLI) gate. Prefer deepening existing IPC (`lerux-interface-types`) and QEMU boards over new hardware profiles.

```
Leftover stretch              Storage                         Trust + automation           Inner loop
────────────────              ───────                         ──────────────────           ──────────
61 QEMU DMA parity
62 FS v3 ───────────────────► 63 Host-backed FS
64 Net multi-client queue
65 Serial virt v2 ──────────► 66 QEMU arch parity
                                                          67 Image signing
                                                          68 TLS roots + certs
                                                          69 Batch runner
                                                                                       70 QEMU developer loop
```

Fold in leftover stretch from 42 / 43 / 50 / 51 / 58 / 60 rather than inventing a parallel backlog.

### Reuse map

| Area | Paths |
|------|--------|
| DMA / net trust | [`net-topology.md`](net-topology.md), [ADR-003](decisions/003-net-virtualiser.md), `virtio-pci-driver`, `virtio-net-driver` |
| FS format + IPC | `lerux-fs`, `lerux-fat`, `fs-server`, `FsRequest` |
| Serial virt | [ADR-002](decisions/002-serial-virtualiser.md), `serial-virt`, `lerux-serial-queue` |
| TLS | [ADR-007](decisions/007-tls-proxy.md), `tls-proxy` |
| Image integrity | [`security.md`](security.md#image-integrity-track-c) |
| Debug | [`debug.md`](debug.md), [ADR-005](decisions/005-debug-pd.md) |
| Benches / ops | [`bench.md`](bench.md), [`ops.md`](ops.md) |
| QEMU boards | [`boards.md`](boards.md), `support/boards.toml` |

---

## Phase 61 — QEMU DMA parity (x86 PCI + RISC-V virtio)

**Why:** ADR-003’s trust map (“driver owns one DMA MR; apps never map NIC DMA”) is only true on aarch64 virtio-net. x86 PCI still has a distinct client-DMA region. RISC-V MMIO virtio should match aarch64. Both are QEMU.

### Steps

- [x] Port `unified-dma` to `virtio-pci-driver` + `net-server` on q35 (Hal + bounce in one MR; drop `virtio_net_client_dma` from x86 net / http / virtio / workstation templates).
- [x] RISC-V virtio-net was still split; ported the same Hal/bounce split as aarch64.
- [x] Update [`net-topology.md`](net-topology.md) and ADR-003 residual notes: QEMU arches are done; GENET stays lab-only.
- [x] Host gate: `lerux-cli` test `qemu_net_boards_have_no_distinct_client_dma_mr`. Guest smokes: `just test-x86-net`, `just test-x86-http`, `just test-riscv-net` (and workstation variants in CI).

### Out of scope

- `genet-driver` unified-dma ([Physical RPi4 lab](plan-arch.md#physical-rpi4-lab-hardware-gated))
- Separate Rx/Tx virt PDs / copy-PD swarm (still deferred)

### Exit

Every QEMU net board used in CI has **no** distinct client-DMA map in the NIC driver SDF. Apps still speak `NetRequest` only. Trust map in [`security.md`](security.md) no longer says “aarch64 virtio-net only”.

---

## Phase 62 — Filesystem v3 (usable size)

**Why:** LERUXFS2 / FAT cap files at 16 KiB and paths at 48 bytes. That is enough for config keys, not for edit buffers, rotated logs, backups, or the batch files in Phase 69.

### Steps

- [x] On-disk: contiguous extent cap **512 sectors / 256 KiB**; in-place `try_extend_contiguous` so grow is not always copy+realloc.
- [x] Keep chunked `Read`/`Write` (`MAX_FS_DATA`); do not grow a single IPC message to the file cap.
- [x] Path grammar: `MAX_FS_PATH` is 128; postcard still prefixes `path_len` so shorter clients stay valid.
- [x] FAT remains the 16 KiB small-file alternate (documented).
- [x] Smokes: `just test-fs` writes/reads 20 KiB (`lerux-fs: v3 20k ok`) and a path > 48 bytes.

### Out of scope

- POSIX VFS, journaling, fsck-as-a-product
- Host-backed / 9p (Phase 63)
- Changing `FsRequest` verbs (`Open`/`Read`/`Write`/…)

### Exit

A developer can store a real edit buffer and a multi-page `/boot.log` without hitting 16 KiB. Path strings are long enough for `/config/secrets/…` plus one extra directory level.

---

## Phase 63 — Host-backed FS (QEMU virtfs / 9p)

**Why:** LionsOS-style NFS and Phase 50’s open “host-backed FS” item. On QEMU you should edit a file on the host and `cat` it in the guest without reformatting `disk.img`.

### Steps

- [x] ADR: [ADR-008](decisions/008-host-backed-fs.md) — **disk inject** for v1 (`lerux fs-host seed`); virtio-9p / NFS deferred.
- [x] Host files land in LERUXFS2 `/host/` so shell / edit stay on `FsRequest`.
- [x] Guest path `/host/…` next to the rest of the volume (not a Linux rootfs).
- [x] Board `qemu_virt_aarch64_fs_host`; `just test-fs-host`.
- [x] Smoke: host writes `build/fs-host/hello.txt`, guest reads it (`lerux-fs: host hello ok`).

### Out of scope

- Mounting a Linux rootfs as the primary UX
- Sharing the host’s `/` or home by default in CI (use a scratch directory)
- RPi4 (no virtio-9p on the Pi path)

### Exit

`just test-fs-host` proves host → guest file visibility through existing FS IPC. Workstation may keep LERUXFS2 as root; 9p is the QEMU dev convenience board (optional workstation wire is Phase 70).

---

## Phase 64 — Multi-client net queue

**Why:** Phase 51 stretch. `net-server` is single-flight: shell `fetch` while `http-file-browser` holds `TcpRecv` yields `Pending`. Daily QEMU use is exactly that overlap (browse + fetch, chat + http-fs).

### Steps

- [ ] Per-client in-flight slot (or a small FIFO) in `net-server` so two `NetRequest` streams can progress. Reuse `lerux-service-async` if a second `SingleTask` is enough; otherwise an explicit queue.
- [ ] Preserve postcard `Poll` RPC and static-map DNS / DHCP behaviour.
- [ ] Smoke: overlapping `fetch` + inbound HTTP on one QEMU net board without a client seeing `Pending` for the other’s op (`just test-net-concurrent` or an extended `just test-workstation` expect).
- [ ] Docs: [`net-topology.md`](net-topology.md) notes multi-flight; QoS still uses priorities, not “one RPC at a time”, as the throttle.

### Out of scope

- Full sDDF copy-PD swarm
- Unlimited sockets / a POSIX `select` API
- MCS (still ADR-006 deferred)

### Exit

Two net clients on QEMU complete overlapping ops. Workstation boot with http-fs + a fetch-shaped probe does not serialize on `Pending`.

---

## Phase 65 — Serial virtualiser v2

**Why:** ADR-002 leftover. Workstation has `serial-driver` (device-only) + `serial-virt`, but queues are shared and echo/composed QEMU boards still use the combined multi-client driver.

### Steps

- [ ] Per-client TX SPSC queues in `lerux-serial-queue` (RX per-client if it stays cheap; otherwise document a shared RX + client filter).
- [ ] Keep the postcard `SerialClient` wire format so shell / log-server / supervisor do not change.
- [ ] Migrate at least one **non-workstation** QEMU board (echo or composed) to `serial-virt`.
- [ ] Optional: separate TX/RX virt PDs only if the extra PD pays for itself on QEMU; otherwise record the no in the ADR residual.
- [ ] Smokes: `just test-workstation`, plus the migrated echo/composed board.

### Out of scope

- Changing `LogRequest` / shell line protocol
- Porting C sDDF serial
- Hardware UART policy (RPi4 already follows workstation virt)

### Exit

Workstation serial mux is per-client on TX. At least one smaller QEMU board uses the same virt topology. Combined multi-client driver is the exception, not the default for new QEMU layouts.

---

## Phase 66 — QEMU arch parity (debug, isolation, serial-virt)

**Why:** Phase 59 made “workstation” a three-arch product. Debug (`just test-debug`) and isolation (`just test-isolation`) are still aarch64-only. Serial-virt on riscv/x86 workstation should be verified, not assumed.

### Steps

- [ ] `debug-handler` + `crash-demo` on `qemu_virt_riscv64_debug` and `x86_64_generic_debug`; `just test-debug-riscv` / `just test-debug-x86`.
- [ ] Isolation smoke on RISC-V and x86 (crash child, then FS or echo still serves). New boards or reuse workstation-minus-apps if PD count hurts.
- [ ] Confirm `workstation-riscv` / `workstation-x86` are on serial-virt (Phase 59 claimed it); add expects if missing.
- [ ] virtio-rng (or equivalent QEMU entropy source) on at least aarch64 workstation if TLS/signing still starve for randomness — small PD or a `rng-driver` behind a tiny IPC.
- [ ] Update the Phase 14 parity table: debug + isolation become **yes** on all three QEMU arches. Treat legacy `composed` as superseded by workstation where the capability already exists.

### Out of scope

- In-guest GDB RSP / libgdb (still needs a forked kernel or new upstream APIs)
- Hardware GDB / OpenOCD
- Recreating every aarch64 *composed* board on riscv/x86

### Exit

`just test-debug{,-riscv,-x86}` and isolation smokes exist for all three QEMU arches. [`debug.md`](debug.md) documents the gdbstub flags per arch.

---

## Phase 67 — Asymmetric image signing

**Why:** Phase 60 Track C ships SHA-256 sidecars. That catches accidental corruption, not a swapped `loader.img`. Host ed25519 signing is QEMU-completable; measured boot is not.

### Steps

- [ ] Host keygen + sign: `lerux keygen` / `lerux sign --board <board>` writes `loader.img.sig` next to the digest.
- [ ] `lerux verify-image` checks digest **and** signature when a key is configured; `lerux run` / `lerux test` can `--require-sig` for QEMU boards.
- [ ] Smoke key in-tree for CI (not a production secret); document the production-key ritual in [`security.md`](security.md).
- [ ] CI: image job signs with the smoke key; at least one QEMU smoke verifies before boot.

### Out of scope

- Measured boot, TPM, fuse OTP, in-guest verification of the loader
- cosign / Sigstore (ed25519 + a well-known pubkey file is enough)
- RPi4 secure-boot ROM

### Exit

A QEMU board refuses to be the smoke image if the signature is missing or wrong (`--require-sig`). Digest-only verify remains the default for unsigned trees.

---

## Phase 68 — TLS roots and cert tool

**Why:** Phase 51 left `webpki-roots` as an optional crate feature; Phase 58 deferred a cert/key tool. Interactive QEMU fetch to a real name needs a trust store; smokes must stay deterministic.

### Steps

- [ ] `webpki-roots` feature on `tls-proxy` for interactive `lerux run` workstation; **smokes stay on the smoke CA**.
- [ ] Cert store under `/config/certs/` (or `secret.*`-adjacent keys) via config-server; shell `cert list|show|trust`.
- [ ] Small `cert` built-in (prefer shell over a new PD unless IPC types demand it).
- [ ] Optional virtio-rng wire if Phase 66 did not already provide entropy for rustls.
- [ ] Docs: how to enable webpki-roots for a laptop QEMU session; CI remains `just test-fetch-tls` + smoke CA.

### Out of scope

- ACME / Let’s Encrypt client
- Mutual TLS as a product
- Replacing ADR-007’s smoke-CA path

### Exit

An operator can inspect and add a trust anchor from the shell. `just test-fetch-tls` is unchanged. Interactive QEMU TLS to the public Web is a documented feature flag, not a hope.

---

## Phase 69 — Batch runner (on-disk shell scripts)

**Why:** Phase 58 deferred a “scripting runtime PD”. A language ecosystem is still a non-goal. What QEMU development actually needs is **a file of shell lines that runs non-interactively** — first-boot recipes, reproducible smokes, demo scripts.

### Steps

- [ ] Shell built-in `source <path>` / `run <path>`: read a LERUXFS2 (or 9p) text file, execute lines, stop on first error unless `-k`.
- [ ] No new language: same built-ins as the REPL (`ls`, `mkdir`, `config`, `fetch`, …). Comments `#…`; ignore blank lines.
- [ ] Cap: Phase 62 file size is the script cap; no heap-grown interpreter.
- [ ] Smoke: seed `/batch/smoke.lerux` (or host-backed equivalent) and expect a final `lerux-shell: batch ok` (`just test-batch` or a workstation script expect).
- [ ] Optional: supervisor can run `/batch/first-boot.lerux` after seed if the file exists.

### Out of scope

- MicroPython, WAMR, Rhai, Lua, or any bytecode VM (would need its own ADR)
- Job control, pipes, redirects, `if`/`for` beyond “run these lines”
- Runtime ELF load

### Exit

A QEMU smoke proves an on-disk script ran to completion through the real shell. Host-scripted serial (`script = [{send, expect}]`) remains for hardware; batch is the in-guest path.

---

## Phase 70 — QEMU developer loop

**Why:** All of 61–69 is only useful if the inner loop stays on QEMU. Today every smoke is a cold boot, benches are aarch64-only, and the FAT workstation demo from Phase 44/50 is still open.

### Steps

- [ ] Snapshot path: `lerux test --snapshot` or a documented `savevm`/`loadvm` profile so a second smoke can skip loader + kernel bring-up when the image hash is unchanged. Best-effort; cold boot remains the CI default.
- [ ] First-class QEMU flags on `lerux run`: gdbstub, monitor, virtfs (Phase 63), `--require-sig` (Phase 67).
- [ ] Cross-arch benches: `just bench` (or `just bench-riscv` / `just bench-x86`) writes the same markdown/JSON shape; thresholds stay aarch64-only unless numbers are stable.
- [ ] Optional FAT workstation: `just test-workstation-fat` (or a profile fragment) so the Phase 44 backend is not fs-board-only.
- [ ] Optional: wire Phase 63 `/host` into `dev-workstation` for laptop use (not CI, unless the scratch dir is isolated).
- [ ] Short doc: “QEMU-only developer workflow” in [`boards.md`](boards.md) or [`ops.md`](ops.md) — build, run, snapshot, gdb, diagnose, batch.

### Out of scope

- Replacing `just` with Nix
- QEMU record/replay as a correctness story (nice extra, not the exit)
- Speeding up `microkit` / rustc themselves

### Exit

A developer can work **only on QEMU** with: a documented inner loop, at least one path faster than a full cold boot, benches on more than one arch (or an explicit “aarch64-only because …” note), and FAT visible on a workstation-shaped board.

---

## Completion bar (QEMU-only program)

Treat 61–70 as done when a developer with **no hardware** can:

1. Trust the same DMA map on aarch64, RISC-V, and x86 QEMU net boards.
2. Store files larger than 16 KiB and paths deeper than 48 bytes.
3. Edit a file on the host and read it in the guest (9p/virtfs).
4. Run overlapping net clients without `Pending`.
5. Use per-client serial virt on workstation (and at least one smaller QEMU board).
6. Hit debug + isolation smokes on all three QEMU arches.
7. Sign and verify `loader.img` with ed25519 before a QEMU boot.
8. Inspect a TLS trust store; keep CI on the smoke CA.
9. Run an on-disk batch of shell commands in a smoke.
10. Follow a written QEMU inner loop that is not “cold-boot `just test-workstation` every time”.

Hardware truth remains [Physical RPi4 lab](plan-arch.md#physical-rpi4-lab-hardware-gated).

---

## Near-term priority

If capacity is limited, do **not** start with 68–70 first:

1. **Phase 61** — x86 unified-dma (documented leftover; unblocks the trust map).
2. **Phase 62** — FS v3 (unblocks 63 and 69).
3. **Phase 64** — net queue (unblocks realistic workstation overlap).

63, 65, 66, 67 can proceed in parallel after 61 once the DMA templates are stable. 68 needs 66’s entropy if rustls is starved. 69 needs 62. 70 last.

RPi4 lab work never blocks this list.

---

## Verification (program-level)

| Gate | Command / artifact |
|------|-------------------|
| Host lint | `just check` |
| PD lint | `just check-pd` |
| DMA parity | `just test-x86-net`, `just test-x86-http`, `just test-workstation-x86` (+ RISC-V net if MRs change) |
| FS v3 | `just test-fs` (file > 16 KiB) |
| Host FS | `just test-fs-host` (new) |
| Net queue | `just test-net` + concurrent/workstation expect |
| Serial virt v2 | `just test-workstation` + migrated echo/composed |
| Arch parity | `just test-debug{,-riscv,-x86}`, isolation on three arches |
| Signing | `lerux verify-image` with sig; one QEMU smoke `--require-sig` |
| TLS / certs | `just test-fetch-tls` (smoke CA unchanged) |
| Batch | `just test-batch` or workstation `batch ok` |
| Inner loop | documented `lerux run` flags; optional `--snapshot`; `just bench` multi-arch |
| Docs | Update `docs/plan.md` when a phase completes; this file is the living checklist |

Each phase adds or extends **one** QEMU smoke (or one host CLI gate for 67) rather than only unit tests.

---

## Explicit non-goals

Same as [`plan-arch.md`](plan-arch.md) and [`context.md`](context.md), plus:

- Any work that **requires** a physical board to close
- MCS, graphics, POSIX, libvmm, in-guest GDB RSP, formal verification of lerux PDs
- Replacing LERUXFS2 with a Linux filesystem as the default root

---

## Summary

Phases **61–70** make QEMU the place you **develop and use** the workstation that 50–60 defined: close ADR-003 on every virt machine, grow storage past toy caps, share host files, overlap net clients, finish serial virt, spread debug/isolation across arches, sign images, make TLS configurable, run on-disk batches, and shorten the inner loop. None of it waits on a Pi.
