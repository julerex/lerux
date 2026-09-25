# ADR-010: Signed Wasm subset in a reserved protection domain

## Status

Accepted

## Date

2026-09-25

## Context

lerux images are static Microkit systems. The monitor maps each protection domain's ELF before boot and will not install another one. A domain has no frame capability, so it cannot turn bytes it just downloaded or compiled into a native executable.

The destination is a Rust program whose source lives on the guest filesystem, compiled by a lerux protection domain, then executed. That compiler is not this change. This change fixes the module format and the executor so a later compiler has a target.

Host `rustc` already lowers `no_std` Rust to `wasm32-unknown-unknown`. One interpreter can run those bytes now and, later, bytes emitted on the guest.

## Decision

1. A program that arrives after boot is an `LRW1` blob executed by `program-runtime`. The blob is magic `LRW1`, version `1`, a Wasm payload of at most 4096 bytes, and an ed25519 signature over the header and payload. The smoke verifying key is `support/keys/smoke.ed25519.pub`, compiled into the domain. Unsigned bytes are not instantiated.

2. The interpreter is a closed subset in `lerux-prog`, written for the module `rustc` emits from `support/prog/smoke.rs`. It is not `wasmi` or `wasmtime`. Any other section, opcode, or import fails the module before `start` runs.

   `lerux prog pack` compiles with `-C opt-level=z -C overflow-checks=off -C lto=yes -C link-arg=-zstack-size=4096`. The stack size keeps linear memory at one 64 KiB page. `core::hint::black_box` keeps the `1 + 1 == 2` test in the body; without it, `opt-level=z` deletes the branch.

   Sections the smoke module contains: type, import, function, memory, global, export, code, data. Custom sections are skipped.

   Opcodes: `block` (empty type), `end`, `br_if`, `call`, `local.get`, `local.set`, `local.tee`, `global.get`, `global.set`, `i32.load`, `i32.store`, `i32.const`, `i32.ne`, `i32.add`, `i32.sub`. `call` and `global.get` indices may be overlong LEB128 (up to 5 bytes), which is what this `rustc` emits.

   The only import is `lerux.log(ptr: i32, len: i32) -> ()`, reading the domain's copy of linear memory. The runtime calls the export `start() -> ()`.

3. `program-runtime` is an untrusted domain. It speaks `HttpRequest` to `request-server` and logs on serial. It does not hold `NetClient` or `TlsClient` and does not map DMA or MMIO. The smoke GETs `https://host:8443/smoke.lrw` from `lerux https-one`. One runtime domain is reserved in the image. A second program needs another pre-declared domain.

4. The on-guest compiler is in scope for the project and out of scope here. It will be a protection domain that reads a Rust subset (`fn`, `i32`, `+`, `==`, `if`, byte-string literals, `extern "lerux"` functions) and emits this Wasm subset. It will not be upstream `rustc`. Network bytes keep the ed25519 check. Locally compiled bytes are a separate trust path for that later change.

## Alternatives considered

### Native ELF in a pre-mapped `rwx` region

A slot domain could copy a position-independent blob and jump to it. The blob would not be a Microkit ELF (those expect the monitor to install the thread and IPC buffer), the domain cannot flush the instruction cache, and the program still could not mint a channel. Rejected.

### A root task that creates address spaces

That is how a downloaded native domain would get its own endpoints. It replaces the Microkit monitor, the system description generator, and the static trust map. Rejected. The 2026-06 pivot chose Microkit.

### General-purpose Wasm (`wasmi`, `wasmtime`) or a toy stack bytecode

A toy bytecode cannot be a Rust compilation target. A full engine is larger than the smoke module needs and is a second language runtime to audit. Rejected in favor of the subset `rustc` actually emits for `support/prog/smoke.rs`, grown only when a new Rust fixture needs an opcode.

## Consequences

- `just test-program` on `qemu_virt_aarch64_program` fetches the signed smoke module and logs `lerux-prog: ran`.
- `support/prog/smoke.lrw` is produced by `lerux prog pack` and served by `https-one`. Regenerating it is required when `smoke.rs` or the pinned nightly changes the Wasm bytes.
- `rust-toolchain.toml` includes the `wasm32-unknown-unknown` target so host tests can compile the smoke crate.
- Native Microkit ELFs remain host-built members of `loader.img`. `pc_z97_d3h` has no NIC, so this fetch stays on QEMU.
