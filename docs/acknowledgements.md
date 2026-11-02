# Acknowledgements

lerux-owned code is MIT ([`LICENSE`](../LICENSE)). The projects below keep their own licenses. Pins for the kernel tree live in [`deps/versions.toml`](../deps/versions.toml). The crate graph is `Cargo.lock`.

## Dependencies

These are linked or read at build time. The seL4, Microkit, and LionsOS checkouts are cloned into `deps/workspace/` and are not part of this git history.

| Project | Pin | Role |
|---------|-----|------|
| [seL4](https://github.com/seL4/seL4) | 16.0.0 | Kernel. GPL-2.0-only |
| [seL4 Microkit](https://github.com/seL4/microkit) | 2.3.0 | Static system layout, loader, and the SDK that supplies the libsel4 headers |
| [rust-sel4](https://github.com/seL4/rust-sel4) | v5.0.0 | Userspace runtime and drivers, including `sel4-microkit`. BSD-2-Clause |
| [LionsOS](https://github.com/au-ts/lionsos) | 0.4.0 | Queue, filesystem, and network structure headers read by `lerux-sddf`. The headers are not linked into a protection domain |
| [smoltcp](https://github.com/smoltcp-rs/smoltcp) | 0.13 | TCP/IP stack in the network clients |
| [virtio-drivers](https://github.com/rcore-os/virtio-drivers) | 0.13 | Virtio device access |
| [rustls](https://github.com/rustls/rustls) | 0.23 | TLS in `lerux-tls` and in the host command-line interface, with `rustls-rustcrypto`, `rustls-pemfile` 2, and optional `webpki-roots` 0.26 |
| [wasmi](https://github.com/wasmi-labs/wasmi) | 2 | WebAssembly interpreter in [`lerux-wasm`](../userspace/crates/lerux-wasm). See [ADR-012](decisions/012-ordinary-webassembly.md) |
| [Verus](https://github.com/verus-lang/verus) | the pins in [`lerux-ring`](../userspace/crates/lerux-ring/Cargo.toml) | Proofs for `lerux-ring` |
| [ed25519-dalek](https://github.com/dalek-cryptography/ed25519-dalek) | 2 | Signatures on WebAssembly modules |

The host command-line interface in [`tools/lerux-cli/Cargo.toml`](../tools/lerux-cli/Cargo.toml) also uses clap, anyhow, and the other crates named there. `Cargo.lock` is the full graph.

Manuals: [seL4 documentation](https://docs.sel4.systems/), the [Microkit tutorial](https://docs.sel4.systems/projects/microkit/tutorial/welcome.html), and [Rust on seL4](https://docs.sel4.systems/projects/rust/).

## Inspirations

These shaped the design. Their source is not vendored.

- [sDDF](https://github.com/au-ts/sddf) queue and virtualiser shape, from the [Trustworthy Systems](https://github.com/au-ts) ecosystem. See [ADR-002](decisions/002-serial-virtualiser.md), [ADR-003](decisions/003-net-virtualiser.md), [ADR-011](decisions/011-lionsos-structures.md), and [`plan-au-ts.md`](plan-au-ts.md).
- [`libmicrokitco`](https://github.com/au-ts/libmicrokitco), reimplemented as `lerux-cothread`. See [ADR-004](decisions/004-service-async.md).
- [`microkit_sdf_gen`](https://github.com/au-ts/microkit_sdf_gen), the idea behind in-tree system generation. See [ADR-001](decisions/001-in-tree-system-generation.md).
- [rust-microkit-demo](https://github.com/seL4/rust-microkit-demo), a multi-protection-domain example.
- [`libgdb`](https://github.com/au-ts/libgdb), the idea behind the debug workflow. See [ADR-005](decisions/005-debug-pd.md).
- [Ladybird](https://github.com/LadybirdBrowser/ladybird), the browser process shape. See [ADR-009](decisions/009-interactive-surface.md) and [`plan-interactive.md`](plan-interactive.md).
- [Grok Build](https://github.com/xai-org/grok-build), the agent tool loop. See [ADR-009](decisions/009-interactive-surface.md).
- [QIP](https://github.com/patrickgwsmith/qip), content and graphical WebAssembly contracts still under investigation. See [`plan-qip.md`](plan-qip.md).
