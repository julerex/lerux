# QIP investigation

Last updated: 2026-10-21

**Status:** Investigation plan. No incorporation decision. This document does not vendor a tree, change a protection domain, or amend an architecture decision record.

**Source:** [patrickgwsmith/qip](https://github.com/patrickgwsmith/qip) at commit `b65cbad5aedf749581f606215f95378810251390` (2026-10-15, "Vectorize WGSL invocations and add Perlin and Atkinson demos"). The site is <https://qip.dev>. The license is Apache-2.0. The authoring language is Zig. The published artifact is a WebAssembly module. Confirm this commit is still `main` at the start of the investigation, and record a newer commit here if it is not.

**Goal:** Decide whether any QIP contract or component belongs in lerux, and which host would run it. The next pass writes that recommendation into this file. It does not land a host.

## What QIP is

QIP is a set of small WebAssembly contracts for a finite transform. A normal component receives bytes, uniforms, and, on the interactive contracts, a time and events that the host supplies. It returns bytes. It does not receive a filesystem, a network, a clock, an environment, or a secret. The project states this boundary as a deliberate rejection of the WebAssembly System Interface, which it treats as too large and too tied to ambient host state for a deterministic transform.

The contract index is `docs/component-contract.md` in that tree. Maturity, in the project's own words:

| Contract | Work | Maturity |
| --- | --- | --- |
| Content | One input, one `render`, one output | Mostly stable |
| Time and Events | Retained state, animation, keyboard, pointer | Implemented; evolving |
| Graphical | Content whose output is a KTX2 frame, plus optional Time and Events | Implemented; evolving |
| Text interface | UTF-8 or ANSI frames | Implemented; evolving |
| Compliance | Executable cases against a Content component | Evolving |
| Tile | In-place 64 by 64 RGBA32Float filters (`IMAGE.md`) | Separate from Content |

Content is the contract an investigation can quote. The interactive contracts can change during the project's alpha, and the repository says components may need a rebuild when they do.

A host calls a Content component. The module does not call the host:

```text
host reads input_ptr and the input capacity
host writes the input bytes into exported memory
host sets optional uniform_set_<key> values
host calls render(input_size) -> i64
host reads the returned range, or stops on rejection
```

Exports that carry a pointer or a size are zero-argument functions returning `i32`. An exported global of the same name does not satisfy the contract. A normal Content module has no imports. `render` packs its result in an `i64`:

```text
bit 63 clear: bits 62..32 are the output pointer, bits 31..0 are the byte count
bit 63 set:   rejection; the low 32 bits are optional failure detail; the host reads no output
```

The pointer on success must be below `0x80000000`, the range must lie in memory, and the size must not exceed the declared output capacity. A trap discards the instance. A recoverable rejection may reuse it.

The command-line host is `qipx` (`npm` package `@qip.dev/qipx`). `qip run` executes Content only. One published vector is `rgb(101, 79, 240)` through `text/rgb-to-hex.wasm`, which prints `#654ff0`. Pipelines are ordered modules. The host, not the module, matches media types between stages.

Hard limits, from `docs/hard-limits.md`: the command-line host rejects `memory.grow` unless the caller opts in and sets a memory cap. A declared memory with no maximum fails that cap. The strict profile (no imports, one memory, a declared maximum, no `memory.grow`, no shared memory, no atomics, no start function, no indirect calls, an acyclic direct-call graph) is an optional checker component, `application/wasm/wasm-strict-profile.wasm`. It is not the default `qip run` policy. Single-instruction multiple-data opcodes are allowed in that strict profile. The checker components are themselves WebAssembly modules.

## What lerux already decided

These decisions stay in force while this investigation runs.

- Userspace is Rust. A protection domain does not link Zig, C, the WebAssembly Micro Runtime, or `wasmtime`. [ADR-012](decisions/012-ordinary-webassembly.md).
- A program that arrives after boot is an `LRW1` blob: magic, version 1, a WebAssembly payload of at most 4096 bytes, and an ed25519 signature. Unsigned bytes are not instantiated. [ADR-010](decisions/010-program-runtime.md).
- The engine is `wasmi` 2 in `lerux-wasm`. The features are `validate`, `prefer-btree-collections`, and `portable-dispatch`. `std`, `wat`, `memory64`, and `simd` stay off. Both runtime heaps are 2 MiB.
- `run` instantiates a module that imports `lerux.log`, and may import `lerux.read` and `lerux.socket`, then calls the export `start`. The postcard `program-runtime` protection domain logs `lerux-prog: ran`. The `qemu_virt_aarch64_program_sddf` protection domain logs `lerux-wasm: ran` after a rejection line. One engine, two protection domains, one calling convention.
- `lerux prog pack` compiles a Rust source with `rustc` and signs the result. It does not wrap a WebAssembly file that already exists. `signing_message` and `encode` in `lerux-wasm` already build an `LRW1` blob from raw bytes.
- The interactive engines are `lerux-html` and `lerux-web`. [ADR-009](decisions/009-interactive-surface.md) chose from-scratch subsets. The desk wayfinder keeps those engines until a ticket on [`.scratch/desk-browser/map.md`](../.scratch/desk-browser/map.md) says otherwise. This investigation is not that ticket.
- The browser image paints a 16 by 16 pattern in `GPU_FORMAT_B8G8R8A8_UNORM`. The desk map has not chosen a framebuffer size or pixel format. QIP's KTX2 profiles include both `R8G8B8A8_SRGB` and `B8G8R8A8_SRGB`, plus 32-bit float profiles. Those are pipeline annotations, not a general media type.

## Where the two calling conventions meet

```text
lerux program                         QIP Content component
  module imports lerux.log              module imports nothing
  runtime calls start()                 host calls render(input_size)
  module asks the host for a file       host writes the input bytes first
  payload at most 4096 bytes            catalog modules range from 135 bytes to megabytes
  signature checked before instantiate  qip.dev is an ordinary HTTPS GET
```

The engine can host both conventions. One `run` cannot. The linker defines `lerux.log`, `lerux.read`, and `lerux.socket` and does not require the module to import them, so a Content module with an empty import section can still instantiate. `run` then looks up the export `start` and returns `Error::Export` when it is missing. `instantiate_and_start` also runs a start section when the module has one. The strict profile forbids that section; a catalog module may still contain one, which step 3 of the investigation records.

Blob sizes at the pinned commit, from the Git tree, before any instantiation:

| Module | Bytes | Against the 4096-byte envelope |
| --- | --- | --- |
| `bytes/identity.wasm` | 135 | inside |
| `text/trim.wasm` | 322 | inside |
| `text/rgb-to-hex.wasm` | 972 | inside |
| `image/png/png-to-bmp-b8g8r8a8-srgb.wasm` | 8905 | outside |
| `image/png/png-to-bmp-b8g8r8a8-srgb-simd.wasm` | 12465 | outside |
| `application/wasm/wasm-strict-profile.wasm` | 10382 | outside |
| `text/html/html-link-extractor.wasm` | 11143 | outside |
| `text/html/html-page-wrap.wasm` | 17508 | outside |
| `text/markdown/commonmark.0.31.2.wasm` | 92647 | outside |
| `text/markdown/gfm-commonmark.0.31.2.wasm` | 97828 | outside |
| `image/svg+xml/svg-rasterize-to-ktx2-r8g8b8a8-srgb.wasm` | 289268 | outside |
| `image/svg+xml/svg-rasterize-skia-with-fonts-to-ktx2-r8g8b8a8-srgb.wasm` | 3439891 | outside |
| `tui/qipdb.wasm` | 128680 | outside |

These are file sizes. Declared linear memory is additional and is not known until the next pass decodes the memory section. A file that fits the envelope can still declare a memory the 2 MiB heap cannot hold. A filename without `simd` is not evidence that the body lacks `simd` opcodes.

## Options

The investigation chooses one. It records the others as declined in the recommendation.

### 1. Stop

Write the measurements and stop. Choose this when every candidate module needs `simd`, `memory.grow`, or a linear memory the heap cannot hold, or when the Content contract is the only fit and no lerux caller needs a pure transform.

### 2. Host fixture tool

Run `qipx` on the developer machine to produce an HTML or pixel fixture an existing smoke already compares. The guest gains no dependency and no module. Choose this when the useful output is a golden file. This does not exercise `wasmi`.

### 3. Content host on the existing engine

Add a second entry point beside `run`. The host checks the signature, instantiates with the current `wasmi` features, writes the input, calls `render`, and reads the packed `i64`. `program-runtime` keeps `start` and the `lerux.*` imports.

The first module is one of `bytes/identity.wasm`, `text/trim.wasm`, or `text/rgb-to-hex.wasm`, packed as `LRW1` through `signing_message` and `encode`, verified with the smoke key. The vector for the third is `rgb(101, 79, 240)` to `#654ff0`. The probe runs as a host test of `lerux-wasm`. A guest smoke waits until that test instantiates the module under the features ADR-012 already enables.

This option does not raise the 4096-byte cap, enable `simd`, or change `run`. A larger catalog module is a later decision and needs its own note in ADR-010.

### 4. Strict profile as a Rust gate

Reimplement the strict-profile rules in Rust and run them on the bytes after the signature check and before instantiate: no imports, one memory, a declared maximum, no `memory.grow`, no start function, no indirect calls, an acyclic direct-call graph. The QIP checker module stays out. A checker that is itself a 10 KiB WebAssembly module needs a host, and using it to admit modules is circular until option 3 exists.

Choose this when the goal is a smaller artifact policy for guest programs, including programs lerux compiles itself. The QIP checker is the specification. It is not the implementation. Single-instruction multiple-data opcodes are legal in the QIP strict profile and illegal under the current `wasmi` features, so a Rust gate has to say which of those two policies it is copying.

### 5. A pipeline in front of `web-content`

`request-server` fetches bytes. A runtime runs a pinned Content module. `lerux-html` and `lerux-web` either consume that output or stay on the path. This touches the desk wayfinder and ADR-009. It starts only after option 3 has measured a module that does useful document work, and only after a ticket on the desk map says the engines may change. CommonMark at this commit is 92 KiB before linear memory. The 4096-byte envelope cannot hold it.

### 6. Graphical frames into the bitmap

A Time and Events module exports `begin_update_at(now_ms)`, `finish_update`, and optional `key_event` and `pointer_event`, and `render` returns a KTX2 frame. The host would copy pixels into the shared bitmap or the virtio graphics client region.

The host supplies `now_ms`. The module does not read a clock. The filesystem image has a timer protection domain. The browser image does not. The desk map has not specified how keystrokes reach `browser-ui`, and it has not specified the pixel format. QIP's 8-bit profiles and `GPU_FORMAT_B8G8R8A8_UNORM` can be the same byte order only for `B8G8R8A8`. This option waits on options 3 and 5, on a framebuffer decision, and on the contract leaving alpha.

## What this investigation will not propose

- Vendoring or submodule-pinning the tree. The GitHub size field is about 171 MiB, the artifacts are committed next to their sources, and the contracts are still in alpha.
- A Zig toolchain, the Go image host, the Node command-line host, or a browser custom element inside a protection domain.
- Turning `simd` on, or switching the engine to `wasmtime`, so that a catalog module runs.
- Fetching a component from `qip.dev` during boot or during a smoke.
- Replacing the Microkit system description with the QIP site router.
- Amending the desk wayfinder, ADR-009, ADR-010, or ADR-012 inside the investigation pass. A recommendation that needs one of those changes says so and stops.

Apache-2.0 bytes that a later pass does copy into the repository keep that license notice. The lerux tree is MIT. Copying the Zig sources is not required for option 3.

## Investigation pass

One pass. Do these in order. The exit is an edit to the recommendation section below.

1. Confirm the pinned commit. If `main` has moved, record the new commit, re-measure the table above, and read the contract pages again before trusting this summary.
2. Read, at that commit, `docs/content-component.md`, `docs/hard-limits.md`, `docs/uniforms.md`, `docs/time-and-events.md`, `docs/gui-components.md`, the KTX2 section of `docs/formats.md`, and the pipeline rules in `docs/recipes.md`. Quote a rule in the recommendation only when the choice depends on it. Do not paste the pages into this repository.
3. For each row in the size table, plus one small `gui/*.wasm` that declares `image/ktx2`, record: import section, memory minimum and maximum, whether `memory.grow` appears, whether a `simd` opcode appears, whether `call_indirect` appears, whether a start function exists, and the export names and types. Decode the module. Do not instantiate it in this step.
4. Instantiate, on the host, the modules from that table that are inside the 4096-byte envelope, using `wasmi` with the features `lerux-wasm` already enables. Call the getters and `render`. Compare `text/rgb-to-hex.wasm` to `#654ff0`. Record the trap, the rejection bit, or the output bytes. This step may be a scratch binary. It becomes a `lerux-wasm` test only if the recommendation selects option 3.
5. Decode the memory section of `text/markdown/commonmark.0.31.2.wasm` and of the PNG module that has no `simd` in its name. Compare the declared memory, the file size, and `wasmi`'s own structures with the 2 MiB heap. State whether option 5 is possible without a heap change.
6. Write the recommendation: the chosen option, each declined option in one sentence, the measurements that forced the choice, and whether a later pass needs an architecture decision record before any production code. If the choice is option 3, name the module, the input, the expected output, and that the guest smoke is a follow-on. Stop.

## Recommendation

Pending. The investigation pass has not run.
