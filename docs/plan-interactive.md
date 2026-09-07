# PLAN — Interactive surface (phases 71–80)

Last updated: 2026-09-07 (phases 71–80; Phase 77 agent runtime + grok-one)

Related: [`plan.md`](plan.md) (roadmap 1–80), [`plan-qemu.md`](plan-qemu.md) (phases 61–70, done), [`plan-arch.md`](plan-arch.md) (phases 50–60 + Physical RPi4 lab), [`plan-au-ts.md`](plan-au-ts.md) (sDDF inspiration), [`context.md`](context.md), [ADR-009](decisions/009-interactive-surface.md).

## Context

Phases 1–70 delivered an Arch-like **workflow** on QEMU and forbade graphics. This program *moves toward* two products that need an interactive surface:

1. A **fully-Rust web browser** inspired by [Ladybird](https://github.com/LadybirdBrowser/ladybird) — independent engines and process isolation, not Chromium/WebKit, not Ladybird C++.
2. A **Grok Build clone** inspired by [xai-org/grok-build](https://github.com/xai-org/grok-build) — agent loop, tool taxonomy, serial TUI, not the host `grok` binary.

Same rule as au-ts: **steal the idea, not the code**. Local reference trees stay outside this repo:

- Ladybird: `/home/julian/repos/github_orgs/LadybirdBrowser/ladybird`
- Grok Build: `/home/julian/repos/github_orgs/xai-org/grok-build`

Every deliverable is **completable on QEMU virt aarch64**. No board on the desk, no HDMI, no GENET. RISC-V/x86 display parity is later, not this decade.

**What still feels toy for an interactive surface today**

| Gap | Today | Why it blocks browser / agent |
|-----|-------|-------------------------------|
| Pixels | Serial only | Ladybird WebContent paints a shared bitmap; nothing presents it |
| HTTP isolation | Apps may hold `NetClient` / `TlsClient` | Ladybird WebContent must not speak TCP; neither should `web-content` |
| HTML/CSS | `http-file-browser` emits markup, nobody lays it out | Need parse → cascade → layout → paint |
| Agent loop | `chat-client` is UDP rooms | Grok Build is prompt → tool-calls → tools → model |
| Tools | Shell builtins + `edit` | Need Read/Edit/ListDir/Execute/WebFetch as one PD |
| LLM in CI | None | Live xAI is non-deterministic; need `grok-one` like `https-one` |
| Joint profile | Workstation is serial REPL | Prove browser + agent compose without giving untrusted PDs a NIC map |

### Hard ceiling (ADR-009)

Lifted: **QEMU software framebuffer** owned by `display-server`. Apps present via a shared bitmap MR.

Still forbidden:

- Linux/POSIX ABI, musl, `fork`/`exec`, unmodified third-party binaries (including `grok` and Ladybird)
- Microkit **static** PD set — one `web-content`, no extra tabs at runtime
- JS / Wasm engines (needs a future ADR)
- GPU compositor, Wayland, virtio-gpu 3D, libvmm / guest Linux
- MCP stdio servers, Landlock/Seatbelt, live xAI in CI
- Vendoring Ladybird, Servo, or grok-build into `deps/`
- GENET / eMMC / HDMI / on-device REPL — [Physical RPi4 lab](plan-arch.md#physical-rpi4-lab-hardware-gated)

---

## Approach

Work in **vertical QEMU slices**, each ending in a `just test-*` (or host CLI) gate. Prefer new postcard types in `lerux-interface-types` and new PDs over POSIX shims.

```
Platform                         Browser (Ladybird-shaped)              Agent (Grok-shaped)
────────                         ─────────────────────────              ───────────────────
71 ADR + domain language ✅
72 display + input (QEMU ramfb) ✅
                                 73 request-server PD ✅
                                 74 HTML/DOM (`lerux-html`) ✅
                                 75 CSS + layout + paint ✅
                                 76 browser-ui + one web-content ✅
                                                                    77 agent runtime + grok-one ✅
                                                                    78 serial TUI
                                                                    79 tools (fs / shell / fetch)
80 joint profile: workstation-interactive
```

### Ladybird → lerux PD map

From Ladybird `Documentation/ProcessArchitecture.md` and `Services/`:

| Ladybird | Job | lerux 71–80 |
|----------|-----|-------------|
| Browser UI (Qt/AppKit) | chrome, input, present pixels | `browser-ui` |
| WebContent | LibWeb + LibJS; paint to shared bitmap | `web-content` (**one**) |
| RequestServer | HTTP(S)/WS/DNS; WebContent has no NIC | `request-server` in front of `tls-proxy` / `net-server` |
| ImageDecoder | decode one image in a fresh process | deferred |
| Compositor | GPU/OpenGL present | skip; `display-server` software blit |
| WasmCompiler / WebWorker | later | out of 71–80 |

`web-content` **cannot** use the network or filesystem except through `request-server`. That is ADR-003/007 with a Ladybird name.

Spawn-on-demand (`/tmp/session/…/portal/webcontent`) does not exist on Microkit.

### Grok Build → lerux PD map

From `crates/codegen/` (`xai-grok-agent`, `xai-grok-tools`, `xai-grok-pager`, `xai-grok-sandbox`):

| Grok Build | Needs on Linux | lerux 71–80 |
|------------|----------------|-------------|
| Agent loop | tokio, files, YAML agents | `agent` PD, postcard `AgentRequest` |
| Tools | process spawn, reqwest, ripgrep, Landlock | IPC to `fs-server`, shell `run`, `request-server` |
| TUI | ratatui, PTY, mouse | serial ANSI fullscreen |
| Sandbox | Landlock / Seatbelt (`nono`) | **PD isolation is the sandbox** |
| MCP / subagents / LSP | stdio children | out of 71–80 |

Core tool kinds: **Read, Edit, Write, ListDir, Search, Execute, WebFetch**.

### Reuse map

| Area | Paths |
|------|--------|
| DMA / net trust | [`net-topology.md`](net-topology.md), [ADR-003](decisions/003-net-virtualiser.md), `net-server`, `tls-proxy` |
| TLS / HTTPS smoke | [ADR-007](decisions/007-tls-proxy.md), `lerux https-one`, smoke CA |
| FS + host inject | `lerux-fs`, `fs-server`, `FsRequest`, [ADR-008](decisions/008-host-backed-fs.md) |
| Serial virt | [ADR-002](decisions/002-serial-virtualiser.md), `serial-virt` |
| Shell batch | Phase 69 `run` / `source` |
| QoS | [`qos.md`](qos.md), [ADR-006](decisions/006-workstation-qos.md) |
| Packages / profiles | [`packages.md`](packages.md), `support/profiles/` |
| Interactive surface | [ADR-009](decisions/009-interactive-surface.md) (this program) |

Channel numbers come from the profile `[[channel]]` manifest, not from this document.

---

## Phase 71 — ADR + domain language ✅

**Why:** `plan-arch.md` and `plan-qemu.md` forbade graphics. Starting PDs without an ADR repeats the sdfgen vs in-tree fight (ADR-001).

### Steps

- [x] [ADR-009](decisions/009-interactive-surface.md): QEMU software framebuffer; static Ladybird PD topology; Grok-shaped agent as PDs; remaining ceilings.
- [x] Glossary in [`context.md`](context.md) (`display-server`, framebuffer MR, `browser-ui`, `web-content`, `request-server`, `agent`).
- [x] Remaining ceilings listed above.

### Out of scope

- Guest code, boards, CI jobs.

### Exit

A future agent can implement Phase 72 without re-litigating “are we allowed to have pixels?”. **Met.**

---

## Phase 72 — Display + input (QEMU) ✅

**Why:** Ladybird WebContent paints to a shared bitmap; something must present it. The agent TUI can stay serial; the browser cannot.

### Steps

- [x] QEMU **ramfb** (`-device ramfb`, configured through fw_cfg DMA). virtio-gpu 2D not needed.
- [x] `display-server` PD owns fw_cfg + the ramfb backing MR. Apps never map display MMIO (ADR-003 shape).
- [x] Shared bitmap MR: producer (`display-demo` this phase; `web-content` later) → `display-server` blit.
- [x] Postcard `DisplayRequest` / `DisplayResponse` (`GetMode` / `Present` / `PollInput`) in `lerux-interface-types`.
- [x] Input v1: serial keys as `InputEvent` via `PollInput` (not virtio-input).
- [x] Board `qemu_virt_aarch64_display`. Smoke `just test-display`: colour-bar pattern; expect `lerux-display: pattern ok`. `lerux run` opens a GTK window when `DISPLAY` is set (`LERUX_QEMU_GRAPHIC=0` forces headless). Host PPM dump is still optional (QEMU `screendump`).

### Out of scope

- GPU, vsync, cursor sprite, virtio-tablet, RPi4 HDMI.
- Putting chrome on the framebuffer (serial chrome in Phase 76 is enough).

### Exit

QEMU window shows a non-serial pixel buffer produced by a PD (`just run` with a display). CI checks the log line. **Met.**

---

## Phase 73 — Request-server PD ✅

**Why:** Ladybird’s load-bearing isolation: WebContent never speaks TCP. lerux already has `tls-proxy` + `net-server`; untrusted browser/agent PDs still must not own that path.

### Steps

- [x] `request-server` is the only HTTP client of `tls-proxy` / `net-server` on the new boards.
- [x] `HttpRequest` / `HttpResponse` in `lerux-interface-types` (method, URL, headers, chunked body; reuse `MAX_NET_TCP_PAYLOAD` chunking).
- [x] Smoke client `request-client` calls `request-server`; it does **not** get `NetClient` / `TlsClient` channels (`web-content` / `agent` later).
- [x] Smoke `just test-request`: GET `https://host:8443/fixture.html` from `lerux https-one`; expect `lerux-http: fixture ok`.

### Out of scope

- Cookies, HTTP cache, WebSocket, HTTP/2, public Web in CI.
- Replacing `fetch-client` on existing workstation boards (it may keep `TlsClient` until a later cutover).

### Exit

An untrusted PD can GET `https://host/…` without mapping NIC DMA or linking rustls. **Met.**

---

## Phase 74 — HTML + DOM (`lerux-html`) ✅

**Why:** LibWeb’s pipeline starts at tokenize → tree builder. We need a `no_std`+`alloc` library, not html5ever/`std`, and not Ladybird’s `libweb_html_tokenizer` (cbindgen + `AK/Rust`).

### Steps

- [x] Shared crate `lerux-html`: tokenizer + tree builder for a **subset** (`html`, `head`, `body`, `title`, `p`, `h1`–`h3`, `a`, `div`, `span`, `ul`/`ol`/`li`, `pre`, `code`, `strong`, `em`, `img` stub, `style` as text).
- [x] Host unit tests on committed fixtures under `support/browser/`.
- [x] PD dump: `just test-html` logs `lerux-html: nodes=8` for the baked-in smoke fixture (`include_str`, not `/host`).

### Out of scope

- `document.write`, SVG, MathML, innerHTML script injection, spec-complete parser.
- Importing Ladybird or Servo parser crates.

### Exit

A fixture parses to a walkable DOM in a PD. **Met.**

---

## Phase 75 — CSS subset + layout + paint ✅

**Why:** Ladybird’s “loading to painting” after parse is CSS → cascade → layout → paint into a bitmap.

### Steps

- [x] CSS subset: `color`, `background-color`, `font-size`, `display: block|inline`, `margin`, `padding`, `width`. Author `<style>` + a tiny UA sheet.
- [x] Block-flow layout only (no flex, grid, floats, positioning).
- [x] Paint RGB888 into the shared bitmap MR (`lerux-web` crate, `paint-demo` PD).
- [x] Host: fixture → PPM (`Bitmap::encode_ppm`) + signature pixels. Guest: `just test-paint` expect `lerux-web: paint ok`.

### Out of scope

- JS-driven style, webfonts, images beyond a solid-color `img` placeholder, GPU.

### Exit

`display-server` presents pixels that correspond to the HTML+CSS fixture (human-checkable in QEMU; CI checks the log line + host signature/PPM). **Met.**

---

## Phase 76 — `browser-ui` + one `web-content` ✅

**Why:** Ladybird’s Browser process owns chrome and input; WebContent owns the engine. Keep that split even with one tab.

### Steps

- [x] `browser-ui`: URL line (serial `open <url>` v1), owns the channel to `web-content`, forwards present to `display-server`.
- [x] `web-content`: parse/layout/paint; **only** `HttpRequest` to `request-server` + bitmap MR. No FS, no NIC.
- [x] Navigate `https://host:8443/paint.html` via `request-server` (smoke CA). First load is implicit; serial `open <url>` remains available.
- [x] Profile `browser` / board `qemu_virt_aarch64_browser`. `just test-browser`.

### Out of scope

- Tabs, history, bookmarks, chrome painted on the framebuffer (serial chrome is enough).
- JS.

### Exit

`just test-browser` loads the fixture through `request-server` and paints it. **Met.**

---

## Phase 77 — Agent runtime + `grok-one` stub ✅

**Why:** Grok Build’s host is `xai-grok-agent`: tools + system prompt + model config. CI cannot hit live xAI (same reason `https-one` exists).

### Steps

- [x] Host `lerux grok-one`: HTTP server that returns scripted tool-calls / final text for a tiny prompt set.
- [x] `agent` PD: `AgentRequest::Prompt` / `AgentResponse::{Text, ToolCall, Done, Error}`. Completions via `request-server` → `grok-one`.
- [x] Smoke CA path only. Reserve `secret.grok.*` in the config schema; unused in CI.
- [x] `just test-agent-runtime`: prompt “read /hello.txt” → stub emits Read → agent returns file body (baked-in `/hello.txt` until Phase 79 FS tools).

### Out of scope

- Streaming tokens as a product, compaction, subagents, real xAI in CI, browser OAuth.

### Exit

One tool-loop round-trip in QEMU against a stub. **Met.**

---

## Phase 78 — Agent serial TUI

**Why:** Grok Build’s pager is the product surface. lerux already owns serial virt; start there, not on the framebuffer.

### Steps

- [ ] Fullscreen ANSI: transcript, status, prompt. Inspired by grok-pager layout, not a ratatui port.
- [ ] Shell `grok` PPC to `agent` (same pattern as `edit` / `chat`).
- [ ] Scripted serial smoke: expect chrome + a stub reply.

### Out of scope

- Mouse, PTY multiplexer, theming engine, ACP.
- Headless JSON as a product (a log line is enough for CI).

### Exit

A human can type a prompt on the workstation serial and see the stub’s reply in a TUI.

---

## Phase 79 — Agent tools

**Why:** Grok Build’s useful core is tools, not the TUI. Map taxonomy onto existing IPC.

### Steps

- [ ] Read / Write / Edit / ListDir / Search (linear scan; not ripgrep) via `FsRequest`.
- [ ] Execute: run an on-disk batch through shell `run` (Phase 69). **No** `fork`/`exec`.
- [ ] WebFetch via `request-server` (not `NetClient`).
- [ ] Workspace = a directory on LERUXFS2 (seeded `/host` or `/work`). File cap remains 256 KiB; tools chunk.
- [ ] `just test-agent`: stub drives Read + Edit + WebFetch; expect `lerux-agent: tools ok`.

### Out of scope

- MCP, LSP, image/video gen, scheduler, `bash` pipelines, Landlock.

### Exit

The agent can change a file and fetch a URL in a QEMU smoke without a POSIX process.

---

## Phase 80 — Joint `workstation-interactive`

**Why:** The two products share `request-server` + display. Prove they compose, then stop.

### Steps

- [ ] Profile `workstation-interactive`: workstation + `display-server` + `request-server` + `web-content` + `browser-ui` + `agent`.
- [ ] Agent tool `browse`: ask `web-content` to load a URL; return extracted text (and optionally “paint ok”).
- [ ] Trust map in [`security.md`](security.md): `web-content` and `agent` are untrusted; `request-server` / `display-server` / fs / net stay trusted.
- [ ] `just test-interactive`. QoS: new PDs in the bulk band; PPC callees outrank callers (ADR-006).
- [ ] Update [`packages.md`](packages.md) / [`boards.md`](boards.md) when boards exist.

### Out of scope

- Making this the default workstation.
- RPi4 HDMI; multi-arch display.

### Exit

One QEMU profile where the agent fetches a local page that the browser also paints.

---

## Completion bar (interactive-surface program)

Treat 71–80 as done when a developer with **no hardware** can:

1. Boot a QEMU board that owns a software framebuffer (`display-server`).
2. Load a committed HTML+CSS fixture through `request-server` into `web-content` and see pixels.
3. Run a serial agent TUI whose stub LLM drives Read/Edit/WebFetch on guest FS + HTTPS.
4. Use a joint profile where the agent’s `browse` / WebFetch path does **not** give `web-content` a NIC mapping.
5. Point at [ADR-009](decisions/009-interactive-surface.md) and [`context.md`](context.md) for why this is not a POSIX desktop and not Ladybird / Servo / `grok` ports.

Hardware truth remains [Physical RPi4 lab](plan-arch.md#physical-rpi4-lab-hardware-gated).

---

## Near-term priority

If capacity is limited, do **not** start with 78–80 first:

1. **Phase 72** — ramfb + `display-server` (done; unblocks paint).
2. **Phase 73** — `request-server` (done; unblocks browser load **and** agent HTTPS).
3. **Phase 74–75** — `lerux-html` + `lerux-web` (done). **76** (browser) done. **77** (agent runtime) done. Then **78–79** (TUI + tools).
4. **Phase 80** last.

RPi4 lab work never blocks this list. JS, GPU, and extra tabs stay off the list until a new ADR.

---

## Verification (program-level)

| Gate | Command / artifact |
|------|-------------------|
| Host lint | `just check` |
| PD lint | `just check-pd` (once PDs exist) |
| Display | `just test-display` (`lerux-display: pattern ok`) |
| Request-server | `just test-request` (`lerux-http: fixture ok`) |
| HTML | `just test-html` (`lerux-html: nodes=8`) |
| Paint | `just test-paint` (`lerux-web: paint ok`) |
| Browser | `just test-browser` (`lerux-browser: paint ok`) |
| Agent runtime | `just test-agent-runtime` (`lerux-agent: runtime ok`) |
| Agent tools | `just test-agent` (`lerux-agent: tools ok`) |
| Joint | `just test-interactive` |
| Isolation residual | existing `just test-isolation` still green |

Smokes stay on the smoke CA + `https-one` / `grok-one`. Do not add a CI job that hits the public Web or live xAI.
