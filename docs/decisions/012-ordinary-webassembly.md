# ADR-012: Ordinary WebAssembly in the LionsOS runtime role

## Status

Accepted for `wasmi` in the LionsOS runtime and for WebAssembly a page runs. The postcard `program-runtime` protection domain is not in the tree. Milestone 17 stays in [`plan-lionsos.md`](../plan-lionsos.md) and is the postcard guest-model removal. The live runtime protection domain is `program` (`sddf-program.elf`) on `qemu_virt_aarch64_program_sddf`. The browser path is [`../../lerux-ladybird/PLAN.md`](../../lerux-ladybird/PLAN.md) milestone 18. The engine is a Rust library, not a port of Ladybird, Chromium, or WebKit C++.

## Date

2026-10-12

## Context

A LionsOS image has one WebAssembly runtime protection domain with filesystem and network access. Milestone 14 is that role. [ADR-010](010-program-runtime.md) fixed the `LRW1` envelope and put a closed opcode list in `lerux-prog`, and it rejected `wasmi` because a general engine was larger than the smoke module. The milestone exit deletes that list. The postcard `program-runtime` protection domain is not in the tree. Milestone 17 stays as the postcard guest-model removal.

## Decision

The runtime is `wasmi` 2.0.0 inside `lerux-wasm`. The crate is `no_std`. The enabled features are `validate`, `prefer-btree-collections`, and `portable-dispatch`. `prefer-btree-collections` avoids a random source. `portable-dispatch` avoids a tail call on the custom target. `std`, `wat`, `memory64`, and `simd` stay off.

`run_signed` checks the ed25519 signature and returns before the module is instantiated. A bad signature never starts the module. The payload is ordinary WebAssembly. There is no opcode allowlist.

`qemu_virt_aarch64_program_sddf` is the fetch image with `program` (`sddf-program.elf`) in place of the shell. That protection domain is the serial client, the filesystem client, and the network client. The module imports `lerux.read`, `lerux.socket`, and `lerux.log`. `lerux.read` reads `/config/hostname` through `lerux-posix`. `lerux.socket` allocates the one socket. The pipe does not connect, and the image does not start Dynamic Host Configuration Protocol. The network queues are still mapped, and `net_buffers_init` runs once on the transmit free queue.

There is one runtime protection domain on the LionsOS image, and it is `program`. The browser's WebAssembly calls go through the same `lerux-wasm` library. Milestone 17 stays.

## Alternatives considered

### Keep the opcode allowlist

Rejected. The milestone exit deletes it. A new hand-rolled list would be the same interpreter under another name.

### `wasmtime`

Rejected. It wants `std` and a much larger runtime than this protection domain.

### The WebAssembly Micro Runtime

Rejected. It is C. This plan does not link the WebAssembly Micro Runtime, musl, MicroPython, lwIP, libvmm, or sdfgen.

## Consequences

- `lerux-prog` and `program-runtime` are deleted. `lerux prog pack` and `sddf-program` use `lerux-wasm`.
- `just test-program-sddf` expects `program: rejected` and then `lerux-wasm: ran`. `just test-program` is gone.
- `wasmtime`, the WebAssembly Micro Runtime, and a second runtime protection domain stay out.
- The heap is 2 MiB. The worker stack is `0x1_0000`. The protection-domain stack attribute stays `0x10_000`.
