# lerux documentation

lerux is a Rust-on-seL4 project. The kernel comes from upstream seL4; lerux owns Microkit system descriptions and protection-domain crates.

## Index

| Doc | Purpose |
|-----|---------|
| [../AGENTS.md](../AGENTS.md) | LLM agent instructions for idiomatic Rust in this repo |
| [context.md](guide/overview/context.md) | Domain language and architectural decisions |
| [ARCHITECTURE.md](guide/overview/ARCHITECTURE.md) | Dependency hierarchy and call graph across lerux, rust-sel4, Microkit, and seL4 |
| [../GLOSSARY.md](../GLOSSARY.md) | Machine names, driver words, the Acer Swift X devices, the Wi-Fi bring-up line, and the Arch live image. Each mention links to its entry |
| [arch-live-image.md](hardware/boards/arch-live-image.md) | What fills the official Arch Linux live installer, measured from the 2026.11.01 image |
| [swift-boot-screen.md](hardware/swift/swift-boot-screen.md) | Plain-language guide to the Swift X Wi-Fi stall: where the ring, the note, the doorbell, and the IOMMU live, how to read the screen lines, and the dump rows 31 to 66 |
| [swift-wifi-assessment.md](hardware/swift/swift-wifi-assessment.md) | Assessment of the Swift X bring-up against Linux v6.8 `mt76`: what is verified, the remaining suspects in order, and the gap between `ping ok` and applications on the internet |
| [swift-wifi-survey-retrospective.md](hardware/swift/swift-wifi-survey-retrospective.md) | Why the Swift survey printed `rx 0` for so many boots: the scan frames arrived on an MCU ring the driver did not read for frames |
| [z97-keyboard-retrospective.md](hardware/z97/z97-keyboard-retrospective.md) | Why the Z97 Microsoft transceiver took four images: VT-d blocked the xHCI command ring, then a control TRB change shipped beside a second one |
| [z97-d3h-hardware.md](hardware/z97/z97-d3h-hardware.md) | Devices on this Gigabyte Z97-D3H, from the 2026-11-27 Linux boot |
| [plan-swift-wifi.md](plans/radio/plan-swift-wifi.md) | Phased plan from the `fw sem` stall to an application on the internet over the Swift radio |
| [plan.md](plans/core/plan.md) | Roadmap and cross-arch smoke parity (phases 1–82, done). The metal target being driven is the Acer Swift X. The Z97 desk page and the [Physical RPi4 lab](plans/core/plan.md#physical-rpi4-lab-hardware-gated) stay secondary |
| [plan-lionsos.md](plans/core/plan-lionsos.md) | LionsOS structures in a Rust guest. Milestone 17 stays (postcard guest model). The metal target being driven is the Acer Swift X (milestones 24–29). Milestone 22, the Z97 bind, stays secondary |
| [plan-arch.md](plans/core/plan-arch.md) | Phases 50–60 living checklist. The Acer Swift X is the metal target being driven. [Physical RPi4 lab](plans/core/plan-arch.md#physical-rpi4-lab-hardware-gated) stays secondary |
| [plan-qemu.md](plans/core/plan-qemu.md) | Phases 61–70 living checklist (QEMU-only workstation deepening; done) |
| [plan-interactive.md](plans/core/plan-interactive.md) | Phases 71–80 record: QEMU interactive surface. Milestone 17 stays and did not remove those domains. JavaScript and WebAssembly are approved |
| [../lerux-ladybird/PLAN.md](../lerux-ladybird/PLAN.md) | Browser modelled on Ladybird and rewritten in Rust, not a port of Ladybird, Chromium, or WebKit C++. Milestone 17 of this file is Forms and stays |
| [desk-browser.md](hardware/boards/desk-browser.md) | Secondary Z97 desk page: one public page, from the typed address to the motherboard HDMI port. The Acer Swift X is the metal target being driven |
| [plan-qip.md](plans/radio/plan-qip.md) | Investigation: QIP component contracts and whether a host belongs in lerux. No decision yet. |
| [bluechip.md](guide/overview/bluechip.md) | Investigation: emulating the Anduril BlueChip laptop (Snapdragon X), the lerux port gap, and a QEMU proxy board. No decision yet. |
| [config.md](guide/ops/config.md) | Phase 54: config key schema, secrets, boot policy |
| [packages.md](guide/ops/packages.md) | Phase 55: package CLI, pins, profile recipes, out-of-tree “AUR” |
| [plan-au-ts.md](plans/radio/plan-au-ts.md) | Phases 41–49: sDDF/LionsOS/sdfgen-inspired work |
| [system-generation.md](guide/overview/system-generation.md) | Phase 41: template inventory (mechanical vs board-specific) |
| [decisions/001-in-tree-system-generation.md](decisions/001-006/001-in-tree-system-generation.md) | ADR-001: in-tree Rust SDF gen (not sdfgen) |
| [decisions/002-serial-virtualiser.md](decisions/001-006/002-serial-virtualiser.md) | ADR-002: serial-driver + serial-virt (sDDF-shaped) |
| [decisions/003-net-virtualiser.md](decisions/001-006/003-net-virtualiser.md) | ADR-003: net trust map; unified-dma on aarch64 virtio |
| [decisions/007-tls-proxy.md](decisions/007-012/007-tls-proxy.md) | ADR-007: superseded; rustls now lives in `lerux-tls` inside the client |
| [decisions/008-host-backed-fs.md](decisions/007-012/008-host-backed-fs.md) | ADR-008: no virtio-9p; the LERUXFS2 `/host` seed is gone |
| [decisions/009-interactive-surface.md](decisions/007-012/009-interactive-surface.md) | ADR-009: QEMU software framebuffer; static Ladybird PDs; Grok-shaped agent |
| [decisions/010-program-runtime.md](decisions/007-012/010-program-runtime.md) | ADR-010: `LRW1` envelope; the postcard runtime PD is deleted |
| [decisions/011-lionsos-structures.md](decisions/007-012/011-lionsos-structures.md) | ADR-011: LionsOS queue and filesystem structures in Rust |
| [decisions/012-ordinary-webassembly.md](decisions/007-012/012-ordinary-webassembly.md) | ADR-012: `wasmi` in `sddf-program`; postcard runtime deleted |
| [decisions/013-desk-intel-scanout.md](decisions/013-014/013-desk-intel-scanout.md) | ADR-013: desk scanout through the Intel graphics |
| [decisions/014-uefi-mb2-stub.md](decisions/013-014/014-uefi-mb2-stub.md) | ADR-014: GRUB loads `mb2-stub`; Limine could not place `sel4.elf` |
| [decisions/004-service-async.md](decisions/001-006/004-service-async.md) | ADR-004: stackless coop async in service PDs |
| [decisions/005-debug-pd.md](decisions/001-006/005-debug-pd.md) | ADR-005: fault parent + QEMU GDB (not libgdb fork) |
| [debug.md](guide/ops/debug.md) | Phase 46: `test-debug` + gdb-multiarch workflow |
| [security.md](guide/overview/security.md) | Phase 60: threat model, pin runbook, image digests, isolation smoke |
| [qos.md](guide/quality/qos.md) | Phase 48: workstation service classes / PD priorities |
| [decisions/006-workstation-qos.md](decisions/001-006/006-workstation-qos.md) | ADR-006: priority policy |
| [bench.md](guide/quality/bench.md) | Phase 49: microbench methodology |
| [ops.md](guide/ops/ops.md) | Operations: diagnose, smoke logs, host helpers |
| [platforms.md](hardware/boards/platforms.md) | Platform notes and hardware bring-up |
| [bench-results.latest.md](guide/quality/bench-results.latest.md) | Latest `just bench` snapshot (regenerated) |
| [net-topology.md](guide/quality/net-topology.md) | NIC paths: device-framework client vs remaining virtio boards |
| [setup.md](guide/ops/setup.md) | First boot of the Swift X live image: build the ISO, write the stick, and join Wi-Fi with `setup-wifi` |
| [boards.md](hardware/boards/boards.md) | Board names, PDs, QEMU profiles; [Swift X install path](hardware/boards/boards.md#acer-swift-x-install-path); [RPi4 install path](hardware/boards/boards.md#rpi4-workstation-install-path-phase-52); [Z97-D3H install path](hardware/boards/boards.md#gigabyte-z97-d3h-install-path) |
| [ci.md](guide/ops/ci.md) | GitHub Actions pipeline, caches, troubleshooting |
| [acknowledgements.md](guide/quality/acknowledgements.md) | Dependencies and inspirations |
| [seL4-whitepaper.pdf](guide/papers/seL4-whitepaper.pdf) | seL4 high-level overview (reference) |

## External references

Dependencies and inspirations are in [acknowledgements.md](guide/quality/acknowledgements.md).