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
: Foundation-maintained Rust bindings and runtimes ([Git dependency](https://github.com/seL4/rust-sel4), tag `v5.0.0`). Not vendored; pinned in `Cargo.toml`.

**No vendoring**
: seL4 and microkit source live in gitignored `deps/workspace/`. Version pins are committed in `deps/versions.toml`.

## Resolved decisions (2026-06 pivot)

| Decision | Choice |
|----------|--------|
| Kernel | Upstream seL4 16.0.0, built from source |
| Userspace model | seL4 Microkit (static PD layout) |
| Userspace language | Rust only (no musllibc/relibc in lerux code) |
| New device and filesystem connections | LionsOS structures in Rust (`lerux-sddf`, [ADR-011](decisions/011-lionsos-structures.md)) |
| First platform | aarch64 QEMU virt; x86_64 parameterized for follow-up |
| Dependency fetch | `lerux fetch` git clones (pinned tags) |

**Interactive surface (2026-09, ADR-009):** QEMU software framebuffer is in scope; GPU/Wayland/libvmm/JS are not. Browser and agent are new PDs, not ports of Ladybird or `grok`. See [`plan-interactive.md`](plan-interactive.md).

The written desk-browser spec is [`desk-browser.md`](desk-browser.md).

**Z97 monitor (phase 82):** `pc_z97_d3h` paints VGA text at `0xb8000` through the firmware boot VGA card. The desk-browser spec makes that card the Intel HD Graphics 4600. The monitor cable is on the motherboard HDMI port. Firmware Peripherals → Initial Display Output is `IGFX`, and Intel Processor Graphics stays enabled. The spec uses the VESA linear framebuffer Limine sets on that legacy entry. `browser-ui` maps it. lerux does not program the Intel display engine or the Polaris display engine. The RX 560 stays unused. After the reboot on 2026-10-22, firmware reports the Intel device as the boot VGA device. Limine's legacy entry reports a linear framebuffer at `0xd0000000`, pitch 4096, 1024 by 768, 32 bits per pixel, with red at shift 16, green at shift 8, and blue at shift 0. See [ADR-013](decisions/013-desk-intel-scanout.md) and [`boards.md`](boards.md#vga-text-on-the-rx-560).

**Z97 fetch (desk-browser spec):** `request-server` fetches the public page through `net-server`. A new driver owns the onboard Ethernet port and serves `net-server`. That port is bus address `00:19.0`, vendor `8086`, device `153b`, revision `0`, subsystem `1458:e000`, with memory `0xf7e00000` (128 KiB), memory `0xf7e3c000` (4 KiB), and I/O `0xf080` (32 bytes). The RTL8852BE probe is not that driver.

**Z97 image (desk-browser spec):** The image this desk boots contains `console-driver`, `serial-driver`, `log-server`, `browser-ui`, one `web-content`, `request-server`, `tls-proxy`, `net-server`, and the new onboard Ethernet driver. `console-driver` forwards key-downs and does not paint. `browser-ui` maps the linear framebuffer. COM1 stays the log, and there is no shell prompt. `tls-proxy` holds the public trust anchors. The shell, the filesystem, `display-server`, and the agent are not on this image. The Phase 82 shell image remains the QEMU console test.

**Z97 demonstration (desk-browser spec):** Before a metal boot, a separate QEMU image boots those browser protection domains on the legacy Limine entry, with the existing x86 virtio network driver in place of the onboard Ethernet driver. `tls-proxy` on that image trusts the smoke certificate authority. The address line accepts `https://host:8443/`. PS/2 injection loads `https://host:8443/desk.html`, follows the link to `https://host:8443/desk-link.html`, and submits the GET field `q` with `hi`, loading `https://host:8443/desk-form.html?q=hi`. COM1 records each address and the first non-white pixel. The framebuffer address in that log is QEMU's. The metal boot remains the public page on this desk.

## Platform parity

x86 names that get mixed across QEMU and the desk (q35, the 82574, the I217-V, COM1, the VGA text page, the CRTC, the Intel display engine) are defined in [GLOSSARY.md](../GLOSSARY.md). The driver tables are in [platforms.md](platforms.md#x86-device-map).

Virtio hello and HTTP smokes run on aarch64, RISC-V virt, and x86 (PCI virtio on q35). Debug hierarchy smokes run on all three. See [`boards.md`](boards.md) and [`plan.md`](plan.md).

## LionsOS structures

The guest remains Rust. Device and filesystem connections use the LionsOS and seL4 Device Driver Framework structures in [`lerux-sddf`](../userspace/crates/lerux-sddf), checked against the LionsOS 0.4.0 headers. A header that is missing from that checkout stops the milestone. The struct is not invented. Milestone 17 deleted the postcard protection domains and `lerux-interface-types`. See [ADR-011](decisions/011-lionsos-structures.md) and the milestone list in [plan-lionsos.md](plan-lionsos.md).

## Non-POSIX direction

The guest does **not** target a Linux or POSIX syscall ABI. Remaining protection domains speak LionsOS queues (`fs_cmd_t`, serial and network rings) and, where a file descriptor table exists, `lerux-posix`. Milestone 17 deleted typed postcard RPC (`lerux-interface-types`). The guest still has no `fork`/`exec`.

“Arch-like” means **workflow**, not binary compatibility: rolling PD artifact pins, named system profiles, init ordering, shell + core utilities — each implemented as PDs you port deliberately. Unmodified Arch packages (`bash`, `pacman`, `firefox`, etc.) are out of scope. Numbered plans through phase 82 are done: Arch-level capability (phases 50–60, [`plan-arch.md`](plan-arch.md)), QEMU deepening (phases 61–70, [`plan-qemu.md`](plan-qemu.md)), the interactive surface (phases 71–80, [`plan-interactive.md`](plan-interactive.md), [ADR-009](decisions/009-interactive-surface.md)), the signed Wasm runtime (phase 81, [ADR-010](decisions/010-program-runtime.md)), and the Z97 on-screen shell (phase 82). Remaining on-device work is the physical lab in [`plan-arch.md`](plan-arch.md).

## System profiles and packages

**System profile**
: A named bundle in `support/profiles/*.toml`: which PD crates, `.system` template, and channel manifest compose one `loader.img`. Use `lerux profile list|build|diff <name>`. Board hardware (MMIO, IRQs, arch) remains in `support/boards.toml`; profiles are selected via default_board or `--board`. (Phase 35)

**System generation (Phase 41)**
: Composed Microkit SDF = board `system_vars` + layout template (MRs/PDs/maps) + **generated channels** from the profile’s structured `[[channel]]` list ([ADR-001](decisions/001-in-tree-system-generation.md)). Workstation templates are channel-free; `lerux profile check-channels` guards PD `Channel::new` drift. Details: [`system-generation.md`](system-generation.md).

**Serial virtualiser (Phase 42)**
: The `qemu_virt_aarch64_serial_sddf` image is the live serial path. Its protection domains are `serial_driver`, `serial_virt_tx`, `serial_virt_rx`, and `serial_client`. The client writes `lerux shell ready` with `serial_enqueue`. `hello`, `html-demo`, and the cothread image still use the remaining `serial-driver` crate. Milestone 17 deleted `serial-virt` and the postcard workstation shell. See [ADR-002](decisions/002-serial-virtualiser.md).

**Network topology (Phase 43 / 51)**
: Milestone 17 deleted `net-server` and `NetRequest`. On the device-framework images the client owns smoltcp. A driver, a transmit virtualiser, a receive virtualiser, and one copier move the frames. The client does not map the device queue. Remaining virtio boards attach `virtio-net-driver` (or `virtio-pci-driver` on x86) next to `hello` or `http-server`. On QEMU virtio-net, **unified-dma** still removes the separate client_dma region: Hal plus bounce share the driver DMA region. Apps never map net DMA. See [ADR-003](decisions/003-net-virtualiser.md), [`net-topology.md`](net-topology.md). The Milestone 7 image is `qemu_virt_aarch64_net_sddf` (`just test-net-sddf`). Agent rules are in [AGENTS.md](../AGENTS.md#network-image-qemu_virt_aarch64_net_sddf). The Milestone 8 image `qemu_virt_aarch64_fetch_sddf` puts that network path and the filesystem shell in one image. The shell fetches the host fixture with `lerux-tls` and writes the body through `lerux-posix`. The same shell runs `calc`, `backup`, `source`, `cert`, `edit`, and `chat`. `chat` uses the socket calls. Agent rules are in [AGENTS.md](../AGENTS.md#fetch-image-qemu_virt_aarch64_fetch_sddf). The Milestone 11 image `qemu_virt_aarch64_web_sddf` reuses that template. The shell reads `/config/hostname` and serves those bytes on guest port 8080. Agent rules are in [AGENTS.md](../AGENTS.md#web-image-qemu_virt_aarch64_web_sddf). The Milestone 12 image `qemu_virt_aarch64_browser_sddf` adds a virtio graphics driver and virtualiser. The shell prints `title lerux` and then `pixel 000000ff` from a 320 by 64 frame. Agent rules are in [AGENTS.md](../AGENTS.md#browser-image-qemu_virt_aarch64_browser_sddf). The Milestone 13 image `qemu_virt_aarch64_agent_sddf` keeps the filesystem and network roles. `grok` notifies the agent. The agent uses `lerux-posix` for the file tools and the socket library for WebFetch, and one shared page plus a notification for Browse. Agent rules are in [AGENTS.md](../AGENTS.md#agent-image-qemu_virt_aarch64_agent_sddf).

**Filesystem backends (Phase 44 / 50)**
: Milestone 17 deleted `fs-server`, `lerux-fs`, and `lerux-fat`. The live filesystem image is `qemu_virt_aarch64_fs_sddf`. It speaks `fs_cmd_t`, keeps its File Allocation Table in `sddf-fatfs`, and does not use `lerux-fat`. The shell on that image is the filesystem client. Serial roles share the image. A path is one 8.3 component in the root, and one read-only open of exactly two components serves `/config/hostname`. Agent rules are in [AGENTS.md](../AGENTS.md#filesystem-image-qemu_virt_aarch64_fs_sddf).

**Service async (Phase 45)**
: Service protection domains keep Microkit `Handler` as the root event loop. Blocking input and output uses stackful cothreads in `lerux-cothread` (Milestone 3). Milestone 17 deleted `lerux-service-async`. See [ADR-004](decisions/004-service-async.md).

**Debug / faults (Phase 46)**
: Child PD faults can be delivered to a **parent** PD (`Handler::fault`) via Microkit hierarchy. Demo: `debug-handler` + `crash-demo` (`just test-debug`). Interactive host debugging uses QEMU’s gdbstub + `gdb-multiarch` ([`debug.md`](debug.md), [ADR-005](decisions/005-debug-pd.md)).

**Security posture (Phase 60)**
: Trust map and threat model in [`security.md`](security.md). `just test-debug` still nests `crash-demo` under `debug-handler`. Milestone 17 deleted the isolation board that completed a filesystem round-trip after that fault.

**Hardware serial smoke (Phase 47 / 52)**
: On-device boot checks use `LERUX_HW_SERIAL` + `--mode hw-serial` / `just test-hw`, with expects from `support/smoke-expects.toml` and a local board lock. Phase 52 adds optional **scripted REPL** steps (`script = [{send, expect}, …]`) after boot match, and `lerux deploy` / `just deploy-rpi4` for SD boot media. First-boot seeds `/config` via supervisor. Cloud CI stays QEMU-only; optional self-hosted workflow is documented in [`ci.md`](ci.md). Install path: [`boards.md`](boards.md#rpi4-workstation-install-path-phase-52). Remaining on-device work: [`plan-arch.md` — Physical RPi4 lab](plan-arch.md#physical-rpi4-lab-hardware-gated).

**Workstation QoS (Phase 48)**
: Fixed Microkit PD priorities form service classes (platform / services / control / bulk / interactive). PPC callees must outrank callers (shell stays lowest among clients). See [`qos.md`](qos.md), [ADR-006](decisions/006-workstation-qos.md).

**Microbenches (Phase 49)**
: `just bench` runs guest-timed echo RTT, blk read IOPS, and UDP TX PPS on QEMU aarch64 and writes markdown/JSON summaries. See [`bench.md`](bench.md).

**Shell (Phase 53 / 57 / 58)**
: Interactive REPL over serial with file/net/sys built-ins (`ls`…`df`, `ip`/`ping`, `uptime`/`history`/`clear`, apps). Long `cat`/`dmesg` use a space/`q` pager. `dmesg --pd` / `-l` filter the log ring; `ps`/`top`/`status` show service state; `calc`, `backup`, `fetch save`, `chat [#room]`. `help -l` and boot log `lerux-shell: cmds=` expose a machine-readable command list for smokes.

**App catalog (Phase 58)**
: Installable PD packages under `support/packages/`: edit, chat-client, http-file-browser, backup, fetch-client (≥5). See [`packages.md`](packages.md).

**Multi-arch workstation (Phase 59)**
: Workstation is a profile concept across arches: `workstation` (aarch64), `workstation-riscv`, `workstation-x86`, `workstation-rpi4`. Shared app channels; arch-specific serial/virtio/time drivers. Tiers: [`platforms.md`](platforms.md).

**Config policy (Phase 54)**
: FS-backed keys under `/config/` via `config-server` ([`docs/config.md`](config.md)). Supervisor seeds missing keys only (`boot.seeded`), logs active hostname/net.mode/log.level, and may rotate `/boot.log`. `net.*` is live-applied (`NetRequest::ApplyIface`); shell `config set net.*` updates the stack without reboot. Secrets use the `secret.*` prefix (`/config/secrets/`). Host: `lerux config schema|defaults|seed-disk`.

**Package (Phase 40 / 55)**
: One PD crate plus its interface-types version and an optional profile fragment (`support/packages/<name>.toml`). Host CLI: `lerux package search|install|remove|upgrade` merges fragments into profiles (channel auto-wiring by name), then `lerux profile build`. Pins in `support/package-pins.toml`. See [`packages.md`](packages.md). Microkit does not load arbitrary ELFs at runtime.

**Supervisor**
: Evolution of `boot-init` (Phase 33): `supervisor` PD provides RTC/timer, brings up FS/net services, performs ordered app notify (generalizes composed-sync), exposes reboot/status IPC. Phase 56: static **service graph** log lines (`unit=… after=… restart=no`) and a post-bring-up **watchdog** re-query of the timer PD. Phase 57: richer `ServiceStatus` (state + last error) from FS/net probes; applies `log.level` to log-server.

**Observability (Phase 57)**
: Tagged log ring (`log-server`), shell filters, host `lerux diagnose` on serial captures under `build/smoke-logs/`, optional `lerux bench --check` against `support/bench-thresholds.toml`. See [`ops.md`](ops.md).

**Ported app checklist** (new PD that users interact with):

1. Use the LionsOS queue or configuration struct from the pinned headers (`lerux-sddf`). Do not invent a postcard type.
2. Implement the protection domain (`#![no_std]`)
3. Wire channels in the profile `[[channel]]` manifest (or copy them from the system template into the config struct). `lerux profile check-channels` validates ends
4. Add `board-<name>` features in PD `Cargo.toml` files when that key exists in `support/boards.toml`
5. Register the board in `support/boards.toml`, smoke expects in `lerux-cli`, and a continuous integration job if needed

**Interactive surface (Phase 71 / ADR-009)**
: Phases 71–80 built a QEMU software framebuffer and Ladybird-shaped browser protection domains. Milestone 17 deleted those protection domains. The live browser smoke is `just test-browser-sddf`. The desk-browser spec names the Ladybird-shaped set again. Checklist: [`plan-interactive.md`](plan-interactive.md).

**display-server**
: Desk-browser spec term for a trusted protection domain that owns a QEMU framebuffer. The crate is not in this tree. Milestone 17 deleted it. The live browser image paints through virtio graphics queues.

**framebuffer MR**
: Desk-browser spec term for a shared bitmap. The live browser image uses the graphics client data region instead.

**browser-ui**
: Desk-browser spec term. Ladybird Browser analogue. Owns the address line, presents the screen, and delivers page keys to `web-content`. Does not parse HTML. The protection domain is not built. Phase 76 chrome was a serial `open <url>`. On the desk-browser spec it maps the VESA linear framebuffer and receives key-downs from `console-driver`.

**desk keyboard**
: The PS/2 keyboard or, when one is armed, the USB HID boot keyboard. `console-driver` owns it and forwards key-downs to `browser-ui`.

**address line**
: The text `browser-ui` paints in the top 16 rows of the desk framebuffer. It shows the address of the public page.

**web-content**
: Desk-browser spec term. Ladybird WebContent analogue. One static protection domain (Microkit cannot spawn tabs). Hosts `lerux-html` + `lerux-web`. The protection domain is not built. The live browser shell links those engines and fetches with `lerux-tls`. The spec grows the engines so this protection domain can show a public page, and on that spec it paints only the rectangle under the address line.

**public name**
: A hostname resolved through the Domain Name System. It is the host of a public page.

**public page**
: What the monitor shows for one `https` address whose host is a public name. The desk-browser spec reaches it by typing the address, by activating a link, or by submitting a form. The typed address is the first load in a fresh profile.

**focused control**
: The one link or form control on the public page that receives keys. Tab moves to the next one and wraps.

**request-server**
: Desk-browser spec term. Ladybird RequestServer analogue, the only fetcher on that image. The protection domain is not built. The live browser shell sends one fixed `GET`.

**program-runtime**
: Phase 81 postcard protection domain. Milestone 17 deleted it. The live runtime is `qemu_virt_aarch64_program_sddf`, which reads a file and allocates a socket through `lerux-posix` and runs a signed module with `wasmi` ([ADR-010](decisions/010-program-runtime.md), [ADR-012](decisions/012-ordinary-webassembly.md)).

**agent**
: Grok Build analogue as a protection domain: prompt → tool-calls → tools → model. The live image is `qemu_virt_aarch64_agent_sddf` (`sddf-agent`). The postcard agent and `request-server` path are gone. Sandbox **is** the protection-domain set, not Landlock.

**lerux-html**
: From-scratch `#![no_std]`+`alloc` HTML engine. The live browser shell and `html-demo` use it. Phases 71–80 parse a fixed element subset. The desk-browser spec grows it to the document class of a public page. Not Ladybird’s `libweb_html_tokenizer`, not html5ever, not Servo.

**lerux-web**
: From-scratch `#![no_std]`+`alloc` layout and paint. Phases 71–80 painted a fixed CSS subset and hosted no JavaScript. The live browser shell uses it on `qemu_virt_aarch64_browser_sddf`. The desk-browser spec grows it to the shared layout, script, WebAssembly, and media behavior of the reference browsers. Not Servo.

## Boundaries

- **In scope:** Rust protection-domain crates, `.system` files, build/CI, docs, host profile tooling, QEMU software framebuffer (ADR-009), ordinary WebAssembly through `wasmi` in the one runtime protection domain (ADR-010, ADR-012). The desk-browser spec also grows `lerux-html` and `lerux-web` to show a public page, including its script and WebAssembly.
- **Out of scope:** POSIX/glibc/musl, Linux ABI emulation, seL4 kernel modifications, C userspace, vendored upstream trees, unmodified third-party binaries (including Ladybird, Servo, and `grok`), `wasmtime`, the WebAssembly Micro Runtime, a second runtime protection domain, GPU compositor / Wayland / libvmm
- **Upstream SDK components:** Microkit monitor, loader, libmicrokit (C) — part of SDK, not lerux-owned
- **Reference trees (read, do not vendor):** Ladybird, Grok Build — steal topology and tool loop only ([`plan-au-ts.md`](plan-au-ts.md) rule)
