# Lerux in Rust, on the LionsOS architecture

Last updated: 2026-10-04

**Status:** Milestone 1 is done and is the shared-structure crate. Milestone 2 is next. Milestones run in order. Each remaining milestone gets its own implementation plan after the previous smoke is green. Do not implement more than one milestone in a pass.

**Goal:** Every lerux guest protection domain stays Rust, and the queues, configuration pages, filesystem messages, and component graph are the same ones LionsOS uses.

**Architecture:** Treat the C headers in LionsOS 0.4.0 and the seL4 Device Driver Framework it pins as a specification, not as code to link. Reproduce those structs with `#[repr(C)]` and check their size and offsets against the headers. Split devices into the same single-purpose protection domains LionsOS composes: driver, transmit virtualiser, receive virtualiser, per-client network copier, block virtualiser, and a filesystem server speaking the LionsOS command queue. Applications call a Rust library with the same file-descriptor and socket model as the LionsOS file and socket library. The Internet Protocol stack behind that socket model is smoltcp, one stack per client. Blocking calls use a Rust cothread runtime with the same operations as `libmicrokitco`.

**Tech stack:** Existing Rust protection domains, `#![no_std]`, rust-sel4, Microkit. New guest crates are Rust. The C headers are compiled only by a host layout test. Python sdfgen, musl, MicroPython, lwIP, libvmm, and the WebAssembly Micro Runtime are not linked.

**Decision:** [011-lionsos-structures.md](decisions/011-lionsos-structures.md). Decision records 001 through 004 and 008 still describe the postcard image. They do not govern new device or filesystem code.

## Global constraints

- Specification checkout: LionsOS tag `0.4.0`, commit `ad4c35ea7ae657d2f450e45c821035e746cfa48a`. Its `dep/sddf` submodule is commit `650ace5dba984f5d9c7dd1a005648ff7b40655f7`, which `git describe --tags` names `0.7.0`. The local au-ts mirror is behind that tag. Do not copy struct layouts from the mirror.
- `lerux fetch` clones that LionsOS tag and initialises only `dep/sddf`. It does not initialise the other submodules. Those are C components and language runtimes, and this checkout does not build them. Widen that checkout only when a later milestone needs a header that is not already there.
- Guest code stays `#![no_std]` Rust. It includes no C. It does not call musl, lwIP, MicroPython, or the WebAssembly Micro Runtime.
- Shared structs use `#[repr(C)]`. A host test fails when `size_of` or `offset_of` disagrees with the header. `fs_cmd_t` and `fs_msg_t` are 64 bytes, which the header asserts.
- Queue indexes use the same acquire and release pairing as the header comments. The producer is the only writer of the tail. The consumer is the only writer of the head. Use that pairing on every new queue, which is the symmetric-multiprocessing branch of the header, including on a single processor.
- A new guest component discovers its queues from a configuration memory region whose magic and fields match the header (`serial_driver_config_t`, `serial_virt_tx_config_t`, `serial_virt_rx_config_t`, `serial_client_config_t`, and the network, block, and filesystem equivalents). A handwritten `Channel::new` is not the protocol. The legacy image still uses channel constants until Milestone 17.
- Postcard request types are not added. An old protection domain stays on postcard only until the milestone that replaces it deletes it.
- Do not modify `deps/workspace/` by hand. Do not vendor the C trees into git.
- The legacy image keeps the Microkit software development kit 2.2.0 and rust-sel4 `v4.0.0` until the last postcard protection domain is gone. Queue layouts do not require the Microkit 2.3.0 bump that LionsOS 0.4.0 uses. Bump the kit when a rust-sel4 release supports Microkit 2.3.0, not before.
- A missing header in the 0.4.0 checkout stops that milestone. The struct is not invented to keep moving.
- Milestones 2–15 are QEMU AArch64. Full terms in new docs and commit messages.

## What "the same" means

