# AGENTS

Instructions for LLM agents writing Rust in this repository.

Read [docs/context.md](docs/context.md) first for domain terms (protection domain, Microkit, board features, rust-sel4).

## Scope

- Applies to all Rust files (`**/*.rs`) in this repo.
- Two contexts with different rules:
  - **Userspace** (`userspace/`) — `#![no_std]` protection domains and shared crates on seL4 Microkit
  - **Host tooling** (`tools/lerux-cli/`) — `std` build and test orchestration on the developer machine
- Favor correctness and matching existing patterns over drive-by refactors.
- Do not modify upstream trees in `deps/workspace/`.

## General idiomatic Rust

Apply to all Rust code unless a context-specific section overrides.

- Prefer borrowing (`&T`, `&str`, `&[T]`) over `.clone()` unless ownership transfer is required.
- Prefer `?` and `let … else { … }` over deep `match` chains for early exit.
- Prefer iterator pipelines for pure transforms; use `for` when `break`, `continue`, or side effects dominate.
- Import order: `core`/`alloc` → external crates → workspace / `lerux-*` → `crate::` / `super::`.
- Prefer `From` / `Into` / `TryFrom` over manual bit-twiddling conversions.
- Use `#[expect(clippy::…)]` with a one-line rationale instead of blanket `#[allow]`.
- Avoid magic numbers for IPC topology; use named `const` `Channel` values that match `.system` XML.
- Match existing naming: `HandlerImpl`, `SERIAL_DRIVER`, `*_DRIVER` channel constants.
- Keep comments purposeful (`why`, invariants, safety); remove stale commentary.
- Link TODOs to issues: `// TODO(#NNN): …`.

## Protection domains (`userspace/pds/**`)

Match the style already used in PD crates such as `echo-server` and `boot-init`.

### Crate attributes and entry

```rust
#![no_std]
#![no_main]

use sel4_microkit::{protection_domain, Channel, Handler, Infallible, MessageInfo};

const CLIENT: Channel = Channel::new(1);

#[protection_domain]
fn init() -> HandlerImpl {
    // init sinks, drivers, log readiness
    HandlerImpl
}
```

- Every PD `main.rs` uses `#![no_std]` and `#![no_main]`.
- Entry is `#[protection_domain] fn init() -> HandlerImpl` (or equivalent handler type).
- IRQ and notification handling uses `impl Handler for HandlerImpl` with `type Error = Infallible`.

### Panics and `unwrap`

`unwrap()` and `expect()` are acceptable in PD init and top-level handlers when failure is unrecoverable (firmware convention). Prefer `expect("invariant message")` over bare `unwrap()`. Use `unreachable!()` for channels that cannot arrive per the `.system` layout.

Do not drive-by refactor existing `unwrap` sites unless the task requires it.

### IPC

- Use `lerux_ipc` and typed messages from `lerux_interface_types` (postcard + serde).
- On decode failure, return `send_unspecified_error()` rather than panicking.
- Example pattern: `userspace/pds/echo-server/src/main.rs`.

### Logging

- Use `lerux_logging::serial` or `lerux_logging::debug` sinks.
- Apply `lerux_logging::default_filter` where noisy `sel4_sys` targets should be suppressed.

### Board features

- Gate platform-specific code with `#[cfg(feature = "board-…")]` in source and matching features in `Cargo.toml`.
- Never hardcode a single platform in logic shared across boards.
- Enable `alloc` only when `sel4-microkit/alloc` (or a PD feature that pulls it in) is required; prefer stack or static buffers in hot paths.

## Shared userspace libraries (`userspace/crates/**`)

Stricter than PDs.

- Stay `#![no_std]` unless there is a strong reason not to.
- Document public items with `//!` / `///`, including IPC contracts and safety assumptions.
- Prefer `Result` in fallible APIs; avoid new `panic!` in library code.
- Re-export upstream rust-sel4 APIs rather than reimplementing them (see `lerux-ipc`).
- Restrict `unsafe` to MMIO/HAL boundaries with documented invariants (see `lerux-virtio-hal`).
- Pin rust-sel4 via workspace git deps at `v4.0.0`; do not vendor copies.

## Host tooling (`tools/lerux-cli/**`)

- Use `anyhow::Result` at the CLI boundary; add context with `.context("…")?`.
- Use `clap` derive for subcommands.
- No `unwrap()` or `expect()` in production paths — use `?` or `bail!`.
- `std` only; do not depend on seL4 userspace crates.

## Quality gates

Run before finishing Rust changes:

```bash
just check
```

This runs `cargo fmt --all --check` and clippy on host crates (`lerux-cli`, `lerux-interface-types`). After PD or shared userspace crate changes, also run:

```bash
just check-pd
```

`check-pd` runs cross-target clippy on all PD and shared userspace crates (one pass per arch). It requires a built SDK (`just build-sdk`) for `SEL4_INCLUDE_DIRS`. `just check-all` runs both.

PD changes may need a full board build (`just build` or `just build-pd <crate>`) because targets are seL4 cross-compile profiles.

Workspace `[lints]` in the root `Cargo.toml` sets clippy defaults; each crate inherits them via `[lints] workspace = true`.

CI runs `just check` before the SDK pipeline and `just check-pd` after the SDK artifact is ready (in parallel with smoke).

## What not to do

- Do not introduce `std` into PD crates.
- Do not modify seL4 or Microkit sources under `deps/workspace/`.
- Do not add vendored rust-sel4 trees — use workspace dependencies.
- Do not "clean up" unrelated code, normalize experiments, or rewrite git history unless asked.

## Further reading

**General idiomatic Rust**

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — official naming, error, and API design
- [Apollo Rust Best Practices](https://github.com/apollographql/rust-best-practices) — practical idioms and lint discipline
- [Rust Design Patterns — Idioms](https://rust-unofficial.github.io/patterns/idioms/)
- [cheats.rs](https://cheats.rs/) — concise ownership, string, and error tips
- [Clippy](https://doc.rust-lang.org/clippy/) — machine-enforced idioms
- [idiomatic-rust index](https://corrode.dev/idiomatic-rust/) — curated article list

**lerux / embedded context**

- [Rust on seL4](https://docs.sel4.systems/projects/rust/)
- [rust-sel4 API docs](https://sel4.github.io/rust-sel4/)
- [Embedded Rust Book — no_std](https://docs.rust-embedded.org/book/intro/no-std.html)
- [High Assurance Rust](https://highassurance.rs/) — firmware-oriented patterns