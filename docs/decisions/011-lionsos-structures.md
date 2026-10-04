# ADR-011: LionsOS structures, Rust guest

## Status

Accepted

## Date

2026-10-04

## Context

lerux and LionsOS both build static Microkit systems on seL4. lerux's device and filesystem connections do not use LionsOS's structures. Applications speak postcard messages. One `net-server` owns the network stack. The on-disk format is LERUXFS2. Serial multiplexing on the workstation is a postcard virtualiser. Those choices are recorded in ADR-001, ADR-002, ADR-003, ADR-004, and ADR-008.

The guest stays Rust. LionsOS's C components, musl, MicroPython, lwIP, and Python system generator are not linked. The C headers in LionsOS 0.4.0 and the seL4 Device Driver Framework that release uses are the specification for the shared memory structures and the component roles.

## Decision

New device and filesystem connections use `lerux-sddf`. The structs are `#[repr(C)]` copies of the pinned headers, including:

- `serial_queue_t`, `serial_queue_handle_t`, and the `serial_*_config_t` pages, whose magic is the five bytes `sDDF` and `0x03`
- `net_buff_desc_t`, `net_queue_t`, and `net_queue_handle_t`, with `oid` kept as a 6-bit field
- `blk_req_t`, `blk_resp_t`, `blk_req_queue_t`, and `blk_resp_queue_t`
- `fs_cmd_t`, `fs_cmpl_t`, and `fs_queue_t`, with `fs_cmd_t` and `fs_msg_t` 64 bytes and `FS_QUEUE_CAPACITY` 511

A host test compiles those headers and compares `sizeof` and `offsetof` with the Rust types. Queue indexes use acquire loads for the index the other side writes and release stores for the index this side owns.

ADR-001, ADR-002, ADR-003, ADR-004, and ADR-008 no longer govern new device or filesystem code. They still describe the postcard image until that image is removed. The guest remains Rust. The specification checkout is LionsOS tag `0.4.0`, cloned by `lerux fetch`, and it is not compiled into a protection domain.

## Alternatives considered

### Link the LionsOS C components

Rejected. The guest stays Rust. The headers are a layout specification. The checkout under `deps/workspace/lionsos` is not a userspace dependency.

### Keep postcard and only copy the virtualiser diagram

Rejected. That is the split ADR-002 and ADR-003 already chose. The shared structures would still be lerux's.

## Consequences

- `userspace/crates/lerux-sddf` is the crate for these structures. New queue and filesystem code uses it.
- `lerux fetch` clones LionsOS `0.4.0` and initialises the `dep/sddf` submodule so the layout test can include the headers.
- Postcard protection domains stay until a later milestone replaces each one. This decision does not change the legacy image.
- Later milestones add the transmit and receive virtualisers, the per-client network copier, the filesystem command queue, and a Rust cothread runtime. Those are not this change.