| LionsOS | Rust in lerux |
| --- | --- |
| `serial_queue_t` (`tail`, `head`, `producer_signalled`) and `serial_queue_handle_t` | Same fields, same enqueue and dequeue rules, in `lerux-sddf` |
| `net_buff_desc_t`, `net_queue_t`, `net_queue_handle_t` | Same fields. `oid` stays a 6-bit field. Clients store `0` there. |
| `blk_req_t`, `blk_resp_t`, `blk_req_queue_t`, `blk_resp_queue_t` | Same fields |
| `fs_cmd_t`, `fs_cmpl_t`, `fs_queue_t`, capacity 511, status codes `FS_STATUS_*`, commands `FS_CMD_*` | Same fields and numeric values |
| `serial_*_config_t` with magic `sDDF\x03` | The system generator writes these bytes into a mapped configuration region |
| Driver, `serial_virt_tx`, `serial_virt_rx`, clients | Four roles. One protection domain does not both drive the serial device and multiplex clients. |
| Network driver, `net_virt_tx`, `net_virt_rx`, one copier per client, one Internet Protocol stack per client | Same roles. The stack is smoltcp. lwIP is not ported. |
| Block driver, `blk_virt`, filesystem server, client | Same roles. The server speaks `fs_queue_t`. The on-disk format is File Allocation Table, implemented in Rust. |
| `libmicrokitco`: `init`, `spawn`, `yield`, `wait_on_channel`, semaphore wait and signal | `lerux-cothread`, stackful, same operations. `lerux-service-async` is not used for new blocking input and output. |
| File and socket calls mapped onto the filesystem queue and the client stack | `lerux-posix`: file-descriptor table, `open`/`read`/`write`/`socket`, filesystem status codes mapped to the same `errno` values as `lib/libc/posix` at the pinned tag |
| sdfgen metaprogram and `serialise_config` | The Rust system generator emits the same protection-domain set and writes the same configuration structs. Python sdfgen is not added. |
| WebAssembly runtime protection domain with filesystem and network access | A Rust WebAssembly runtime in that role. The `LRW1` opcode allowlist is removed. A signature check before instantiate may stay. |
| Kitty Linux guest framebuffer | Not ported. That path is a Linux guest, and the LionsOS header calls it unprincipled. Display uses the driver-framework virtio graphics class, in Rust, once its headers are in the specification checkout. |

LERUXFS2, the combined `net-server`, the postcard `serial-virt`, and `config-server` as its own protocol are the divergences this plan removes.

## Rules for later milestones

1. Copy the struct from the pinned header. Do not rename fields. Do not drop a field because the current lerux code does not use it.
2. One protection domain, one role from the table above.
3. An untrusted client maps its own queue and data region. It does not map device registers or another client's data region.
4. The milestone that makes the new smoke pass also deletes the postcard protection domain it replaced, when that milestone's exit says so. Milestone 2 keeps the postcard serial path because the shell has not moved.
5. A feature with no counterpart becomes a Rust program on `lerux-posix` and the queues. It does not grow a new message crate.

## Milestone 1 — Shared structs that match the headers

Done. No QEMU behavior change. The legacy `just test` smoke still passes.

Landed:

