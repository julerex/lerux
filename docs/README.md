# lerux documentation

lerux is a Rust-on-seL4 project. The kernel comes from upstream seL4; lerux owns Microkit system descriptions and protection-domain crates.

## Index

| Doc | Purpose |
|-----|---------|
| [../AGENTS.md](../AGENTS.md) | LLM agent instructions for idiomatic Rust in this repo |
| [context.md](context.md) | Domain language and architectural decisions |
| [plan.md](plan.md) | Roadmap and cross-arch smoke parity |
| [boards.md](boards.md) | Board names, PDs, QEMU profiles; [RPi4 workstation HW gate](boards.md#rpi4-workstation-manual-hw-gate-phase-39) |
| [ci.md](ci.md) | GitHub Actions pipeline, caches, troubleshooting |
| [seL4-whitepaper.pdf](seL4-whitepaper.pdf) | seL4 high-level overview (reference) |

## External references

- [seL4 documentation](https://docs.sel4.systems/)
- [Microkit tutorial](https://docs.sel4.systems/projects/microkit/tutorial/welcome.html)
- [Rust on seL4](https://docs.sel4.systems/projects/rust/)
- [rust-sel4 crates](https://github.com/seL4/rust-sel4)
- [rust-microkit-demo](https://github.com/seL4/rust-microkit-demo) — multi-PD IPC example