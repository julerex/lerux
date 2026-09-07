# ADR-009: Interactive surface on QEMU (Ladybird-shaped browser PDs + Grok-shaped agent PDs)

## Status

Accepted (Phase 71; phases 72–77 done, 78–80 planned)

## Date

2026-09-05

## Context

Phases 1–70 delivered a QEMU workstation with typed postcard RPC, static Microkit images, and no POSIX ABI. The documented hard ceiling still said **no desktop / GPU** ([`plan-qemu.md`](../plan-qemu.md), [`plan-arch.md`](../plan-arch.md): “do not start with graphics”) and deferred language VMs.

Phases 71–80 want to *move toward*:

1. A **fully-Rust web browser** inspired by [Ladybird](https://github.com/LadybirdBrowser/ladybird) (independent engines, process isolation; not Chromium/WebKit).
2. A **Grok Build clone** inspired by [xai-org/grok-build](https://github.com/xai-org/grok-build) (agent loop, tool taxonomy, serial TUI).

Neither upstream tree is a candidate to vendor or compile for seL4. Ladybird is C++ (with small Rust FFI islands) and spawns processes on demand. Grok Build is a `std`/tokio TUI that spawns subprocesses and sandboxes them with Landlock/Seatbelt. Both fights are the same one [`plan-au-ts.md`](../plan-au-ts.md) already resolved for sDDF: **steal the idea, not the code**.

Constraints that still hold:

- Userspace stays Rust-only. No C++ PDs, no musl, no `fork`/`exec`.
- Microkit **static** PD set — no runtime spawn of extra tabs or MCP children.
- Untrusted apps never map NIC/block/display MMIO (ADR-003 shape).
- Smokes stay deterministic: no public Web and no live xAI in CI (`https-one` / a new `grok-one` stub).
- Local reference trees (`LadybirdBrowser/ladybird`, `xai-org/grok-build`) stay **outside** this repo. Do not copy them into `deps/`.

## Decision

1. **Allow a QEMU software framebuffer.** A `display-server` PD owns the device (ramfb first; virtio-gpu 2D only if ramfb is painful). Apps present via a **shared bitmap memory region** plus postcard `DisplayRequest` — they never map display MMIO. This lifts the “no graphics” ceiling **only** for QEMU software pixels. GPU compositors, Wayland, virtio-gpu 3D, and RPi4 HDMI stay out.

2. **Browser topology is static Ladybird.** Map processes to PDs, do not spawn them:

   | Ladybird | lerux PD (71–80) |
   |----------|------------------|
   | Browser UI | `browser-ui` |
   | WebContent (one per tab) | `web-content` (**one**; no extra tabs) |
   | RequestServer | `request-server` in front of `tls-proxy` / `net-server` |
   | ImageDecoder | deferred (placeholder or later PD) |
   | Compositor (GPU) | skip; `display-server` software blit |

   `web-content` has **no** `NetClient` / `TlsClient` / `FsClient`. It speaks `HttpRequest` to `request-server` and paints into the bitmap MR. That is Ladybird’s load-bearing isolation, expressed as Microkit channels.

3. **Engines are from-scratch Rust subsets** (`lerux-html` + CSS/layout/paint) under `#![no_std]` + `alloc`. Do not embed Servo (std, threads, SpiderMonkey, GPU). Do not compile Ladybird’s workspace. JS / Wasm need a future ADR.

4. **Agent is a PD + serial TUI + tools over existing IPC.** Steal Grok Build’s loop (prompt → tool-calls → tools → model) and the core tool kinds **Read, Edit, Write, ListDir, Search, Execute, WebFetch**. Reimplement them on `FsRequest`, shell `run` (Phase 69), and `request-server`. Isolation **is** the PD set — not Landlock. Do not port `xai-grok-pager`, tokio, ratatui, or MCP stdio servers.

5. **CI talks to stubs.** Browser HTTPS uses the existing smoke CA + `lerux https-one`. Agent completions use a new host `lerux grok-one` with scripted tool-calls. Live xAI is a documented feature flag, not a smoke.

## Alternatives considered

### Embed Servo (or Servo crates) in a PD

- **Pros:** Real CSS/layout; `html5ever` / `cssparser` already exist.
- **Cons:** Servo wants `std`, threads, SpiderMonkey (C++), and a GPU compositor. That is a POSIX desktop in all but name.
- **Rejected** for 71–80. Revisit only with a dedicated ADR if a `no_std` subset of Servo ever exists.

### Port Ladybird C++ under Microkit

- **Pros:** Independent engine that already splits WebContent / RequestServer.
- **Cons:** Violates Rust-only userspace; Qt/AppKit UI; process spawn; curl. The point of lerux is not “run Ladybird on seL4”.
- **Rejected.** Topology and pipeline (parse → cascade → layout → paint) are the steal.

### Lynx-like serial HTML only (keep the graphics ban)

- **Pros:** No framebuffer work; fits phases 1–70 unchanged.
- **Cons:** Does not move toward a graphical browser. Agent TUI can stay serial; the browser cannot if the program is honest.
- **Rejected** as the sole surface. Serial remains the agent UI in 71–80.

### Run host `grok` against the guest (9p + serial)

- **Pros:** Instant agent; no PD work.
- **Cons:** The clone would not *run on lerux*. Sandbox/tools would be Linux’s. Conflicts with “ported as PDs” (Phase 58 catalog).
- **Rejected.** Host `grok` may still be used by developers on the host; it is not the in-guest product.

### libvmm + guest Linux + Chrome / Ladybird / grok

- **Pros:** Real web and a real agent tomorrow.
- **Cons:** Already an explicit non-goal; would need its own ADR and abandons the typed-RPC userspace.
- **Rejected.**

## Consequences

- `alloc` is expected in `display-server`, `web-content`, `request-server`, `browser-ui`, and `agent` (already used by `tls-proxy` / `net-server` / `fs-server`).
- Framebuffer MRs are large (~2 MiB at 800×600×32). Channel numbers still come from the profile manifest (AGENTS.md).
- New PDs sit in the **bulk** QoS band; PPC callees outrank callers (ADR-006). `web-content` must not outrank `request-server` / `display-server` if it PPCs them.
- [`plan-arch.md`](../plan-arch.md) “do not start with graphics” is **superseded for QEMU software framebuffer only**. GPU, JS, POSIX, and libvmm remain forbidden.
- [`security.md`](../security.md) gains a planned trust row: `web-content` and `agent` are untrusted; `request-server` and `display-server` are trusted services.
- Living checklist: [`plan-interactive.md`](../plan-interactive.md).

## References

- [`plan-interactive.md`](../plan-interactive.md) (phases 71–80)
- [ADR-003](003-net-virtualiser.md) (apps never map NIC DMA)
- [ADR-007](007-tls-proxy.md) (`tls-proxy` owns rustls)
- [ADR-006](006-workstation-qos.md) (PPC vs priority)
- Ladybird `Documentation/ProcessArchitecture.md` (local: `~/repos/github_orgs/LadybirdBrowser/ladybird`)
- Grok Build `crates/codegen/xai-grok-agent`, `xai-grok-tools`, `xai-grok-pager` (local: `/home/julian/repos/github_orgs/xai-org/grok-build`)
