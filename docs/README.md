# lerux documentation

lerux is a Rust-on-seL4 project. The kernel comes from upstream seL4; lerux owns Microkit system descriptions and protection-domain crates.

## Index

| Doc | Purpose |
|-----|---------|
| [../AGENTS.md](../AGENTS.md) | LLM agent instructions for idiomatic Rust in this repo |
| [context.md](context.md) | Domain language and architectural decisions |
| [plan.md](plan.md) | Roadmap and cross-arch smoke parity (phases 1–60; 50–60 core done) |
| [plan-arch.md](plan-arch.md) | Phases 50–60: Arch-level functionality gap plan |
| [config.md](config.md) | Phase 54: config key schema, secrets, boot policy |
| [packages.md](packages.md) | Phase 55: package CLI, pins, profile recipes, out-of-tree “AUR” |
| [plan-au-ts.md](plan-au-ts.md) | Phases 41–49: sDDF/LionsOS/sdfgen-inspired work |
| [system-generation.md](system-generation.md) | Phase 41: template inventory (mechanical vs board-specific) |
| [decisions/001-in-tree-system-generation.md](decisions/001-in-tree-system-generation.md) | ADR-001: in-tree Rust SDF gen (not sdfgen) |
| [decisions/002-serial-virtualiser.md](decisions/002-serial-virtualiser.md) | ADR-002: serial-driver + serial-virt (sDDF-shaped) |
| [decisions/003-net-virtualiser.md](decisions/003-net-virtualiser.md) | ADR-003: net trust map; unified-dma on aarch64 virtio |
| [decisions/004-service-async.md](decisions/004-service-async.md) | ADR-004: stackless coop async in service PDs |
| [decisions/005-debug-pd.md](decisions/005-debug-pd.md) | ADR-005: fault parent + QEMU GDB (not libgdb fork) |
| [debug.md](debug.md) | Phase 46: `test-debug` + gdb-multiarch workflow |
| [security.md](security.md) | Phase 60: threat model, trust map, isolation smoke |
| [qos.md](qos.md) | Phase 48: workstation service classes / PD priorities |
| [decisions/006-workstation-qos.md](decisions/006-workstation-qos.md) | ADR-006: priority policy |
| [bench.md](bench.md) | Phase 49: microbench methodology |
| [ops.md](ops.md) | Operations: diagnose, smoke logs, host helpers |
| [platforms.md](platforms.md) | Platform notes and hardware bring-up |
| [bench-results.latest.md](bench-results.latest.md) | Latest `just bench` snapshot (regenerated) |
| [net-topology.md](net-topology.md) | NIC / net-server / app channel map |
| [boards.md](boards.md) | Board names, PDs, QEMU profiles; [RPi4 install path (Phase 52)](boards.md#rpi4-workstation-install-path-phase-52) |
| [ci.md](ci.md) | GitHub Actions pipeline, caches, troubleshooting |
| [seL4-whitepaper.pdf](seL4-whitepaper.pdf) | seL4 high-level overview (reference) |

## External references

- [seL4 documentation](https://docs.sel4.systems/)
- [Microkit tutorial](https://docs.sel4.systems/projects/microkit/tutorial/welcome.html)
- [Rust on seL4](https://docs.sel4.systems/projects/rust/)
- [rust-sel4 crates](https://github.com/seL4/rust-sel4)
- [rust-microkit-demo](https://github.com/seL4/rust-microkit-demo) — multi-PD IPC example
- [au-ts org](https://github.com/au-ts) — upstream Microkit / sDDF / LionsOS / rust-sel4 ecosystem (see [plan-au-ts.md](plan-au-ts.md))