- `userspace/crates/lerux-sddf` holds `serial_queue_t`, `serial_queue_handle_t`, the four `serial_*_config_t` structs, `net_buff_desc_t`, `net_queue_t`, `net_queue_handle_t`, `blk_req_t`, `blk_resp_t`, the block queues, `fs_cmd_t`, `fs_cmpl_t`, `fs_queue_t`, and the `FS_CMD_*` and `FS_STATUS_*` constants. `FS_QUEUE_CAPACITY` is 511. `SDDF_SERIAL_MAGIC` is the five bytes `s`, `D`, `D`, `F`, `0x03`.
- `net_queue_t` and `blk_req_queue_t` use `#[repr(C, align(8))]`. The C types end in a flexible array whose element alignment is 8. The array itself is not part of the Rust prefix.
- `oid` in `net_buff_desc_t` is the low 6 bits of its byte. The layout test compares a byte image from the host C compiler, because `offsetof` on a bit-field is not the check.
- Serial operations: `serial_enqueue`, `serial_dequeue`, `serial_enqueue_local`, `serial_update_shared_tail`, `serial_update_shared_head`, and the consumer-signal helpers. Filesystem operations are the queue half of `fs_command_issue`: enqueue one `fs_msg_t` and dequeue one message. Notification and request-id allocation stay with the caller. Serial colour and batch transfer are not ported.
- `tests/layout.rs` compiles the fetched headers with the host `cc` and compares `sizeof`, `_Alignof`, and `offsetof`. When the headers are absent the test prints that `lerux fetch` is required and passes, so `just check` can run before fetch. Continuous integration runs `just check` before `just fetch`, so that job does not compare layouts.
- Checked on 2026-10-04: `cargo test -p lerux-cli -- parses_lionsos` passed, `lerux fetch` checked out the tag and both header paths, `cargo test -p lerux-sddf` passed, `just check` passed, `just check-pd` passed, and `just test` printed `lerux: Hello from Rust on seL4 Microkit!`.

## Milestone 2 — Serial roles

**Exit:** `just test-serial-sddf` boots a QEMU AArch64 image whose protection domains are only `serial_driver`, `serial_virt_tx`, `serial_virt_rx`, and one client. The client writes `lerux shell ready` by `serial_enqueue` into the queue described by `serial_client_config_t`. The driver is the only protection domain that maps the serial device. The configuration regions start with the serial magic. The legacy echo smoke still passes.

**Files:** `userspace/pds/sddf-serial-driver`, `userspace/pds/sddf-serial-virt-tx`, `userspace/pds/sddf-serial-virt-rx`, `userspace/pds/sddf-serial-client`, a system template, and a generator function that fills `serial_driver_config_t`, `serial_virt_tx_config_t`, and `serial_virt_rx_config_t`.

The existing `serial-driver` and `serial-virt` postcard path stay until a later workstation milestone switches the shell and deletes them.

## Milestone 3 — Cothreads

**Exit:** `cargo test -p lerux-cothread` spawns two cothreads, blocks one in `wait_on_channel`, runs the other, signals the channel, and observes the blocked thread resume. A protection-domain smoke on QEMU does the same wait against a Microkit notification. The operations match `microkit_cothread_init`, `spawn`, `yield`, `wait_on_channel`, `semaphore_wait`, and `semaphore_signal`. Stacks are explicit. This crate replaces `lerux-service-async` for new blocking input and output.

## Milestone 4 — Block and the filesystem queue

**Exit:** `just test-fs-sddf` has `blk_driver`, `blk_virt`, `fatfs`, and one client. The client blocks in a cothread on `FS_CMD_FILE_OPEN`, `FS_CMD_FILE_WRITE`, and `FS_CMD_FILE_READ`. The bytes read back match the bytes written. Messages are `fs_cmd_t` and `fs_cmpl_t`. The on-disk format is File Allocation Table. LERUXFS2 is not mounted by this image.

## Milestone 5 — File-descriptor library

**Exit:** `lerux-posix` on the host, with a fake filesystem queue, implements `open`, `read`, `write`, `mkdir`, `unlink`, `rename`, and `stat`. Status codes map to the `errno` values in the pinned `lib/libc/posix`. `MAX_FDS` defaults to 128. Descriptors 0, 1, and 2 are reserved. `cargo test -p lerux-posix` covers a successful read and `FS_STATUS_NO_FILE`.

## Milestone 6 — Shell file commands

**Exit:** The scripted serial session on the Milestone 4 image runs `mkdir`, `write`, `cat`, `ls`, `mv`, `rm`, and `stat` through `lerux-posix`. Golden strings live in `support/smoke-expects.toml`. The shell protection domain does not depend on `lerux-interface-types`.

Shell commands this plan keeps: `ls`, `cat`, `write`, `mkdir`, `rm`, `mv`, `cd`, `pwd`, `stat`, `df`, `echo`, `help`, `history`, `clear`, `date`, `uptime`, `ip`, `ping`, `fetch`, `config`, `hostname`, `cert`, `backup`, `calc`, `source`, `run`, `reboot`, `dmesg`, `ps`, `status`, `qos`.

