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
- **Channel numbers come from the profile manifest**, not freehand magic. Use named `const` `Channel` values that match `support/profiles/*.toml` `[[channel]]` ends (and the composed SDF). Run `lerux profile check-channels` after renumbering; see Phase 41 / ADR-001 / `docs/system-generation.md`. An sDDF image with no profile keeps the channel ids in the system template and copies them into the config struct. The protection domain reads that struct. See [LionsOS queues](#lionsos-queues-lerux-sddf).
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
- New queue and filesystem code uses `lerux-sddf` (`serial_queue_t`, `fs_cmd_t`, and the other headers named in ADR-011). Do not add a postcard message for a new device or filesystem connection. Postcard remains for protection domains that ADR-011 has not replaced yet.

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

## LionsOS queues (`lerux-sddf`)

Read [docs/plan-lionsos.md](docs/plan-lionsos.md) and [ADR-011](docs/decisions/011-lionsos-structures.md). One milestone per pass. Leave a postcard protection domain in place until that milestone's exit says to delete it. The serial example is `qemu_virt_aarch64_serial_sddf`: `serial_driver`, `serial_virt_tx`, `serial_virt_rx`, and `serial_client`.

### Config bytes

- Fill a `#[repr(C)]` config by zeroing it, then assigning fields. A struct literal leaves padding uninitialized, so two fills of `serial_connection_resource_t` do not compare equal: the `u8` `id` is padded out to the next pointer. `serial_config_from_bytes` is `unsafe` because a `bool` other than 0 or 1 is undefined behavior. The generator writes 0 or 1.
- The installed Microkit kit is 2.2.0 and cannot prefill a memory region. Do not objcopy a zero `#[link_section]` static. rustc places that static in `.bss`. Each protection domain `build.rs` writes `OUT_DIR/config.bin` from the `lerux-sddf::serial_image` constructors. The domain `include_bytes!` those bytes and copies them into an aligned value. `include_bytes!` has alignment 1. The host and the guest are both 64-bit little-endian, so the pointer-sized virtual addresses survive the copy.
- The protection domain compiles under `just check-pd` without a board build. It must not read files that `lerux-cli` writes during `build()`. `tools/lerux-cli/src/serial_sddf.rs` writes the same bytes so a host test can compare them with the rendered system description.
- Template virtual addresses use the same digit grouping as the Rust constants (`0x2_000_000`). `0x2_000_000` and `0x2000000` are the same number and different spellings.
- Copy a large config once during init. `serial_virt_tx_config_t` is several kilobytes, and the stack is `0x10_000`. Keep queue handles and channel ids in the handler.

### Channels, queues, and the serial device

- The device interrupt is not a field of `serial_driver_config_t`. The template `<irq id>` uses `DRIVER_IRQ_CHANNEL`.
- Implement `Handler::notified` on every protection domain that can be notified. Implement `Handler::protected` before setting `pp="true"`. Both defaults panic.
- Microkit zeroes shared pages, so `producer_signalled` starts at 0 and the consumer signals the producer. The client still implements `notified` when that signal only returns transmit space. After an enqueue, the producer notifies the consumer. The consumer notifies the producer only when the queue asks for that signal, then cancels it.
- Queue indexes use an acquire load for the index the other side writes and a release store for the index this side owns.
- The QEMU PL011 path reuses `sel4-pl011-driver`. Leave the baud alone. `Write::write` spins while the transmit FIFO is full, so the transmit drain does not arm a transmit interrupt. The C driver in `deps/workspace/lionsos/dep/sddf/drivers/serial/arm/uart.c` ORs `PL011_LCR_PARTY_EN` where a disable was intended. That OR enables parity. The Rust driver does not copy it.
- Crate and program names use hyphens (`sddf-serial-driver.elf`). Microkit protection-domain names use underscores (`serial_driver`). A search for `serial-driver.elf` also matches `sddf-serial-driver.elf`.
- `just test-serial-sddf` expects `lerux shell ready` from `serial_enqueue`. The client does not link `lerux-logging`. Keep that substring out of `expect` and `assert` messages.

## Cothreads (`lerux-cothread`)

Read Milestone 3 in [docs/plan-lionsos.md](docs/plan-lionsos.md). New blocking input and output uses `lerux-cothread`. Leave `lerux-service-async` in place for postcard servers.

- The root cothread is the Microkit kernel thread. Only the root calls `microkit_cothread_recv_ntfn`, and it calls that from `Handler::notified`. A worker blocks in `microkit_cothread_wait_on_channel` or `semaphore_wait`. `semaphore_signal` and `recv_ntfn` switch to the waiter before they return.
- Pass explicit stacks. The minimum is `MIN_STACK_SIZE` (`0x1000`). The smoke stacks are `0x4000`, 16-byte aligned, and live in static storage. There is no guard page.
- A contract break panics: a second init, a bad handle, a wait on the root, `recv_ntfn` from a worker, or destroying a blocked cothread. Spawn of a full pool returns `NULL_HANDLE`.
- Do not panic inside a cothread. Unwind across the context switch is unsafe. Put assertions on the root.
- Host tests take a process lock before they use the runtime. The runtime itself does not lock. A lock held across the switch deadlocks.
- The x86 context switch is Intel syntax. This rustc rejects `.intel_syntax` under `-D warnings`. RISC-V is not implemented.
- `just test-cothread` expects `cothread waiting` and then `cothread resumed`. Keep `lerux shell ready` out of that protection domain.

## Host tooling (`tools/lerux-cli/**`)

- Use `anyhow::Result` at the CLI boundary; add context with `.context("…")?`.
- Use `clap` derive for subcommands.
- No `unwrap()` or `expect()` in production paths — use `?` or `bail!`.
- `std` only; do not depend on seL4 userspace crates. `lerux-sddf` is allowed: it has no seL4 dependency and is the shared layout for the config bytes the command-line interface checks. Do not depend on a protection-domain crate.

## Quality gates

Run before finishing Rust changes:

```bash
just check
```

This runs `cargo fmt --all --check`, clippy and tests for every crate named in the justfile `check` recipe (including `lerux-sddf`), and `lerux profile check-qos`. After protection-domain or shared userspace crate changes, also run:

```bash
just check-pd
```

`check-pd` runs cross-target clippy on all PD and shared userspace crates (one pass per arch). It requires a built SDK (`just build-sdk`) for `SEL4_INCLUDE_DIRS`. `just check-all` runs both.

PD changes may need a full board build (`just build` or `just build-pd <crate>`) because targets are seL4 cross-compile profiles.

Workspace `[lints]` in the root `Cargo.toml` sets clippy defaults; each crate inherits them via `[lints] workspace = true`.

CI runs `just check` before the SDK pipeline and `just check-pd` after the SDK artifact is ready (in parallel with smoke).

A few results look like success and are not:

- `lerux-cli` is a binary crate. Test it with `cargo test -p lerux-cli -- <filter>`. There is no `--lib` target. `--exact` needs the full path `module::tests::<name>`. A filter matches test function names, not the integration-test file name. Run a file with `cargo test -p <crate> --test <file stem>`. A filter that matches nothing still exits 0; `running 0 tests` is the result.
- Do not run `just check`, `just check-pd`, and `just test-*` together. `just check` uses `build/host`. `just check-pd` and a smoke share `build/target`, and both invoke `cargo run -p lerux-cli` in the default target directory.
- Update the milestone status, the board row, and the smoke counts in `docs/ci.md` and the README only after `just check`, `just check-pd`, and that milestone's smokes have passed. The workflow `include` list and those hand-written counts move together. Leave the historical job counts inside `docs/plan.md` as they were.
- Lint protection domains with `just check-pd`. That command sets up libclang. `cargo clippy` on a PD target without it panics inside bindgen.
- `ci = true` in `support/boards.toml` is the set `just test-all` runs. GitHub smoke is the `include` list in `.github/workflows/rust.yml`. The job count in `docs/ci.md` and the README is written by hand. A new board name is passed as `--features board-<name>` only when that key exists in the PD `Cargo.toml`.
- Take a QEMU `-device` property from that binary's `-device <name>,help`. The host kernel's interrupt mode is a different machine. `just test-x86-usb-kbd` covers qemu-xhci and QEMU's `usb-kbd`. The desk keyboard and the Intel xHCI path are the metal boot in `docs/boards.md`.
- The exit status of a shell string is its last command. A redirected recipe that ends in `echo` reports that echo. Read the log for the recipe's own status line.

## What not to do

- Do not introduce `std` into PD crates.
- Do not modify seL4 or Microkit sources under `deps/workspace/`.
- Do not add vendored rust-sel4 trees — use workspace dependencies.
- Do not "clean up" unrelated code, normalize experiments, or rewrite git history unless asked.

## Cursor Cloud specific instructions

The VM snapshot already has the full dev environment: system packages (cmake, ninja, dtc, xmllint, `qemu-system-{arm,x86,misc}`, libclang, glib/pixman for SP804 QEMU), the pinned Rust nightly, `just`, the prebuilt Microkit SDK, and the cross toolchains. Do not reinstall these; the startup update script only runs `cargo fetch --locked`.

- **SDK**: obtained via `just fetch-sdk` (prebuilt SDK 2.2.0 at `deps/microkit-sdk/`, path in `deps/.sdk-path`), not `just build-sdk`. It covers the `qemu_virt_aarch64`, `qemu_virt_riscv64`, `x86_64_generic`, and `rpi4b_4gb` boards used by the smoke matrix. All of `deps/workspace/`, `deps/microkit-sdk/`, `deps/toolchains/`, `deps/.sdk-path` are gitignored but persist in the snapshot. If the SDK is ever missing, re-run `just fetch && just fetch-sdk`.
- **Cross toolchains auto-install**: `just build`/`just test` auto-download the ARM (`aarch64-none-elf`) and RISC-V toolchains into `deps/toolchains/` on the first PD build if absent — no manual ARM toolchain step needed.
- **Commands** are documented in the `README.md`/`justfile`: `just check` (host lint, no SDK), `just check-pd` (cross clippy, needs SDK), `just test` (default aarch64 hello smoke in QEMU). Other boards via `BOARD=... just test` or the `just test-*` recipes.
- **Caveat**: `init`/`composed`/`*-composed` boards need patched SP804 QEMU — run `cargo run -p lerux-cli -- install sp804-qemu` first (build deps are already installed). Not required for the default hello/echo/virtio/http flows.

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