## Milestone 7 — Network roles

**Exit:** The image adds `net_driver`, `net_virt_tx`, `net_virt_rx`, one copier, and a client that owns smoltcp. The client does not map the device queue. After Dynamic Host Configuration Protocol on QEMU user-net, `ip` prints an address and `ping` of the gateway gets a reply. There is no `net-server` in this image.

## Milestone 8 — Sockets and fetch

**Exit:** `lerux-posix` grows `socket`, `connect`, `send`, and `recv` on that client's smoltcp. `fetch` of the existing host `lerux https-one` fixture writes a file, and `cat` prints it. Transport Layer Security is a Rust library inside the client, or one protection domain in front of it, using those socket calls. There is no `TlsRequest` or `HttpRequest` type.

## Milestone 9 — Config, time, reboot, static status

**Exit:** `config get hostname` reads `/config/hostname` with `open`. `date` reads the driver-framework timer client. `reboot` resets QEMU and the next log contains `lerux shell ready`. `ps` and `qos` print the protection domains, priorities, and budgets written into the generated configuration. `dmesg` prints serial bytes the shell stored. No `config-server`, `supervisor`, or `log-server` is in the image. The supervisor-only `secret.*` rule is not reimplemented.

## Milestone 10 — Remaining shell programs

**Exit:** Scripted smokes for `calc`, `backup`, `source`, `cert`, `edit`, and `chat`. `edit` and `backup` use `lerux-posix`. `chat` uses the socket calls. Each smoke has one expected serial string.

## Milestone 11 — File browser

**Exit:** A Rust program on the socket and file libraries serves one seeded file over Hypertext Transfer Protocol. `just test-web-sddf` expects that body in the host response. `http-file-browser` and `http-server` are not ported as postcard clients.

## Milestone 12 — Display and browser

**Exit:** A Rust virtio graphics driver and virtualiser use the queue and configuration structs from the pinned driver-framework graphics headers. The browser program writes pixels through that client interface and fetches the page through the socket library. `just test-browser-sddf` expects a known pixel pattern and the page title on the serial log. `DisplayRequest` is not ported.

## Milestone 13 — Agent

**Exit:** `grok` notifies the agent protection domain. The agent uses `lerux-posix` for Read, Edit, Write, ListDir, and Search, the socket library for WebFetch, and one shared memory region plus a notification for Browse. The session expects `agent: tool write ok` and the written file bytes. Tool arguments are a `#[repr(C)]` struct in the agent header, not postcard.

## Milestone 14 — WebAssembly role

**Exit:** One Rust runtime protection domain has filesystem and socket access through `lerux-posix`, which is the role of the LionsOS WebAssembly component. `just test-program-sddf` runs a signed ordinary WebAssembly module and expects its print. A bad signature does not instantiate the module. The `LRW1` opcode list and `lerux-prog` interpreter are deleted in this milestone.

## Milestone 15 — Packages

**Exit:** `lerux package install edit` adds the edit program to the Rust system description for the workstation. The generator fails when the component name is unknown. Installing still rebuilds the image.

## Milestone 16 — Other boards

**Exit:** The same protection-domain roles boot on QEMU RISC-V and QEMU x86, using the devices the pinned driver framework already classifies (virtio, `ns16550`). `pc_z97_d3h` is unchanged in this milestone. A native Advanced Host Controller Interface, Video Graphics Array, or desk Ethernet driver is a later driver-framework class driver in Rust, with a queue struct taken from the matching header, and it is not part of this milestone.

## Milestone 17 — Remove the old model

**Exit:** Guest code has no `lerux_interface_types` dependency, no `LERUXFS`, and no postcard `SerialClient` or `NetRequest`. `docs/ci.md` lists the new recipes. `just test` is the Rust serial-and-filesystem workstation smoke. `just check` still lints the host tool.
